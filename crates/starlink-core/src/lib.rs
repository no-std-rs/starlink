//! gRPC wire framing and the [`Transport`] trait used to talk to a Starlink
//! Device API endpoint.
//!
//! This crate is `no_std + alloc` and has zero external dependencies.  It is
//! intentionally transport-agnostic: callers on Linux wire up a [`Transport`]
//! backed by `rustix` + an HTTP/2 client, and embedded targets can wire up
//! their own HTTP/2 stack behind the same trait.  The crate itself knows
//! nothing about sockets, TLS, executors, or DNS.
//!
//! # What "framing" means here
//!
//! gRPC over HTTP/2 frames each message with a five-byte length prefix on top
//! of the protobuf payload:
//!
//! ```text
//! +--------+-------------------+----------------------+
//! | 1 byte | 4 bytes big-endian|  protobuf payload    |
//! | compr. | payload length    |  (`payload length`)  |
//! +--------+-------------------+----------------------+
//! ```
//!
//! [`frame`] and [`unframe`] handle that five-byte prefix.  Everything else
//! (HTTP/2 `:path`, `content-type: application/grpc+proto`, trailers with the
//! `grpc-status` header) is the [`Transport`] implementation's job.
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;
use core::future::Future;

/// A minimal unary gRPC transport.
///
/// The caller hands in the fully-qualified method path (for example
/// `"/SpaceX.API.Device.Device/Handle"`) and the already-framed request body
/// produced by [`frame`].  The transport performs the HTTP/2 request and
/// returns the framed response body, which the caller then hands to
/// [`unframe`] before protobuf-decoding.
///
/// The trait is deliberately narrower than full gRPC: no streaming, no
/// metadata, no deadlines.  Those can be layered on later without changing
/// this trait.
pub trait Transport {
    /// Transport-specific error type.
    type Error: fmt::Debug;

    /// Perform a unary gRPC call.
    fn unary(
        &mut self,
        path: &str,
        body: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>>;
}

/// Prepend the five-byte gRPC length prefix to a protobuf-encoded message.
///
/// Byte 0 is the compression flag (always `0` — no compression).  Bytes
/// 1..=4 are the big-endian `u32` payload length.
///
/// # Panics
///
/// Panics (via the `as` cast) if `payload.len()` exceeds `u32::MAX`, which
/// would in any case be rejected by the peer.
#[must_use]
pub fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + payload.len());
    out.push(0);
    #[allow(clippy::cast_possible_truncation)]
    let len = payload.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Strip the five-byte gRPC length prefix from a response body and return a
/// slice of just the protobuf payload.
///
/// # Errors
///
/// Returns [`FrameError`] if the buffer is shorter than five bytes, indicates
/// compression we do not support, or the declared length does not match the
/// buffer length.
pub fn unframe(body: &[u8]) -> Result<&[u8], FrameError> {
    if body.len() < 5 {
        return Err(FrameError::Short { got: body.len() });
    }
    if body[0] != 0 {
        return Err(FrameError::Compressed { flag: body[0] });
    }
    let len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    let actual = body.len() - 5;
    if actual != len {
        return Err(FrameError::LengthMismatch {
            declared: len,
            actual,
        });
    }
    Ok(&body[5..])
}

/// Errors reported by [`unframe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Buffer shorter than the five-byte gRPC length prefix.
    Short {
        /// Number of bytes actually received.
        got: usize,
    },
    /// Compression flag byte was non-zero.  This crate does not decompress.
    Compressed {
        /// The unexpected flag byte.
        flag: u8,
    },
    /// The declared payload length does not match the number of bytes that
    /// follow the five-byte prefix.
    LengthMismatch {
        /// Length declared in the prefix.
        declared: usize,
        /// Number of payload bytes actually present.
        actual: usize,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Short { got } => write!(f, "gRPC frame shorter than 5-byte prefix (got {got})"),
            Self::Compressed { flag } => {
                write!(f, "gRPC frame has unsupported compression flag {flag:#x}")
            }
            Self::LengthMismatch { declared, actual } => write!(
                f,
                "gRPC frame length mismatch: prefix declared {declared} bytes, found {actual}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_prepends_five_byte_prefix() {
        let framed = frame(b"hello");
        assert_eq!(
            framed,
            [0x00, 0x00, 0x00, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn frame_empty_payload() {
        let framed = frame(b"");
        assert_eq!(framed, [0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn unframe_accepts_what_frame_produced() {
        let payload = b"protobuf-bytes";
        let framed = frame(payload);
        assert_eq!(unframe(&framed).unwrap(), payload);
    }

    #[test]
    fn unframe_empty_payload() {
        let framed = frame(b"");
        assert_eq!(unframe(&framed).unwrap(), b"");
    }

    #[test]
    fn unframe_rejects_short_buffer() {
        assert_eq!(unframe(&[0, 0, 0, 1]), Err(FrameError::Short { got: 4 }));
    }

    #[test]
    fn unframe_rejects_compression_flag() {
        let mut framed = frame(b"x");
        framed[0] = 1;
        assert_eq!(unframe(&framed), Err(FrameError::Compressed { flag: 1 }));
    }

    #[test]
    fn unframe_rejects_length_mismatch() {
        // Declares 10 bytes of payload but only supplies 1.
        let framed = [0x00, 0x00, 0x00, 0x00, 0x0a, b'x'];
        assert_eq!(
            unframe(&framed),
            Err(FrameError::LengthMismatch {
                declared: 10,
                actual: 1
            })
        );
    }
}
