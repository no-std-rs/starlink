//! `SpaceX.API.Device.Device` request types.
//!
//! The dish exposes a single unary method, `Handle`, which takes a
//! `SpaceX.API.Device.Request` message whose body is a `oneof` of roughly
//! ninety arms — `get_status`, `reboot`, `dish_get_obstruction_map`, ….
//! We currently model just the read-only arms we plan to call plus
//! `reboot`, which is included because the encoding is trivial and
//! symmetric (callers still have to gate it behind an explicit flag).
//!
//! Each request arm is an empty sub-message, so on the wire it is
//! `tag(field, LEN) || varint(0)` — three or four bytes total.  Tag
//! numbers come from reflection-describe output captured on firmware
//! `2026.04.03.cr77363`.
//!
//! # Example
//!
//! ```
//! use prost::Message;
//! use starlink_proto::device::Request;
//!
//! let body = Request::get_status().encode_to_vec();
//! assert_eq!(body, [0xE2, 0x3E, 0x00]);
//! ```

use alloc::string::String;

/// Fully-qualified HTTP/2 `:path` for the unary `Handle` method.
///
/// Pair this with the payload bytes produced by [`prost::Message::encode_to_vec`]
/// on a [`Request`] and a [`crate::...`]-framed body to make a real call.
pub const HANDLE_PATH: &str = "/SpaceX.API.Device.Device/Handle";

/// `SpaceX.API.Device.Request`.
///
/// The top-level request envelope for the `Handle` RPC.  `id`,
/// `target_id`, and `epoch_id` are authentication-scoped; for the
/// read-only arms we use they can be left at their defaults.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    /// Request ID — paired with [`Response::id`] by the dish.  We do not
    /// use it and leave it at `0`.
    #[prost(uint64, tag = "1")]
    pub id: u64,
    /// Authentication target — empty for unauthenticated queries.
    #[prost(string, tag = "13")]
    pub target_id: String,
    /// Epoch counter for signed requests — unused, leave at `0`.
    #[prost(uint64, tag = "14")]
    pub epoch_id: u64,
    /// The oneof arm identifying which operation this request carries.
    #[prost(
        oneof = "request::Body",
        tags = "1001, 1004, 1006, 1007, 1008, 1009, 1017, 1022, 2008, 3002, 3015, 6000"
    )]
    pub body: Option<request::Body>,
}

impl Request {
    /// Construct a `Request` whose body is the given oneof arm, with all
    /// envelope fields at their defaults.
    #[must_use]
    pub fn new(body: request::Body) -> Self {
        Self {
            body: Some(body),
            ..Self::default()
        }
    }

    /// `Request { get_status: GetStatusRequest{} }`.
    #[must_use]
    pub fn get_status() -> Self {
        Self::new(request::Body::GetStatus(GetStatusRequest {}))
    }

    /// `Request { get_history: GetHistoryRequest{} }`.
    #[must_use]
    pub fn get_history() -> Self {
        Self::new(request::Body::GetHistory(GetHistoryRequest {}))
    }

    /// `Request { get_device_info: GetDeviceInfoRequest{} }`.
    #[must_use]
    pub fn get_device_info() -> Self {
        Self::new(request::Body::GetDeviceInfo(GetDeviceInfoRequest {}))
    }

    /// `Request { get_ping: GetPingRequest{} }`.
    #[must_use]
    pub fn get_ping() -> Self {
        Self::new(request::Body::GetPing(GetPingRequest {}))
    }

    /// `Request { get_location: GetLocationRequest{} }`.
    ///
    /// Returns `PermissionDenied` unless the owner enabled location
    /// queries in the Starlink app.
    #[must_use]
    pub fn get_location() -> Self {
        Self::new(request::Body::GetLocation(GetLocationRequest {}))
    }

    /// `Request { get_persistent_stats: GetPersistentStatsRequest{} }`.
    #[must_use]
    pub fn get_persistent_stats() -> Self {
        Self::new(request::Body::GetPersistentStats(
            GetPersistentStatsRequest {},
        ))
    }

    /// `Request { get_next_id: GetNextIdRequest{} }`.
    ///
    /// Useful as a cheap liveness probe even outside authenticated flows.
    #[must_use]
    pub fn get_next_id() -> Self {
        Self::new(request::Body::GetNextId(GetNextIdRequest {}))
    }

    /// `Request { dish_get_obstruction_map: DishGetObstructionMapRequest{} }`.
    #[must_use]
    pub fn dish_get_obstruction_map() -> Self {
        Self::new(request::Body::DishGetObstructionMap(
            DishGetObstructionMapRequest {},
        ))
    }

    /// `Request { get_diagnostics: GetDiagnosticsRequest{} }`.
    #[must_use]
    pub fn get_diagnostics() -> Self {
        Self::new(request::Body::GetDiagnostics(GetDiagnosticsRequest {}))
    }

    /// `Request { wifi_get_clients: WifiGetClientsRequest{} }`.
    ///
    /// Served by the **router** endpoint (e.g. `192.168.1.1:9000`), not the
    /// dish — it lists the Wi-Fi/Ethernet clients and mesh nodes attached to
    /// the router.
    #[must_use]
    pub fn wifi_get_clients() -> Self {
        Self::new(request::Body::WifiGetClients(WifiGetClientsRequest {}))
    }

    /// `Request { wifi_get_client_history: { client_id } }`.
    ///
    /// Router endpoint.  `client_id` comes from the `client_id` field of a
    /// [`crate::response::WifiClient`] returned by [`Self::wifi_get_clients`].
    #[must_use]
    pub fn wifi_get_client_history(client_id: u32) -> Self {
        Self::new(request::Body::WifiGetClientHistory(
            WifiGetClientHistoryRequest { client_id },
        ))
    }

    /// `Request { reboot: RebootRequest{} }`.
    ///
    /// **Write operation.**  Exposed because the encoding is trivial and
    /// symmetric — callers must still gate it behind an explicit flag.
    #[must_use]
    pub fn reboot() -> Self {
        Self::new(request::Body::Reboot(RebootRequest {}))
    }
}

/// Oneof types for [`Request::body`].
pub mod request {
    /// One arm of the `SpaceX.API.Device.Request.request` oneof.
    ///
    /// Only the arms this crate currently encodes are represented — the
    /// rest of the ~90 oneof arms the dish exposes are intentionally
    /// omitted rather than stubbed with `()` placeholders.
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Body {
        /// `reboot = 1001`.
        #[prost(message, tag = "1001")]
        Reboot(super::RebootRequest),
        /// `get_status = 1004`.
        #[prost(message, tag = "1004")]
        GetStatus(super::GetStatusRequest),
        /// `get_next_id = 1006`.
        #[prost(message, tag = "1006")]
        GetNextId(super::GetNextIdRequest),
        /// `get_history = 1007`.
        #[prost(message, tag = "1007")]
        GetHistory(super::GetHistoryRequest),
        /// `get_device_info = 1008`.
        #[prost(message, tag = "1008")]
        GetDeviceInfo(super::GetDeviceInfoRequest),
        /// `get_ping = 1009`.
        #[prost(message, tag = "1009")]
        GetPing(super::GetPingRequest),
        /// `get_location = 1017`.
        #[prost(message, tag = "1017")]
        GetLocation(super::GetLocationRequest),
        /// `get_persistent_stats = 1022`.
        #[prost(message, tag = "1022")]
        GetPersistentStats(super::GetPersistentStatsRequest),
        /// `dish_get_obstruction_map = 2008`.
        #[prost(message, tag = "2008")]
        DishGetObstructionMap(super::DishGetObstructionMapRequest),
        /// `wifi_get_clients = 3002`.
        #[prost(message, tag = "3002")]
        WifiGetClients(super::WifiGetClientsRequest),
        /// `wifi_get_client_history = 3015`.
        #[prost(message, tag = "3015")]
        WifiGetClientHistory(super::WifiGetClientHistoryRequest),
        /// `get_diagnostics = 6000`.
        #[prost(message, tag = "6000")]
        GetDiagnostics(super::GetDiagnosticsRequest),
    }
}

/// `SpaceX.API.Device.RebootRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct RebootRequest {}

/// `SpaceX.API.Device.GetStatusRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetStatusRequest {}

/// `SpaceX.API.Device.GetNextIdRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetNextIdRequest {}

/// `SpaceX.API.Device.GetHistoryRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetHistoryRequest {}

/// `SpaceX.API.Device.GetDeviceInfoRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetDeviceInfoRequest {}

/// `SpaceX.API.Device.GetPingRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetPingRequest {}

/// `SpaceX.API.Device.GetLocationRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetLocationRequest {}

/// `SpaceX.API.Device.GetPersistentStatsRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetPersistentStatsRequest {}

/// `SpaceX.API.Device.DishGetObstructionMapRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct DishGetObstructionMapRequest {}

/// `SpaceX.API.Device.GetDiagnosticsRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetDiagnosticsRequest {}

/// `SpaceX.API.Device.WifiGetClientsRequest` — empty message.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct WifiGetClientsRequest {}

/// `SpaceX.API.Device.WifiGetClientHistoryRequest` — selects one client by id.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct WifiGetClientHistoryRequest {
    /// The `client_id` of the client whose history to return.
    #[prost(uint32, tag = "2")]
    pub client_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn get_status_bytes_are_well_known() {
        // (1004 << 3) | 2 = 8034 → [0xE2, 0x3E] + length 0.
        assert_eq!(Request::get_status().encode_to_vec(), [0xE2, 0x3E, 0x00]);
    }

    #[test]
    fn get_history_bytes_are_well_known() {
        // (1007 << 3) | 2 = 8058 → [0xFA, 0x3E] + length 0.
        assert_eq!(Request::get_history().encode_to_vec(), [0xFA, 0x3E, 0x00]);
    }

    #[test]
    fn get_device_info_bytes_are_well_known() {
        // (1008 << 3) | 2 = 8066 → [0x82, 0x3F] + length 0.
        assert_eq!(
            Request::get_device_info().encode_to_vec(),
            [0x82, 0x3F, 0x00]
        );
    }

    #[test]
    fn get_location_bytes_are_well_known() {
        // (1017 << 3) | 2 = 8138 → [0xCA, 0x3F] + length 0.
        assert_eq!(Request::get_location().encode_to_vec(), [0xCA, 0x3F, 0x00]);
    }

    #[test]
    fn dish_get_obstruction_map_bytes_are_well_known() {
        // (2008 << 3) | 2 = 16066 → [0xC2, 0x7D] + length 0.
        assert_eq!(
            Request::dish_get_obstruction_map().encode_to_vec(),
            [0xC2, 0x7D, 0x00]
        );
    }

    #[test]
    fn get_diagnostics_bytes_are_well_known() {
        // (6000 << 3) | 2 = 48002 → [0x82, 0xF7, 0x02] + length 0.
        assert_eq!(
            Request::get_diagnostics().encode_to_vec(),
            [0x82, 0xF7, 0x02, 0x00]
        );
    }

    /// Every constructor round-trips through prost's decoder back to the
    /// same body variant — catches accidental tag drift between the
    /// convenience method and the oneof declaration.
    #[test]
    fn every_constructor_round_trips() {
        let cases: [Request; 12] = [
            Request::reboot(),
            Request::get_status(),
            Request::get_next_id(),
            Request::get_history(),
            Request::get_device_info(),
            Request::get_ping(),
            Request::get_location(),
            Request::get_persistent_stats(),
            Request::dish_get_obstruction_map(),
            Request::wifi_get_clients(),
            Request::wifi_get_client_history(1_582_731_053),
            Request::get_diagnostics(),
        ];
        for original in cases {
            let bytes = original.encode_to_vec();
            let decoded = Request::decode(&*bytes).expect("decode must succeed");
            assert_eq!(decoded, original);
        }
    }

    /// `HANDLE_PATH` is consumed by the transport layer as an HTTP/2 `:path`
    /// header value.  Structural invariants: leading slash, single method
    /// suffix.
    #[test]
    fn handle_path_shape() {
        assert_eq!(HANDLE_PATH, "/SpaceX.API.Device.Device/Handle");
        assert!(HANDLE_PATH.starts_with('/'));
        assert_eq!(HANDLE_PATH.matches('/').count(), 2);
    }
}
