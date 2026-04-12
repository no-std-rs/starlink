//! `grpc.reflection.v1.ServerReflection` request types.
//!
//! The gRPC reflection service lets a client ask a running server for the
//! file descriptors of the protobuf types it uses.  We use it to pull a
//! `FileDescriptorSet` off a live dish without ever vendoring a `.proto`
//! file — see the workspace README for why.
//!
//! Only the `ServerReflectionInfo` RPC matters here.  It is bidi-streaming
//! in the proto definition, but in practice every interesting query is a
//! single request followed by a single response, so the transport layer
//! only needs to handle unary-over-streaming (one message each way on a
//! stream that then closes).
//!
//! Proto reference (paraphrased):
//!
//! ```proto
//! package grpc.reflection.v1;
//! message ServerReflectionRequest {
//!   string host = 1;
//!   oneof message_request {
//!     string file_by_filename = 3;
//!     string file_containing_symbol = 4;
//!     ExtensionRequest file_containing_extension = 5;
//!     string all_extension_numbers_of_type = 6;
//!     string list_services = 7;
//!   }
//! }
//! ```
//!
//! Starlink dishes honour both `grpc.reflection.v1` and the older
//! `v1alpha` name; the wire format is identical and we default to `v1`.

use alloc::string::String;

/// Fully-qualified HTTP/2 `:path` for `grpc.reflection.v1.ServerReflection`.
pub const V1_PATH: &str = "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo";

/// Same RPC under the older `v1alpha` name — some dish firmwares still
/// only register this one; newer firmwares register both.
pub const V1ALPHA_PATH: &str = "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo";

/// `grpc.reflection.v1.ServerReflectionRequest`.
///
/// We only ever populate the `message_request` oneof and leave `host`
/// empty.  `host` is a server-side hint when the same endpoint fronts
/// multiple virtual services; Starlink does not use it.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerReflectionRequest {
    /// Virtual-host disambiguator.  Unused — we always leave this empty.
    #[prost(string, tag = "1")]
    pub host: String,
    /// Which reflection query to run.
    #[prost(oneof = "server_reflection_request::MessageRequest", tags = "3, 4, 7")]
    pub message_request: Option<server_reflection_request::MessageRequest>,
}

impl ServerReflectionRequest {
    /// `ServerReflectionRequest { list_services: "" }`.
    ///
    /// The server response enumerates every service registered on the
    /// endpoint — e.g. `SpaceX.API.Device.Device` and
    /// `grpc.reflection.v1.ServerReflection`.
    #[must_use]
    pub fn list_services() -> Self {
        Self {
            message_request: Some(server_reflection_request::MessageRequest::ListServices(
                String::new(),
            )),
            ..Self::default()
        }
    }

    /// `ServerReflectionRequest { file_containing_symbol: <symbol> }`.
    ///
    /// The server response contains the `FileDescriptorProto` that
    /// defines `symbol` *and* every file transitively imported by it —
    /// in practice one call with `"SpaceX.API.Device.Device"` pulls the
    /// entire Device API schema.
    #[must_use]
    pub fn file_containing_symbol(symbol: impl Into<String>) -> Self {
        Self {
            message_request: Some(
                server_reflection_request::MessageRequest::FileContainingSymbol(symbol.into()),
            ),
            ..Self::default()
        }
    }

    /// `ServerReflectionRequest { file_by_filename: <name> }`.
    ///
    /// Less useful than [`Self::file_containing_symbol`] in practice —
    /// we usually know a symbol we want, not the filename of the
    /// `.proto` it came from.
    #[must_use]
    pub fn file_by_filename(name: impl Into<String>) -> Self {
        Self {
            message_request: Some(server_reflection_request::MessageRequest::FileByFilename(
                name.into(),
            )),
            ..Self::default()
        }
    }
}

/// Oneof types for [`ServerReflectionRequest::message_request`].
pub mod server_reflection_request {
    use alloc::string::String;

    /// One arm of the reflection request oneof.
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum MessageRequest {
        /// `file_by_filename = 3` — fetch the file with the given path.
        #[prost(string, tag = "3")]
        FileByFilename(String),
        /// `file_containing_symbol = 4` — fetch the file defining the
        /// given fully-qualified symbol, plus everything it imports.
        #[prost(string, tag = "4")]
        FileContainingSymbol(String),
        /// `list_services = 7` — list every service on the endpoint.
        /// The payload string is traditionally empty.
        #[prost(string, tag = "7")]
        ListServices(String),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn list_services_is_two_bytes() {
        // Tag (7,LEN) = (7 << 3) | 2 = 58 = 0x3A; length 0.
        assert_eq!(
            ServerReflectionRequest::list_services().encode_to_vec(),
            [0x3A, 0x00]
        );
    }

    #[test]
    fn file_containing_symbol_encodes_device_service() {
        let bytes = ServerReflectionRequest::file_containing_symbol("SpaceX.API.Device.Device")
            .encode_to_vec();
        // Tag (4,LEN) = (4 << 3) | 2 = 34 = 0x22; length 24 = 0x18.
        assert_eq!(bytes[0], 0x22);
        assert_eq!(bytes[1], 0x18);
        assert_eq!(&bytes[2..], b"SpaceX.API.Device.Device");
        assert_eq!(bytes.len(), 2 + 24);
    }

    #[test]
    fn file_by_filename_encodes_symbol_bytes() {
        let bytes = ServerReflectionRequest::file_by_filename("spacex_api/device/device.proto")
            .encode_to_vec();
        // Tag (3,LEN) = (3 << 3) | 2 = 26 = 0x1A.
        assert_eq!(bytes[0], 0x1A);
        assert_eq!(bytes[1], 30); // len("spacex_api/device/device.proto") == 30
        assert_eq!(&bytes[2..], b"spacex_api/device/device.proto");
    }

    /// Round-trip every constructor through prost's decoder.  If the
    /// oneof tag numbers ever drift out of sync with the constructor
    /// they will decode into the wrong variant and this catches it.
    #[test]
    fn every_constructor_round_trips() {
        let cases = [
            ServerReflectionRequest::list_services(),
            ServerReflectionRequest::file_containing_symbol("x"),
            ServerReflectionRequest::file_by_filename("x.proto"),
        ];
        for original in cases {
            let bytes = original.encode_to_vec();
            let decoded = ServerReflectionRequest::decode(&*bytes).expect("decode must succeed");
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn reflection_paths_are_slash_rooted() {
        assert!(V1_PATH.starts_with('/'));
        assert!(V1ALPHA_PATH.starts_with('/'));
        assert_eq!(V1_PATH.matches('/').count(), 2);
        assert_eq!(V1ALPHA_PATH.matches('/').count(), 2);
    }
}
