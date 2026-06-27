//! A deliberately tiny prior-knowledge HTTP/2 (h2c) client, just big enough
//! to make one unary gRPC call against the dish.
//!
//! The dish speaks plaintext HTTP/2 (no TLS, no upgrade dance) on
//! `192.168.100.1:9200`, so we open a TCP socket, send the connection
//! preface and an empty `SETTINGS` frame, then for each call write a
//! `HEADERS` + `DATA` pair on a fresh odd stream id and read frames back
//! until the stream ends.
//!
//! Scope on purpose:
//!
//! * **Outbound HPACK only.**  Request headers are encoded as HPACK
//!   *literal-without-indexing, new-name* fields — no static table, no
//!   dynamic table, no Huffman.  That is the simplest fully spec-compliant
//!   encoding and keeps us from needing an HPACK *decoder*: we never read the
//!   server's header blocks, we only pull the `DATA` frames off our stream.
//! * **No flow control.**  A `get_status` reply is ~1–2 KiB, far under the
//!   65535-byte default stream/connection window, and our request is a
//!   handful of bytes, so neither side's window is ever exhausted in one
//!   unary exchange.  We never send `WINDOW_UPDATE`.
//! * **One in-flight stream.**  No multiplexing; each [`H2Transport::call`]
//!   runs to completion before the next.
//!
//! This is `std`-only today (it uses [`std::net::TcpStream`]); the workspace
//! plan is to swap the socket for `rustix` later behind the same
//! [`starlink_core::Transport`] trait, which is why none of this leaks above
//! that interface.

use core::future::Future;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use starlink_core::Transport;

/// HTTP/2 connection preface every client must send first (RFC 9113 §3.4).
const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

// Frame types (RFC 9113 §6).
const FRAME_DATA: u8 = 0x0;
const FRAME_HEADERS: u8 = 0x1;
const FRAME_RST_STREAM: u8 = 0x3;
const FRAME_SETTINGS: u8 = 0x4;
const FRAME_PING: u8 = 0x6;
const FRAME_GOAWAY: u8 = 0x7;

// Frame flags (meaning varies by frame type).
const FLAG_ACK: u8 = 0x1; // SETTINGS, PING
const FLAG_END_STREAM: u8 = 0x1; // DATA, HEADERS
const FLAG_END_HEADERS: u8 = 0x4; // HEADERS

/// Errors raised while driving the HTTP/2 connection.
#[derive(Debug)]
pub(crate) enum H2Error {
    /// Underlying socket I/O failed.
    Io(io::Error),
    /// The peer reset our stream with the given error code.
    Reset(u32),
    /// The peer sent `GOAWAY` with the given error code.
    GoAway(u32),
}

impl From<io::Error> for H2Error {
    fn from(e: io::Error) -> Self {
        H2Error::Io(e)
    }
}

impl core::fmt::Display for H2Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            H2Error::Io(e) => write!(f, "http/2 I/O error: {e}"),
            H2Error::Reset(code) => write!(f, "stream reset by dish (error code {code})"),
            H2Error::GoAway(code) => write!(f, "connection closed by dish (GOAWAY code {code})"),
        }
    }
}

/// A single TCP/HTTP-2 connection to the dish.
pub(crate) struct H2Transport {
    stream: TcpStream,
    authority: String,
    /// Next stream id to use; HTTP/2 client streams are odd and increasing.
    next_stream_id: u32,
}

impl H2Transport {
    /// Connect to `addr` (`host:port`), send the preface, and advertise our
    /// (empty) `SETTINGS`.  `addr` doubles as the `:authority` header value.
    pub(crate) fn connect(addr: &str) -> Result<Self, H2Error> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_nodelay(true)?;
        let mut t = Self {
            stream,
            authority: addr.to_string(),
            next_stream_id: 1,
        };
        t.stream.write_all(PREFACE)?;
        // Empty SETTINGS frame on the control stream (id 0).
        write_frame(&mut t.stream, FRAME_SETTINGS, 0, 0, &[])?;
        t.stream.flush()?;
        Ok(t)
    }

    /// Perform one unary call: send `path` + `body` (already gRPC-framed) and
    /// return the concatenated response `DATA` payload (still gRPC-framed).
    fn call(&mut self, path: &str, body: &[u8]) -> Result<Vec<u8>, H2Error> {
        let stream_id = self.next_stream_id;
        self.next_stream_id += 2;

        // --- HEADERS ---
        let mut hb = Vec::new();
        hpack_literal(&mut hb, ":method", "POST");
        hpack_literal(&mut hb, ":scheme", "http");
        hpack_literal(&mut hb, ":path", path);
        hpack_literal(&mut hb, ":authority", &self.authority);
        hpack_literal(&mut hb, "content-type", "application/grpc");
        hpack_literal(&mut hb, "te", "trailers");
        write_frame(
            &mut self.stream,
            FRAME_HEADERS,
            FLAG_END_HEADERS,
            stream_id,
            &hb,
        )?;

        // --- DATA (closes our half of the stream) ---
        write_frame(
            &mut self.stream,
            FRAME_DATA,
            FLAG_END_STREAM,
            stream_id,
            body,
        )?;
        self.stream.flush()?;

        // --- read until the dish ends our stream ---
        let mut response = Vec::new();
        loop {
            let frame = read_frame(&mut self.stream)?;
            match frame.kind {
                FRAME_SETTINGS if frame.flags & FLAG_ACK == 0 => {
                    // Acknowledge the dish's settings.
                    write_frame(&mut self.stream, FRAME_SETTINGS, FLAG_ACK, 0, &[])?;
                    self.stream.flush()?;
                }
                FRAME_PING if frame.flags & FLAG_ACK == 0 => {
                    // Echo the 8-byte opaque payload back as a PING ACK.
                    write_frame(&mut self.stream, FRAME_PING, FLAG_ACK, 0, &frame.payload)?;
                    self.stream.flush()?;
                }
                FRAME_DATA if frame.stream_id == stream_id => {
                    response.extend_from_slice(&frame.payload);
                    if frame.flags & FLAG_END_STREAM != 0 {
                        break;
                    }
                }
                // Trailers (or trailers-only) that close our stream.  We never
                // decode response header blocks; a HEADERS frame on our stream
                // without END_STREAM is the initial response headers and is
                // ignored by the catch-all below.
                FRAME_HEADERS
                    if frame.stream_id == stream_id && frame.flags & FLAG_END_STREAM != 0 =>
                {
                    break;
                }
                FRAME_RST_STREAM if frame.stream_id == stream_id => {
                    return Err(H2Error::Reset(error_code(&frame.payload)));
                }
                FRAME_GOAWAY => {
                    // GOAWAY error code is the second u32 of the payload.
                    let code = frame
                        .payload
                        .get(4..8)
                        .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
                    return Err(H2Error::GoAway(code));
                }
                // SETTINGS ACK, WINDOW_UPDATE, frames for other streams, etc.
                _ => {}
            }
        }
        Ok(response)
    }
}

impl Transport for H2Transport {
    type Error = H2Error;

    fn unary(
        &mut self,
        path: &str,
        body: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        // The work is synchronous blocking I/O; wrap the result in an
        // already-ready future so we satisfy the async `Transport` interface
        // without pulling in an executor.  A real async socket would return a
        // genuinely pending future here instead.
        core::future::ready(self.call(path, body))
    }
}

/// First four bytes of an `RST_STREAM`/`GOAWAY` payload as a `u32` error code.
fn error_code(payload: &[u8]) -> u32 {
    payload
        .get(0..4)
        .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// A parsed HTTP/2 frame header plus its payload.
struct Frame {
    kind: u8,
    flags: u8,
    stream_id: u32,
    payload: Vec<u8>,
}

/// Write one frame: 9-byte header (24-bit length, type, flags, 31-bit stream
/// id) followed by `payload`.
fn write_frame(
    w: &mut impl Write,
    kind: u8,
    flags: u8,
    stream_id: u32,
    payload: &[u8],
) -> io::Result<()> {
    let len = payload.len();
    debug_assert!(len <= 0x00FF_FFFF, "frame payload exceeds 24-bit length");
    #[allow(clippy::cast_possible_truncation)]
    let header = [
        (len >> 16) as u8,
        (len >> 8) as u8,
        len as u8,
        kind,
        flags,
        (stream_id >> 24) as u8,
        (stream_id >> 16) as u8,
        (stream_id >> 8) as u8,
        stream_id as u8,
    ];
    w.write_all(&header)?;
    w.write_all(payload)
}

/// Read exactly one frame: its 9-byte header, then its declared payload.
fn read_frame(r: &mut impl Read) -> Result<Frame, H2Error> {
    let mut header = [0u8; 9];
    r.read_exact(&mut header)?;
    let len = usize::from(header[0]) << 16 | usize::from(header[1]) << 8 | usize::from(header[2]);
    let kind = header[3];
    let flags = header[4];
    // Mask the reserved high bit of the stream id.
    let stream_id = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7FFF_FFFF;
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;
    Ok(Frame {
        kind,
        flags,
        stream_id,
        payload,
    })
}

/// Append an HPACK *literal header field without indexing — new name* entry
/// (RFC 7541 §6.2.2): a `0x00` prefix byte, then the Huffman-free name and
/// value strings.
fn hpack_literal(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(0x00); // pattern 0000, name index 0 ⇒ literal name follows
    hpack_string(out, name);
    hpack_string(out, value);
}

/// Append an HPACK string literal: a 7-bit-prefix length (H bit = 0, no
/// Huffman) followed by the raw bytes.
fn hpack_string(out: &mut Vec<u8>, s: &str) {
    hpack_int(out, s.len(), 7, 0x00);
    out.extend_from_slice(s.as_bytes());
}

/// Append an HPACK variable-length integer with an `prefix_bits`-bit prefix
/// (RFC 7541 §5.1).  `first_byte_flags` supplies the bits above the prefix in
/// the first byte (e.g. the Huffman flag for string lengths).
fn hpack_int(out: &mut Vec<u8>, value: usize, prefix_bits: u8, first_byte_flags: u8) {
    let max_prefix = (1usize << prefix_bits) - 1;
    if value < max_prefix {
        #[allow(clippy::cast_possible_truncation)]
        out.push(first_byte_flags | value as u8);
        return;
    }
    #[allow(clippy::cast_possible_truncation)]
    out.push(first_byte_flags | max_prefix as u8);
    let mut rest = value - max_prefix;
    while rest >= 128 {
        #[allow(clippy::cast_possible_truncation)]
        out.push((rest % 128 + 128) as u8);
        rest /= 128;
    }
    #[allow(clippy::cast_possible_truncation)]
    out.push(rest as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hpack_short_string_is_length_prefixed() {
        let mut out = Vec::new();
        hpack_string(&mut out, "POST");
        assert_eq!(out, vec![0x04, b'P', b'O', b'S', b'T']);
    }

    #[test]
    fn hpack_literal_header_layout() {
        let mut out = Vec::new();
        hpack_literal(&mut out, "te", "trailers");
        // 0x00 (literal/new name), 0x02 "te", 0x08 "trailers".
        let mut expected = vec![0x00, 0x02, b't', b'e', 0x08];
        expected.extend_from_slice(b"trailers");
        assert_eq!(out, expected);
    }

    #[test]
    fn hpack_int_uses_continuation_above_prefix() {
        // 1337 with a 5-bit prefix is the canonical RFC 7541 example:
        // 31, then 154, then 10.
        let mut out = Vec::new();
        hpack_int(&mut out, 1337, 5, 0x00);
        assert_eq!(out, vec![31, 154, 10]);
    }

    #[test]
    fn frame_header_round_trips() {
        let mut buf = Vec::new();
        write_frame(&mut buf, FRAME_DATA, FLAG_END_STREAM, 1, b"hi").unwrap();
        let frame = read_frame(&mut &buf[..]).unwrap();
        assert_eq!(frame.kind, FRAME_DATA);
        assert_eq!(frame.flags, FLAG_END_STREAM);
        assert_eq!(frame.stream_id, 1);
        assert_eq!(frame.payload, b"hi");
    }
}
