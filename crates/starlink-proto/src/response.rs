//! `SpaceX.API.Device.Response` decode types.
//!
//! The `Handle` RPC returns a `SpaceX.API.Device.Response` whose `response`
//! field is a `oneof` of the same shape as the request's — one arm per
//! operation, but using the *response* tag numbers, which differ from the
//! request arms.  Notably a `get_status` request (request arm `1004`) comes
//! back in the `dish_get_status` response arm (`2004`); the request and
//! response oneofs are numbered independently.
//!
//! Only the arms and fields we actually read are modelled here.  prost skips
//! unknown fields on decode, so a partial [`DishGetStatusResponse`] decodes a
//! full wire message and silently ignores the ~50 fields we do not name.
//!
//! Tag numbers were captured from `reflection`-describe output on a live dish
//! running firmware `2026.06.15.mr81291` (hardware `mini1_prod2`).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// `SpaceX.API.Device.Response` — the envelope returned by `Handle`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    /// Echoes [`crate::device::Request::id`].
    #[prost(uint64, tag = "1")]
    pub id: u64,
    /// Application-level status.  `code == 0` means success; a non-zero code
    /// with a `message` is how the dish reports e.g. `PermissionDenied` for
    /// `get_location` without surfacing a gRPC-level error.
    #[prost(message, optional, tag = "2")]
    pub status: Option<Status>,
    /// Schema/API version the dish speaks.  `42` on the captured firmware.
    #[prost(uint64, tag = "3")]
    pub api_version: u64,
    /// The populated response arm.
    #[prost(
        oneof = "response::Body",
        tags = "1004, 1009, 1015, 1016, 1017, 1023, 1035, 1037, 2003, 2004, 2006, 2008, 2011, 3002, 3004, 3006, 3007, 3015, 4003, 4004, 6001, 7000"
    )]
    pub body: Option<response::Body>,
}

/// Oneof types for [`Response::body`].
// Mirrors prost's generated layout (a `mod` named after the oneof field, here
// `response`), which collides in name with this file's module.
#[allow(clippy::module_inception)]
pub mod response {
    /// One arm of the `SpaceX.API.Device.Response.response` oneof.
    ///
    /// Only the arms this crate decodes are represented; the rest of the
    /// ~90 response arms are intentionally omitted.
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Body {
        /// `get_device_info = 1004` — reply to a `get_device_info` request.
        #[prost(message, tag = "1004")]
        GetDeviceInfo(super::GetDeviceInfoResponse),
        /// `get_ping = 1009` — reply to a `get_ping` request.
        #[prost(message, tag = "1009")]
        GetPing(super::GetPingResponse),
        /// `get_network_interfaces = 1015`.
        #[prost(message, tag = "1015")]
        GetNetworkInterfaces(super::GetNetworkInterfacesResponse),
        /// `ping_host = 1016`.
        #[prost(message, tag = "1016")]
        PingHost(super::PingHostResponse),
        /// `get_location = 1017`.
        #[prost(message, tag = "1017")]
        GetLocation(super::GetLocationResponse),
        /// `get_connections = 1023`.
        #[prost(message, tag = "1023")]
        GetConnections(super::GetConnectionsResponse),
        /// `get_radio_stats = 1035`.
        #[prost(message, tag = "1035")]
        GetRadioStats(super::GetRadioStatsResponse),
        /// `time = 1037`.
        #[prost(message, tag = "1037")]
        GetTime(super::GetTimeResponse),
        /// `dish_get_context = 2003`.
        #[prost(message, tag = "2003")]
        DishGetContext(super::DishGetContextResponse),
        /// `dish_get_status = 2004` — the reply to a `get_status` request.
        #[prost(message, tag = "2004")]
        DishGetStatus(super::DishGetStatusResponse),
        /// `dish_get_history = 2006` — the reply to a `get_history` request.
        #[prost(message, tag = "2006")]
        DishGetHistory(super::DishGetHistoryResponse),
        /// `dish_get_obstruction_map = 2008` — the sky-view obstruction grid.
        #[prost(message, tag = "2008")]
        DishGetObstructionMap(super::DishGetObstructionMapResponse),
        /// `dish_get_config = 2011`.
        #[prost(message, tag = "2011")]
        DishGetConfig(super::DishGetConfigResponse),
        /// `wifi_get_clients = 3002` — the reply to a `wifi_get_clients`
        /// request (router endpoint).
        #[prost(message, tag = "3002")]
        WifiGetClients(super::WifiGetClientsResponse),
        /// `wifi_get_status = 3004` — the router's reply to a `get_status`
        /// request (router endpoint).
        #[prost(message, tag = "3004")]
        WifiGetStatus(super::WifiGetStatusResponse),
        /// `wifi_get_history = 3006` — the router's reply to a `get_history`
        /// request (router endpoint).
        #[prost(message, tag = "3006")]
        WifiGetHistory(super::WifiGetHistoryResponse),
        /// `wifi_get_ping_metrics = 3007` (router endpoint).
        #[prost(message, tag = "3007")]
        WifiGetPingMetrics(super::WifiGetPingMetricsResponse),
        /// `wifi_get_client_history = 3015` — per-client throughput history
        /// (router endpoint).
        #[prost(message, tag = "3015")]
        WifiGetClientHistory(super::WifiGetClientHistoryResponse),
        /// `transceiver_get_status = 4003`.
        #[prost(message, tag = "4003")]
        TransceiverGetStatus(super::TransceiverGetStatusResponse),
        /// `transceiver_get_telemetry = 4004`.
        #[prost(message, tag = "4004")]
        TransceiverGetTelemetry(super::TransceiverGetTelemetryResponse),
        /// `dish_get_diagnostics = 6001` — reply to `get_diagnostics` on the
        /// dish endpoint.
        #[prost(message, tag = "6001")]
        DishGetDiagnostics(super::DishGetDiagnosticsResponse),
        /// `get_gnss_measurement = 7000` — raw per-satellite GNSS measurements.
        #[prost(message, tag = "7000")]
        GetGnssMeasurement(super::GetGnssMeasurementResponse),
    }
}

/// `SpaceX.API.Status.Status`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Status {
    /// Status code — `0` is success.
    #[prost(int32, tag = "1")]
    pub code: i32,
    /// Human-readable detail, empty on success.
    #[prost(string, tag = "2")]
    pub message: String,
}

/// `SpaceX.API.Device.DishGetStatusResponse` (decode subset).
///
/// Field numbers above ~1000 are dish-specific telemetry; the low-numbered
/// `device_info`/`device_state` fields are shared with other devices.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishGetStatusResponse {
    /// Hardware/software identity.
    #[prost(message, optional, tag = "1")]
    pub device_info: Option<DeviceInfo>,
    /// Runtime state (uptime).
    #[prost(message, optional, tag = "2")]
    pub device_state: Option<DeviceState>,
    /// Rolling fraction of pings dropped to the `PoP`, `0.0..=1.0`.
    #[prost(float, tag = "1003")]
    pub pop_ping_drop_rate: f32,
    /// Obstruction statistics.
    #[prost(message, optional, tag = "1004")]
    pub obstruction_stats: Option<DishObstructionStats>,
    /// Measured downlink throughput, bits/s.
    #[prost(float, tag = "1007")]
    pub downlink_throughput_bps: f32,
    /// Measured uplink throughput, bits/s.
    #[prost(float, tag = "1008")]
    pub uplink_throughput_bps: f32,
    /// Round-trip latency to the `PoP`, milliseconds.
    #[prost(float, tag = "1009")]
    pub pop_ping_latency_ms: f32,
    /// Antenna boresight azimuth, degrees.
    #[prost(float, tag = "1011")]
    pub boresight_azimuth_deg: f32,
    /// Antenna boresight elevation, degrees.
    #[prost(float, tag = "1012")]
    pub boresight_elevation_deg: f32,
    /// GPS fix statistics.
    #[prost(message, optional, tag = "1015")]
    pub gps_stats: Option<DishGpsStats>,
    /// Negotiated Ethernet link speed, Mbps.
    #[prost(int32, tag = "1016")]
    pub eth_speed_mbps: i32,
    /// Detailed antenna alignment, including the *desired* boresight the dish
    /// wants you to aim at versus where it currently points.
    #[prost(message, optional, tag = "1027")]
    pub alignment_stats: Option<AlignmentStats>,
    /// Friendly names of the routers/mesh nodes the dish sees downstream.
    #[prost(string, repeated, tag = "1040")]
    pub connected_routers: Vec<String>,
    /// Downstream routers keyed by id, with role and last-seen timestamp.
    #[prost(btree_map = "string, message", tag = "1050")]
    pub downstream_routers: BTreeMap<String, RouterInfo>,
}

/// `SpaceX.API.Device.AlignmentStats` (decode subset).
///
/// `boresight_*` is where the dish currently points; `desired_boresight_*` is
/// where it wants to point.  On a manually-aimed dish (no actuators) the gap
/// between the two is your aiming correction.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct AlignmentStats {
    /// `HasActuators` enum — whether the dish can self-aim.  See
    /// [`has_actuators_name`].
    #[prost(int32, tag = "1")]
    pub has_actuators: i32,
    /// Mechanical tilt of the dish face, degrees.
    #[prost(float, tag = "3")]
    pub tilt_angle_deg: f32,
    /// Current boresight azimuth, degrees.
    #[prost(float, tag = "4")]
    pub boresight_azimuth_deg: f32,
    /// Current boresight elevation, degrees.
    #[prost(float, tag = "5")]
    pub boresight_elevation_deg: f32,
    /// `AttitudeEstimationState` enum — attitude filter state.  See
    /// [`attitude_estimation_state_name`].
    #[prost(int32, tag = "6")]
    pub attitude_estimation_state: i32,
    /// 1-sigma uncertainty in the attitude estimate, degrees.
    #[prost(float, tag = "7")]
    pub attitude_uncertainty_deg: f32,
    /// Azimuth the dish wants to point at, degrees.
    #[prost(float, tag = "8")]
    pub desired_boresight_azimuth_deg: f32,
    /// Elevation the dish wants to point at, degrees.
    #[prost(float, tag = "9")]
    pub desired_boresight_elevation_deg: f32,
}

/// `SpaceX.API.Device.RouterInfo` — one downstream mesh node.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct RouterInfo {
    /// `RouterRole` enum.  See [`router_role_name`].
    #[prost(int32, tag = "1")]
    pub role: i32,
    /// Last-seen time, nanoseconds since the Unix epoch.
    #[prost(int64, tag = "2")]
    pub last_seen: i64,
}

/// Map a `SpaceX.API.Device.HasActuators` enum value to its name.
#[must_use]
pub fn has_actuators_name(v: i32) -> &'static str {
    match v {
        1 => "YES",
        2 => "NO",
        _ => "UNKNOWN",
    }
}

/// Map a `SpaceX.API.Device.AttitudeEstimationState` enum value to its name.
#[must_use]
pub fn attitude_estimation_state_name(v: i32) -> &'static str {
    match v {
        0 => "FILTER_RESET",
        1 => "FILTER_UNCONVERGED",
        2 => "FILTER_CONVERGED",
        3 => "FILTER_FAULTED",
        4 => "FILTER_INVALID",
        _ => "UNKNOWN",
    }
}

/// Map a `SpaceX.API.Device.RouterRole` enum value to its name.
#[must_use]
pub fn router_role_name(v: i32) -> &'static str {
    match v {
        1 => "CONTROLLER",
        2 => "REPEATER",
        3 => "BYPASSED",
        _ => "UNSPECIFIED",
    }
}

/// `SpaceX.API.Device.DeviceInfo` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeviceInfo {
    /// Dish identifier, e.g. `ut40909c84-...`.
    #[prost(string, tag = "1")]
    pub id: String,
    /// Hardware version string, e.g. `mini1_prod2`.
    #[prost(string, tag = "2")]
    pub hardware_version: String,
    /// Firmware version, e.g. `2026.06.15.mr81291`.
    #[prost(string, tag = "3")]
    pub software_version: String,
    /// Two-letter country code.
    #[prost(string, tag = "4")]
    pub country_code: String,
    /// UTC offset in seconds.
    #[prost(int32, tag = "5")]
    pub utc_offset_s: i32,
    /// Number of times the dish has booted.
    #[prost(int32, tag = "8")]
    pub bootcount: i32,
    /// Monotonic generation counter.
    #[prost(int64, tag = "12")]
    pub generation_number: i64,
    /// Build identifier.
    #[prost(string, tag = "15")]
    pub build_id: String,
}

/// `SpaceX.API.Device.DeviceState`.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct DeviceState {
    /// Seconds since boot.
    #[prost(uint64, tag = "1")]
    pub uptime_s: u64,
}

/// `SpaceX.API.Device.DishObstructionStats` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishObstructionStats {
    /// Fraction of the sky view currently obstructed, `0.0..=1.0`.
    #[prost(float, tag = "1")]
    pub fraction_obstructed: f32,
    /// Seconds of valid obstruction observations.
    #[prost(float, tag = "4")]
    pub valid_s: f32,
    /// Whether the dish is obstructed right now.
    #[prost(bool, tag = "5")]
    pub currently_obstructed: bool,
    /// Average duration of a prolonged obstruction event, seconds.
    #[prost(float, tag = "6")]
    pub avg_prolonged_obstruction_duration_s: f32,
    /// Average interval between prolonged obstruction events, seconds
    /// (`NaN` until enough have been observed).
    #[prost(float, tag = "7")]
    pub avg_prolonged_obstruction_interval_s: f32,
    /// Whether the prolonged-obstruction averages are valid yet.
    #[prost(bool, tag = "8")]
    pub avg_prolonged_obstruction_valid: bool,
    /// Total seconds spent obstructed.
    #[prost(float, tag = "9")]
    pub time_obstructed: f32,
    /// Number of valid sky-map patches.
    #[prost(uint32, tag = "10")]
    pub patches_valid: u32,
}

/// `SpaceX.API.Device.DishGpsStats` (decode subset).
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct DishGpsStats {
    /// Whether the GPS fix is valid.
    #[prost(bool, tag = "1")]
    pub gps_valid: bool,
    /// Number of satellites used in the fix.
    #[prost(uint32, tag = "2")]
    pub gps_sats: u32,
}

/// `SpaceX.API.Device.DishGetHistoryResponse` (decode subset).
///
/// Each `repeated` field is a fixed-length ring buffer (900 samples ≈ 15
/// minutes at 1 Hz on the captured firmware).  [`current`](Self::current) is
/// the running write index; `current % len` is the slot the next sample lands
/// in, so the most recent sample is at `(current - 1) % len`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishGetHistoryResponse {
    /// Monotonic sample counter / ring write index.
    #[prost(uint64, tag = "1")]
    pub current: u64,
    /// Fraction of pings dropped per sample, `0.0..=1.0`.
    #[prost(float, repeated, tag = "1001")]
    pub pop_ping_drop_rate: Vec<f32>,
    /// Round-trip latency to the `PoP` per sample, milliseconds.
    #[prost(float, repeated, tag = "1002")]
    pub pop_ping_latency_ms: Vec<f32>,
    /// Downlink throughput per sample, bits/s.
    #[prost(float, repeated, tag = "1003")]
    pub downlink_throughput_bps: Vec<f32>,
    /// Uplink throughput per sample, bits/s.
    #[prost(float, repeated, tag = "1004")]
    pub uplink_throughput_bps: Vec<f32>,
    /// Outage events overlapping the history window.
    #[prost(message, repeated, tag = "1009")]
    pub outages: Vec<DishOutage>,
    /// Input power draw per sample, watts.
    #[prost(float, repeated, tag = "1010")]
    pub power_in: Vec<f32>,
}

/// `SpaceX.API.Device.DishOutage`.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct DishOutage {
    /// `DishOutage.Cause` enum.  See [`outage_cause_name`].
    #[prost(int32, tag = "1")]
    pub cause: i32,
    /// Start time, nanoseconds since the Unix epoch.
    #[prost(int64, tag = "2")]
    pub start_timestamp_ns: i64,
    /// Outage duration, nanoseconds.
    #[prost(uint64, tag = "3")]
    pub duration_ns: u64,
    /// Whether the outage coincided with a satellite handover.
    #[prost(bool, tag = "4")]
    pub did_switch: bool,
}

/// Map a `SpaceX.API.Device.DishOutage.Cause` enum value to its name.
#[must_use]
pub fn outage_cause_name(v: i32) -> &'static str {
    match v {
        1 => "BOOTING",
        2 => "STOWED",
        3 => "THERMAL_SHUTDOWN",
        4 => "NO_SCHEDULE",
        5 => "NO_SATS",
        6 => "OBSTRUCTED",
        7 => "NO_DOWNLINK",
        8 => "NO_PINGS",
        9 => "ACTUATOR_ACTIVITY",
        10 => "CABLE_TEST",
        11 => "SLEEPING",
        13 => "SKY_SEARCH",
        14 => "INHIBIT_RF",
        _ => "UNKNOWN",
    }
}

/// `SpaceX.API.Device.WifiGetClientsResponse` — served by the router endpoint.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WifiGetClientsResponse {
    /// Attached clients and mesh nodes.
    #[prost(message, repeated, tag = "1")]
    pub clients: Vec<WifiClient>,
}

/// `SpaceX.API.Device.WifiClient` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WifiClient {
    /// Friendly device name (may be empty for some clients).
    #[prost(string, tag = "1")]
    pub name: String,
    /// MAC address.
    #[prost(string, tag = "2")]
    pub mac_address: String,
    /// IPv4 address (empty if none leased).
    #[prost(string, tag = "3")]
    pub ip_address: String,
    /// Signal strength, dBm (0 for wired clients).
    #[prost(float, tag = "4")]
    pub signal_strength: f32,
    /// Receive (download) statistics.
    #[prost(message, optional, tag = "5")]
    pub rx_stats: Option<WifiRxStats>,
    /// Transmit (upload) statistics.
    #[prost(message, optional, tag = "6")]
    pub tx_stats: Option<WifiTxStats>,
    /// Seconds since this client associated.
    #[prost(uint32, tag = "7")]
    pub associated_time_s: u32,
    /// `WifiClient.Interface` enum — ETH / `RF_2GHZ` / `RF_5GHZ` / …  See
    /// [`wifi_interface_name`].
    #[prost(int32, tag = "9")]
    pub iface: i32,
    /// Signal-to-noise ratio, dB (0 for wired clients).
    #[prost(float, tag = "10")]
    pub snr: f32,
    /// MAC of the upstream node this client is connected through.
    #[prost(string, tag = "13")]
    pub upstream_mac_address: String,
    /// `WifiClient.Role` enum — CLIENT / REPEATER / CONTROLLER.  See
    /// [`wifi_role_name`].
    #[prost(int32, tag = "14")]
    pub role: i32,
    /// Mesh hops between this client and the controller.
    #[prost(uint32, tag = "32")]
    pub hops_from_controller: u32,
    /// Stable per-client id, used to request this client's history via
    /// [`crate::device::Request::wifi_get_client_history`].  `0` for nodes
    /// that have none (e.g. the controller itself).
    #[prost(uint32, tag = "43")]
    pub client_id: u32,
}

/// `SpaceX.API.Device.WifiClient.RxStats` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct WifiRxStats {
    /// Total bytes received from this client.
    #[prost(uint64, tag = "1")]
    pub bytes: u64,
    /// Current PHY rate, Mbps.
    #[prost(uint32, tag = "8")]
    pub rate_mbps: u32,
}

/// `SpaceX.API.Device.WifiClient.TxStats` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct WifiTxStats {
    /// Total bytes transmitted to this client.
    #[prost(uint64, tag = "1")]
    pub bytes: u64,
    /// Current PHY rate, Mbps.
    #[prost(uint32, tag = "8")]
    pub rate_mbps: u32,
}

/// `SpaceX.API.Device.WifiGetClientHistoryResponse` (decode subset).
///
/// Per-client ring buffers, same `current`-index convention as
/// [`DishGetHistoryResponse`].  `rx_rate_mbps` and `rssi` are often empty
/// (the router only fills them in some conditions).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WifiGetClientHistoryResponse {
    /// Monotonic sample counter / ring write index.
    #[prost(uint64, tag = "1")]
    pub current: u64,
    /// Transmit (to-client / download) throughput per sample, Mbps.
    #[prost(float, repeated, tag = "2")]
    pub tx_throughput_mbps: Vec<f32>,
    /// Receive (from-client / upload) throughput per sample, Mbps.
    #[prost(float, repeated, tag = "3")]
    pub rx_throughput_mbps: Vec<f32>,
    /// Receive PHY rate per sample, Mbps (often empty).
    #[prost(float, repeated, tag = "5")]
    pub rx_rate_mbps: Vec<f32>,
    /// Per-sample RSSI, one signed byte (dBm) each (often empty).
    #[prost(bytes = "vec", tag = "6")]
    pub rssi: Vec<u8>,
}

/// Map a `SpaceX.API.Device.WifiClient.Interface` enum value to its name.
#[must_use]
pub fn wifi_interface_name(v: i32) -> &'static str {
    match v {
        1 => "ETH",
        2 => "RF_2GHZ",
        3 => "RF_5GHZ",
        4 => "RF_5GHZ_HIGH",
        _ => "UNKNOWN",
    }
}

/// Map a `SpaceX.API.Device.WifiClient.Role` enum value to its name.
#[must_use]
pub fn wifi_role_name(v: i32) -> &'static str {
    match v {
        1 => "CLIENT",
        2 => "REPEATER",
        3 => "CONTROLLER",
        _ => "UNKNOWN",
    }
}

// ---------------------------------------------------------------------------
// dish_get_obstruction_map (2008)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.DishGetObstructionMapResponse`.
///
/// A `num_rows × num_cols` row-major grid in [`snr`](Self::snr).  Despite the
/// field name the values are an obstruction/observation map, not a
/// signal-to-noise ratio: `1.0` is clear sky the dish has observed, values in
/// `0.0..1.0` are partially obstructed, and `-1.0` marks cells outside the
/// field of view.  The disk is centred on boresight and spans
/// [`max_theta_deg`](Self::max_theta_deg) from it.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishGetObstructionMapResponse {
    /// Grid height.
    #[prost(uint32, tag = "1")]
    pub num_rows: u32,
    /// Grid width.
    #[prost(uint32, tag = "2")]
    pub num_cols: u32,
    /// Row-major obstruction values, `num_rows * num_cols` of them.
    #[prost(float, repeated, tag = "3")]
    pub snr: Vec<f32>,
    /// Angular radius of the mapped disk from boresight, degrees.
    #[prost(float, tag = "5")]
    pub max_theta_deg: f32,
    /// `ObstructionMapReferenceFrame` enum (`FRAME_UT` = boresight-centred).
    #[prost(int32, tag = "6")]
    pub map_reference_frame: i32,
}

// ---------------------------------------------------------------------------
// dish_get_diagnostics (6001)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.DishGetDiagnosticsResponse` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishGetDiagnosticsResponse {
    /// Dish identifier.
    #[prost(string, tag = "1")]
    pub id: String,
    /// Hardware version string.
    #[prost(string, tag = "2")]
    pub hardware_version: String,
    /// Firmware version.
    #[prost(string, tag = "3")]
    pub software_version: String,
    /// UTC offset, seconds.
    #[prost(int32, tag = "4")]
    pub utc_offset_s: i32,
    /// Active alert flags.
    #[prost(message, optional, tag = "5")]
    pub alerts: Option<DiagnosticsAlerts>,
    /// `DisablementCode` enum — why service is disabled, if it is.  See
    /// [`disablement_code_name`].
    #[prost(int32, tag = "6")]
    pub disablement_code: i32,
    /// `TestResult` enum for the hardware self-test.  See [`test_result_name`].
    #[prost(int32, tag = "7")]
    pub hardware_self_test: i32,
    /// Location (only populated if location sharing is enabled).
    #[prost(message, optional, tag = "8")]
    pub location: Option<DiagnosticsLocation>,
    /// Current and desired boresight.
    #[prost(message, optional, tag = "9")]
    pub alignment_stats: Option<DiagnosticsAlignmentStats>,
    /// Whether the dish is stowed.
    #[prost(bool, tag = "10")]
    pub stowed: bool,
    /// Whether the account is currently overage-rate-limited.
    #[prost(bool, tag = "14")]
    pub overage_rate_limited: bool,
}

/// `DishGetDiagnosticsResponse.Alerts`.
// One bool per hardware alert — the count mirrors the protobuf message, which
// is the clearest mapping; a bitflags newtype would only obscure it.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct DiagnosticsAlerts {
    /// Dish is actively heating itself.
    #[prost(bool, tag = "1")]
    pub dish_is_heating: bool,
    /// Throttling output due to heat.
    #[prost(bool, tag = "2")]
    pub dish_thermal_throttle: bool,
    /// Shut down due to heat.
    #[prost(bool, tag = "3")]
    pub dish_thermal_shutdown: bool,
    /// Power supply throttling due to heat.
    #[prost(bool, tag = "4")]
    pub power_supply_thermal_throttle: bool,
    /// Motors stuck (self-aiming dishes).
    #[prost(bool, tag = "5")]
    pub motors_stuck: bool,
    /// Mast is not near vertical.
    #[prost(bool, tag = "6")]
    pub mast_not_near_vertical: bool,
    /// Ethernet negotiated below 1 Gbps.
    #[prost(bool, tag = "7")]
    pub slow_ethernet_speeds: bool,
    /// A software install is pending a reboot.
    #[prost(bool, tag = "8")]
    pub software_install_pending: bool,
    /// Sky view is obstructed.
    #[prost(bool, tag = "10")]
    pub obstructed: bool,
}

/// `DishGetDiagnosticsResponse.Location`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct DiagnosticsLocation {
    /// Whether location sharing is enabled.
    #[prost(bool, tag = "1")]
    pub enabled: bool,
    /// Latitude, degrees.
    #[prost(double, tag = "2")]
    pub latitude: f64,
    /// Longitude, degrees.
    #[prost(double, tag = "3")]
    pub longitude: f64,
    /// Altitude, metres.
    #[prost(double, tag = "4")]
    pub altitude_meters: f64,
}

/// `DishGetDiagnosticsResponse.AlignmentStats`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct DiagnosticsAlignmentStats {
    /// Current boresight azimuth, degrees.
    #[prost(float, tag = "1")]
    pub boresight_azimuth_deg: f32,
    /// Current boresight elevation, degrees.
    #[prost(float, tag = "2")]
    pub boresight_elevation_deg: f32,
    /// Desired boresight azimuth, degrees.
    #[prost(float, tag = "3")]
    pub desired_boresight_azimuth_deg: f32,
    /// Desired boresight elevation, degrees.
    #[prost(float, tag = "4")]
    pub desired_boresight_elevation_deg: f32,
}

/// Map a `DishGetDiagnosticsResponse.DisablementCode` enum value to its name.
#[must_use]
pub fn disablement_code_name(v: i32) -> &'static str {
    match v {
        1 => "OKAY",
        2 => "NO_ACTIVE_ACCOUNT",
        3 => "TOO_FAR_FROM_SERVICE_ADDRESS",
        4 => "IN_OCEAN",
        6 => "BLOCKED_COUNTRY",
        7 => "DATA_OVERAGE_SANDBOX_POLICY",
        8 => "CELL_IS_DISABLED",
        10 => "ROAM_RESTRICTED",
        11 => "UNKNOWN_LOCATION",
        12 => "ACCOUNT_DISABLED",
        13 => "UNSUPPORTED_VERSION",
        14 => "MOVING_TOO_FAST_FOR_POLICY",
        15 => "UNDER_AVIATION_FLYOVER_LIMITS",
        16 => "BLOCKED_AREA",
        _ => "UNKNOWN",
    }
}

/// Map a `DishGetDiagnosticsResponse.TestResult` enum value to its name.
#[must_use]
pub fn test_result_name(v: i32) -> &'static str {
    match v {
        1 => "PASSED",
        2 => "FAILED",
        _ => "NO_RESULT",
    }
}

// ---------------------------------------------------------------------------
// get_device_info (1004)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetDeviceInfoResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDeviceInfoResponse {
    /// Hardware/software identity.
    #[prost(message, optional, tag = "1")]
    pub device_info: Option<DeviceInfo>,
}

// ---------------------------------------------------------------------------
// get_ping (1009) / ping_host (1016)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetPingResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPingResponse {
    /// Ping results keyed by target name.
    #[prost(btree_map = "string, message", tag = "1")]
    pub results: BTreeMap<String, PingResult>,
}

/// `SpaceX.API.Device.PingHostResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PingHostResponse {
    /// The ping result for the requested host.
    #[prost(message, optional, tag = "1")]
    pub result: Option<PingResult>,
}

/// `SpaceX.API.Device.PingResult` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct PingResult {
    /// Fraction of pings dropped, `0.0..=1.0`.
    #[prost(float, tag = "1")]
    pub drop_rate: f32,
    /// Round-trip latency, milliseconds.
    #[prost(float, tag = "2")]
    pub latency_ms: f32,
}

// ---------------------------------------------------------------------------
// get_network_interfaces (1015)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetNetworkInterfacesResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNetworkInterfacesResponse {
    /// One entry per network interface.
    #[prost(message, repeated, tag = "1006")]
    pub network_interfaces: Vec<NetworkInterface>,
}

/// `SpaceX.API.Device.NetworkInterface` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NetworkInterface {
    /// Interface name (e.g. `eth0`).
    #[prost(string, tag = "1")]
    pub name: String,
    /// Receive byte/packet counters.
    #[prost(message, optional, tag = "2")]
    pub rx_stats: Option<NetIfStats>,
    /// Transmit byte/packet counters.
    #[prost(message, optional, tag = "3")]
    pub tx_stats: Option<NetIfStats>,
    /// Whether the link is up.
    #[prost(bool, tag = "4")]
    pub up: bool,
    /// MAC address.
    #[prost(string, tag = "5")]
    pub mac_address: String,
    /// Assigned IPv4 addresses.
    #[prost(string, repeated, tag = "6")]
    pub ipv4_addresses: Vec<String>,
    /// Assigned IPv6 addresses.
    #[prost(string, repeated, tag = "7")]
    pub ipv6_addresses: Vec<String>,
}

/// `NetworkInterface.RxStats` / `TxStats` (the `bytes`/`packets` subset, which
/// both share).
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct NetIfStats {
    /// Total bytes.
    #[prost(uint64, tag = "1")]
    pub bytes: u64,
    /// Total packets.
    #[prost(uint64, tag = "2")]
    pub packets: u64,
}

// ---------------------------------------------------------------------------
// get_location (1017)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetLocationResponse`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct GetLocationResponse {
    /// Latitude/longitude/altitude.
    #[prost(message, optional, tag = "1")]
    pub lla: Option<LlaPosition>,
    /// `PositionSource` enum.  See [`position_source_name`].
    #[prost(int32, tag = "3")]
    pub source: i32,
    /// 1-sigma position uncertainty, metres.
    #[prost(double, tag = "4")]
    pub sigma_m: f64,
    /// Horizontal speed, metres/second.
    #[prost(double, tag = "5")]
    pub horizontal_speed_mps: f64,
    /// Vertical speed, metres/second.
    #[prost(double, tag = "6")]
    pub vertical_speed_mps: f64,
}

/// `SpaceX.API.Device.LLAPosition`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct LlaPosition {
    /// Latitude, degrees.
    #[prost(double, tag = "1")]
    pub lat: f64,
    /// Longitude, degrees.
    #[prost(double, tag = "2")]
    pub lon: f64,
    /// Altitude, metres.
    #[prost(double, tag = "3")]
    pub alt: f64,
}

/// Map a `SpaceX.API.Device.PositionSource` enum value to its name.
#[must_use]
pub fn position_source_name(v: i32) -> &'static str {
    match v {
        0 => "AUTO",
        1 => "NONE",
        2 => "UT_INFO",
        3 => "EXTERNAL",
        4 => "GPS",
        5 => "STARLINK",
        6 => "GNC_FUSED",
        7 => "GNC_BAD_SAT",
        8 => "GNC_GPS",
        9 => "GNC_PNT",
        10 => "GNC_STATIC",
        _ => "UNKNOWN",
    }
}

// ---------------------------------------------------------------------------
// get_connections (1023)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetConnectionsResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConnectionsResponse {
    /// Backend service connections keyed by service name.
    #[prost(btree_map = "string, message", tag = "1")]
    pub services: BTreeMap<String, ServiceConnection>,
}

/// `GetConnectionsResponse.ServiceConnection`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServiceConnection {
    /// Remote address.
    #[prost(string, tag = "1")]
    pub address: String,
    /// Seconds since the last successful contact.
    #[prost(int32, tag = "2")]
    pub seconds_since_success: i32,
}

// ---------------------------------------------------------------------------
// get_radio_stats (1035)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetRadioStatsResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRadioStatsResponse {
    /// Per-band radio statistics.
    #[prost(message, repeated, tag = "1")]
    pub radio_stats: Vec<RadioStats>,
}

/// `SpaceX.API.Device.RadioStats` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct RadioStats {
    /// `WifiConfig.Band` enum value.
    #[prost(int32, tag = "1")]
    pub band: i32,
    /// Thermal state.
    #[prost(message, optional, tag = "4")]
    pub thermal_status: Option<RadioThermalStatus>,
    /// Per-antenna RSSI.
    #[prost(message, optional, tag = "5")]
    pub antenna_status: Option<RadioAntennaStatus>,
}

/// `RadioStats.ThermalStatus` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct RadioThermalStatus {
    /// Thermal level.
    #[prost(uint32, tag = "1")]
    pub level: u32,
    /// Temperature, °C.
    #[prost(double, tag = "3")]
    pub temp2: f64,
    /// Power reduction applied, percent.
    #[prost(uint32, tag = "4")]
    pub power_reduction: u32,
    /// Duty cycle, percent.
    #[prost(uint32, tag = "5")]
    pub duty_cycle: u32,
}

/// `RadioStats.AntennaStatus`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct RadioAntennaStatus {
    /// RSSI on antenna 1, dBm.
    #[prost(float, tag = "1")]
    pub rssi1: f32,
    /// RSSI on antenna 2, dBm.
    #[prost(float, tag = "2")]
    pub rssi2: f32,
    /// RSSI on antenna 3, dBm.
    #[prost(float, tag = "3")]
    pub rssi3: f32,
    /// RSSI on antenna 4, dBm.
    #[prost(float, tag = "4")]
    pub rssi4: f32,
}

// ---------------------------------------------------------------------------
// time (1037)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetTimeResponse`.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GetTimeResponse {
    /// Current time, nanoseconds since the Unix epoch.
    #[prost(int64, tag = "1")]
    pub unix_nano: i64,
}

// ---------------------------------------------------------------------------
// dish_get_context (2003)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.DishGetContextResponse` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishGetContextResponse {
    /// Hardware/software identity.
    #[prost(message, optional, tag = "1")]
    pub device_info: Option<DeviceInfo>,
    /// Fraction of sky obstructed, `0.0..=1.0`.
    #[prost(float, tag = "2")]
    pub obstruction_fraction: f32,
    /// Seconds of valid obstruction observation.
    #[prost(float, tag = "3")]
    pub obstruction_valid_s: f32,
    /// Serving cell id.
    #[prost(uint32, tag = "4")]
    pub cell_id: u32,
    /// Serving `PoP` rack id.
    #[prost(uint32, tag = "5")]
    pub pop_rack_id: u32,
    /// Seconds until the current scheduling slot ends.
    #[prost(float, tag = "6")]
    pub seconds_to_slot_end: f32,
    /// Runtime state (uptime).
    #[prost(message, optional, tag = "7")]
    pub device_state: Option<DeviceState>,
    /// Satellite id at session start.
    #[prost(uint32, tag = "8")]
    pub initial_satellite_id: u32,
    /// Gateway id at session start.
    #[prost(uint32, tag = "9")]
    pub initial_gateway_id: u32,
    /// Whether the dish is on a backup beam.
    #[prost(bool, tag = "10")]
    pub on_backup_beam: bool,
    /// 15-second mean `PoP` ping drop rate.
    #[prost(float, tag = "13")]
    pub pop_ping_drop_rate_15s_mean: f32,
    /// 15-second mean `PoP` ping latency, milliseconds.
    #[prost(float, tag = "14")]
    pub pop_ping_latency_ms_15s_mean: f32,
    /// Total seconds spent obstructed.
    #[prost(float, tag = "20")]
    pub obstruction_time: f32,
    /// Fraction of Ku-band MAC slots active.
    #[prost(float, tag = "22")]
    pub ku_mac_active_ratio: f32,
}

// ---------------------------------------------------------------------------
// dish_get_config (2011)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.DishGetConfigResponse`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DishGetConfigResponse {
    /// The dish configuration.
    #[prost(message, optional, tag = "1")]
    pub dish_config: Option<DishConfig>,
}

/// `SpaceX.API.Device.DishConfig` (decode subset — the live settings, skipping
/// the parallel `apply_*` write-mask booleans).
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct DishConfig {
    /// `SnowMeltMode` enum: 0 `AUTO`, 1 `ALWAYS_ON`, 2 `ALWAYS_OFF`.
    #[prost(int32, tag = "1")]
    pub snow_melt_mode: i32,
    /// `LocationRequestMode` enum: 0 `NONE`, 1 `LOCAL`.
    #[prost(int32, tag = "2")]
    pub location_request_mode: i32,
    /// `LevelDishMode` enum: 0 `TILT_LIKE_NORMAL`, 1 `FORCE_LEVEL`.
    #[prost(int32, tag = "3")]
    pub level_dish_mode: i32,
    /// Power-save window start, minutes past midnight.
    #[prost(uint32, tag = "4")]
    pub power_save_start_minutes: u32,
    /// Power-save window duration, minutes.
    #[prost(uint32, tag = "5")]
    pub power_save_duration_minutes: u32,
    /// Whether power-save mode is enabled.
    #[prost(bool, tag = "6")]
    pub power_save_mode: bool,
    /// Whether the 3-day software-update deferral is enabled.
    #[prost(bool, tag = "7")]
    pub swupdate_three_day_deferral_enabled: bool,
    /// Asset class identifier.
    #[prost(uint32, tag = "8")]
    pub asset_class: u32,
    /// Preferred reboot hour for software updates.
    #[prost(uint32, tag = "9")]
    pub swupdate_reboot_hour: u32,
}

// ---------------------------------------------------------------------------
// transceiver_get_status (4003) / transceiver_get_telemetry (4004)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.TransceiverGetStatusResponse` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct TransceiverGetStatusResponse {
    /// `TransceiverModulatorState` enum for the modulator.
    #[prost(int32, tag = "1")]
    pub mod_state: i32,
    /// `TransceiverModulatorState` enum for the demodulator.
    #[prost(int32, tag = "2")]
    pub demod_state: i32,
    /// `TransceiverTxRxState` enum for transmit.
    #[prost(int32, tag = "3")]
    pub tx_state: i32,
    /// `TransceiverTxRxState` enum for receive.
    #[prost(int32, tag = "4")]
    pub rx_state: i32,
    /// `DishState` enum (1006).  See [`dish_state_name`].
    #[prost(int32, tag = "1006")]
    pub state: i32,
    /// Modem ASIC temperature, °C.
    #[prost(float, tag = "1009")]
    pub modem_asic_temp: f32,
    /// Transmit IF temperature, °C.
    #[prost(float, tag = "1010")]
    pub tx_if_temp: f32,
}

/// `SpaceX.API.Device.TransceiverGetTelemetryResponse` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct TransceiverGetTelemetryResponse {
    /// Antenna pitch, degrees.
    #[prost(float, tag = "1002")]
    pub antenna_pitch: f32,
    /// Antenna roll, degrees.
    #[prost(float, tag = "1003")]
    pub antenna_roll: f32,
    /// Antenna true heading, degrees.
    #[prost(float, tag = "1005")]
    pub antenna_true_heading: f32,
    /// Serving cell id.
    #[prost(uint32, tag = "1007")]
    pub current_cell_id: u32,
    /// Wideband RSSI peak magnitude, dB.
    #[prost(float, tag = "1009")]
    pub wb_rssi_peak_mag_db: f32,
    /// `PoP` ping drop rate.
    #[prost(float, tag = "1010")]
    pub pop_ping_drop_rate: f32,
    /// Signal-to-noise ratio, dB.
    #[prost(float, tag = "1011")]
    pub snr_db: f32,
    /// Average L1 SNR, dB.
    #[prost(float, tag = "1012")]
    pub l1_snr_avg_db: f32,
    /// Serving (LMAC) satellite id.
    #[prost(uint32, tag = "1015")]
    pub lmac_satellite_id: u32,
    /// Target satellite id.
    #[prost(uint32, tag = "1016")]
    pub target_satellite_id: u32,
}

/// Map a `SpaceX.API.Device.DishState` enum value to its name.
#[must_use]
pub fn dish_state_name(v: i32) -> &'static str {
    match v {
        1 => "CONNECTED",
        2 => "SEARCHING",
        3 => "BOOTING",
        _ => "UNKNOWN",
    }
}

/// Map a `TransceiverModulatorState`/`TransceiverTxRxState` enum value (which
/// share `1 = ENABLED`, `2 = DISABLED`) to its name.
#[must_use]
pub fn transceiver_state_name(v: i32) -> &'static str {
    match v {
        1 => "ENABLED",
        2 => "DISABLED",
        _ => "UNKNOWN",
    }
}

// ---------------------------------------------------------------------------
// get_gnss_measurement (7000)
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.GetGnssMeasurementResponse` (decode subset).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGnssMeasurementResponse {
    /// Reporting device id.
    #[prost(string, tag = "1")]
    pub device_id: String,
    /// One entry per tracked satellite.
    #[prost(message, repeated, tag = "2")]
    pub measurements: Vec<GnssMeasurement>,
}

/// `SpaceX.API.Device.Gnss.Measurement` (decode subset — identity fields only;
/// the nested pseudorange/ephemeris sub-messages are skipped).
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct GnssMeasurement {
    /// Measurement validity time, nanoseconds since the Unix epoch.
    #[prost(int64, tag = "1")]
    pub time_of_validity_ns: i64,
    /// `Gnss.SatelliteSystem` enum (`GPS` / `GLONASS` / `Galileo` / `BeiDou` …).
    #[prost(int32, tag = "2")]
    pub satellite_system: i32,
    /// Satellite PRN / id within its system.
    #[prost(int32, tag = "3")]
    pub prn: i32,
}

/// Map a `SpaceX.API.Device.Gnss.SatelliteSystem` enum value to its name.
#[must_use]
pub fn gnss_system_name(v: i32) -> &'static str {
    match v {
        1 => "GPS",
        2 => "GLONASS",
        3 => "GALILEO",
        4 => "BEIDOU",
        5 => "QZSS",
        6 => "SBAS",
        _ => "UNKNOWN",
    }
}

// ---------------------------------------------------------------------------
// wifi_get_status (3004) — router endpoint
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.WifiGetStatusResponse` (decode subset).
///
/// Returned by the **router** endpoint in reply to a `get_status` request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WifiGetStatusResponse {
    /// Hardware/software identity.
    #[prost(message, optional, tag = "3")]
    pub device_info: Option<DeviceInfo>,
    /// Runtime state (uptime).
    #[prost(message, optional, tag = "4")]
    pub device_state: Option<DeviceState>,
    /// IPv4 WAN address (with CIDR suffix).
    #[prost(string, tag = "1003")]
    pub ipv4_wan_address: String,
    /// Router→internet ping drop rate, `0.0..=1.0`.
    #[prost(float, tag = "1004")]
    pub ping_drop_rate: f32,
    /// Router→internet ping latency, milliseconds.
    #[prost(float, tag = "1005")]
    pub ping_latency_ms: f32,
    /// Active alert flags.
    #[prost(message, optional, tag = "1010")]
    pub alerts: Option<WifiAlerts>,
    /// Router→dish ping drop rate, `0.0..=1.0`.
    #[prost(float, tag = "1012")]
    pub dish_ping_drop_rate: f32,
    /// Router→dish ping latency, milliseconds.
    #[prost(float, tag = "1013")]
    pub dish_ping_latency_ms: f32,
    /// Router→`PoP` ping drop rate, `0.0..=1.0`.
    #[prost(float, tag = "1014")]
    pub pop_ping_drop_rate: f32,
    /// Router→`PoP` ping latency, milliseconds.
    #[prost(float, tag = "1015")]
    pub pop_ping_latency_ms: f32,
    /// IPv6 WAN addresses.
    #[prost(string, repeated, tag = "1017")]
    pub ipv6_wan_addresses: Vec<String>,
    /// 5-minute mean `PoP` ping drop rate.
    #[prost(float, tag = "1020")]
    pub pop_ping_drop_rate_5m: f32,
    /// 5-minute mean internet ping drop rate.
    #[prost(float, tag = "1021")]
    pub ping_drop_rate_5m: f32,
    /// Paired dish identifier.
    #[prost(string, tag = "1023")]
    pub dish_id: String,
    /// Current time, nanoseconds since the Unix epoch.
    #[prost(int64, tag = "1024")]
    pub utc_ns: i64,
    /// Seconds since the public IPv4 address last changed.
    #[prost(float, tag = "1030")]
    pub secs_since_last_public_ipv4_change: f32,
    /// Mesh hops from the controller (0 on the controller itself).
    #[prost(uint32, tag = "1034")]
    pub hops_from_controller: u32,
    /// Whether the router currently has no WAN link.
    #[prost(bool, tag = "1035")]
    pub no_wan_link: bool,
}

/// `SpaceX.API.Device.WifiAlerts` (decode subset of the notable flags).
// One bool per alert flag, mirroring the protobuf message (see `DiagnosticsAlerts`).
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
pub struct WifiAlerts {
    /// Router is thermally throttling.
    #[prost(bool, tag = "1")]
    pub thermal_throttle: bool,
    /// A software install is pending.
    #[prost(bool, tag = "2")]
    pub install_pending: bool,
    /// LAN Ethernet stuck at a 10 Mbps link.
    #[prost(bool, tag = "4")]
    pub lan_eth_slow_link_10: bool,
    /// LAN Ethernet stuck at a 100 Mbps link.
    #[prost(bool, tag = "5")]
    pub lan_eth_slow_link_100: bool,
    /// Poor WAN Ethernet connection.
    #[prost(bool, tag = "10")]
    pub wan_eth_poor_connection: bool,
    /// Mesh backhaul is unreliable.
    #[prost(bool, tag = "12")]
    pub mesh_unreliable_backhaul: bool,
    /// High ping drop on the dish cable.
    #[prost(bool, tag = "21")]
    pub high_cable_ping_drop_rate: bool,
}

// ---------------------------------------------------------------------------
// wifi_get_history (3006) — router endpoint
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.WifiGetHistoryResponse` (decode subset).
///
/// `ping_*` are 1 Hz ring buffers (43200 ≈ 12 h) indexed by
/// [`current`](Self::current); the `*_last_15s` arrays are coarse 15-second
/// buckets (60 ≈ 15 min) indexed by [`current_index_15s`](Self::current_index_15s).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WifiGetHistoryResponse {
    /// Monotonic 1 Hz sample counter / ring write index.
    #[prost(uint64, tag = "1")]
    pub current: u64,
    /// Monotonic 15-second-bucket counter / ring write index.
    #[prost(uint64, tag = "2")]
    pub current_index_15s: u64,
    /// Internet ping drop rate per 1 Hz sample.
    #[prost(float, repeated, tag = "1001")]
    pub ping_drop_rate: Vec<f32>,
    /// Internet ping latency per 1 Hz sample, milliseconds.
    #[prost(float, repeated, tag = "1002")]
    pub ping_latency_ms: Vec<f32>,
    /// `PoP` IPv4 ping drop rate per 15-second bucket.
    #[prost(float, repeated, tag = "1003")]
    pub pop_ipv4_ping_drop_rate_last_15s: Vec<f32>,
    /// `PoP` IPv6 ping drop rate per 15-second bucket.
    #[prost(float, repeated, tag = "1004")]
    pub pop_ipv6_ping_drop_rate_last_15s: Vec<f32>,
    /// Google IPv4 ping drop rate per 15-second bucket.
    #[prost(float, repeated, tag = "1005")]
    pub google_ipv4_ping_drop_rate_last_15s: Vec<f32>,
    /// Google IPv6 ping drop rate per 15-second bucket.
    #[prost(float, repeated, tag = "1006")]
    pub google_ipv6_ping_drop_rate_last_15s: Vec<f32>,
    /// Cloudflare IPv4 ping drop rate per 15-second bucket.
    #[prost(float, repeated, tag = "1007")]
    pub cloudflare_ipv4_ping_drop_rate_last_15s: Vec<f32>,
    /// Cloudflare IPv6 ping drop rate per 15-second bucket.
    #[prost(float, repeated, tag = "1008")]
    pub cloudflare_ipv6_ping_drop_rate_last_15s: Vec<f32>,
}

// ---------------------------------------------------------------------------
// wifi_get_ping_metrics (3007) — router endpoint
// ---------------------------------------------------------------------------

/// `SpaceX.API.Device.WifiGetPingMetricsResponse`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct WifiGetPingMetricsResponse {
    /// Internet-facing ping metrics.
    #[prost(message, optional, tag = "1")]
    pub internet: Option<PingMetrics>,
}

/// `SpaceX.API.Device.PingMetrics` (decode subset).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct PingMetrics {
    /// Mean latency, milliseconds.
    #[prost(float, tag = "1")]
    pub latency_mean_ms: f32,
    /// Latency standard deviation, milliseconds.
    #[prost(float, tag = "2")]
    pub latency_stddev_ms: f32,
    /// Drop rate, `0.0..=1.0`.
    #[prost(float, tag = "6")]
    pub drop_rate: f32,
    /// 5-minute mean drop rate.
    #[prost(float, tag = "7")]
    pub drop_rate_5m: f32,
    /// 1-hour mean drop rate.
    #[prost(float, tag = "8")]
    pub drop_rate_1h: f32,
    /// Seconds since the last successful ping.
    #[prost(float, tag = "10")]
    pub seconds_since_last_success: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use prost::Message;

    /// A `Response` carrying a `dish_get_status` arm round-trips through the
    /// decoder with every modelled field intact — guards against tag drift
    /// between the envelope, the oneof, and the inner message.
    #[test]
    fn dish_get_status_round_trips() {
        let original = Response {
            id: 7,
            status: Some(Status {
                code: 0,
                message: String::new(),
            }),
            api_version: 42,
            body: Some(response::Body::DishGetStatus(DishGetStatusResponse {
                device_info: Some(DeviceInfo {
                    id: "ut-test".to_string(),
                    hardware_version: "mini1_prod2".to_string(),
                    software_version: "2026.06.15.mr81291".to_string(),
                    country_code: "PL".to_string(),
                    utc_offset_s: 3600,
                    bootcount: 227,
                    generation_number: 1_781_893_554,
                    build_id: "build".to_string(),
                }),
                device_state: Some(DeviceState { uptime_s: 1171 }),
                pop_ping_drop_rate: 0.0,
                obstruction_stats: Some(DishObstructionStats {
                    fraction_obstructed: 0.005,
                    valid_s: 1071.0,
                    currently_obstructed: false,
                    avg_prolonged_obstruction_duration_s: 0.0,
                    avg_prolonged_obstruction_interval_s: f32::NAN,
                    avg_prolonged_obstruction_valid: false,
                    time_obstructed: 12.5,
                    patches_valid: 1759,
                }),
                downlink_throughput_bps: 6_780_939.5,
                uplink_throughput_bps: 163_963.73,
                pop_ping_latency_ms: 18.819_225,
                boresight_azimuth_deg: -163.975_74,
                boresight_elevation_deg: 81.044_02,
                gps_stats: Some(DishGpsStats {
                    gps_valid: true,
                    gps_sats: 15,
                }),
                eth_speed_mbps: 1000,
                alignment_stats: Some(AlignmentStats {
                    has_actuators: 2,
                    tilt_angle_deg: 7.95,
                    boresight_azimuth_deg: -164.9,
                    boresight_elevation_deg: 81.04,
                    attitude_estimation_state: 2,
                    attitude_uncertainty_deg: 0.55,
                    desired_boresight_azimuth_deg: 175.0,
                    desired_boresight_elevation_deg: 70.0,
                }),
                connected_routers: alloc::vec!["Router-01000000000000000094A00C".to_string()],
                downstream_routers: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "Router-01000000000000000094A00C".to_string(),
                        RouterInfo {
                            role: 1,
                            last_seen: 1_782_526_447_507_494_709,
                        },
                    );
                    m
                },
            })),
        };

        // NaN never compares equal, so zero it before the round-trip assert.
        let mut original = original;
        if let Some(response::Body::DishGetStatus(s)) = &mut original.body {
            if let Some(o) = &mut s.obstruction_stats {
                o.avg_prolonged_obstruction_interval_s = 0.0;
            }
        }

        let bytes = original.encode_to_vec();
        let decoded = Response::decode(&*bytes).expect("decode must succeed");
        assert_eq!(decoded, original);
    }

    /// A `dish_get_history` response round-trips, including ring buffers and a
    /// nested outage, and lands in the 2006 arm.
    #[test]
    fn dish_get_history_round_trips() {
        let original = Response {
            api_version: 42,
            body: Some(response::Body::DishGetHistory(DishGetHistoryResponse {
                current: 1929,
                pop_ping_drop_rate: alloc::vec![0.0, 0.0, 0.1],
                pop_ping_latency_ms: alloc::vec![18.4, 18.5, 41.0],
                downlink_throughput_bps: alloc::vec![7_025_551.5, 0.0],
                uplink_throughput_bps: alloc::vec![194_691.2],
                outages: alloc::vec![DishOutage {
                    cause: 1,
                    start_timestamp_ns: 1_466_559_688_846_927_899,
                    duration_ns: 93_893_245_349,
                    did_switch: false,
                }],
                power_in: alloc::vec![20.31, 19.94, 20.28],
            })),
            ..Response::default()
        };

        let bytes = original.encode_to_vec();
        let decoded = Response::decode(&*bytes).expect("decode must succeed");
        assert_eq!(decoded, original);
    }

    /// A `wifi_get_clients` response round-trips, including a nested client
    /// with rx/tx stats, and lands in the 3002 arm.
    #[test]
    fn wifi_get_clients_round_trips() {
        let original = Response {
            api_version: 128,
            body: Some(response::Body::WifiGetClients(WifiGetClientsResponse {
                clients: alloc::vec![
                    WifiClient {
                        name: "Controller".to_string(),
                        mac_address: "74:24:9f:24:a0:0c".to_string(),
                        iface: 1,
                        role: 3,
                        ..WifiClient::default()
                    },
                    WifiClient {
                        name: "Xiaomi-17T-Pro".to_string(),
                        mac_address: "aa:e2:08:00:00:00".to_string(),
                        ip_address: "192.168.1.113".to_string(),
                        signal_strength: -85.0,
                        snr: 3.0,
                        iface: 3,
                        role: 1,
                        associated_time_s: 261,
                        upstream_mac_address: "74:24:9f:24:a0:0c".to_string(),
                        hops_from_controller: 1,
                        rx_stats: Some(WifiRxStats {
                            bytes: 43_455_854,
                            rate_mbps: 260,
                        }),
                        tx_stats: Some(WifiTxStats {
                            bytes: 495_162,
                            rate_mbps: 54,
                        }),
                        client_id: 2_298_735_863,
                    },
                ],
            })),
            ..Response::default()
        };

        let bytes = original.encode_to_vec();
        let decoded = Response::decode(&*bytes).expect("decode must succeed");
        assert_eq!(decoded, original);
    }

    /// A `wifi_get_client_history` response round-trips, including the ring
    /// buffers, and lands in the 3015 arm.
    #[test]
    fn wifi_get_client_history_round_trips() {
        let original = Response {
            api_version: 128,
            body: Some(response::Body::WifiGetClientHistory(
                WifiGetClientHistoryResponse {
                    current: 2595,
                    tx_throughput_mbps: alloc::vec![0.0, 0.84, 0.0],
                    rx_throughput_mbps: alloc::vec![45.18, 12.0],
                    rx_rate_mbps: alloc::vec![],
                    rssi: alloc::vec![],
                },
            )),
            ..Response::default()
        };

        let bytes = original.encode_to_vec();
        let decoded = Response::decode(&*bytes).expect("decode must succeed");
        assert_eq!(decoded, original);
    }

    /// The `dish_get_status` arm sits at tag 2004 — verify the envelope key
    /// byte so a future renumber can't silently decode into the wrong arm.
    #[test]
    fn dish_get_status_arm_tag_is_2004() {
        let resp = Response {
            body: Some(response::Body::DishGetStatus(
                DishGetStatusResponse::default(),
            )),
            ..Response::default()
        };
        let bytes = resp.encode_to_vec();
        // (2004 << 3) | 2 = 16034 → varint [0xA2, 0x7D], then length 0.
        assert_eq!(bytes, Vec::from([0xA2u8, 0x7D, 0x00]));
    }

    /// A `dish_get_obstruction_map` response round-trips with its grid intact
    /// and lands in the 2008 arm.
    #[test]
    fn dish_get_obstruction_map_round_trips() {
        let original = Response {
            api_version: 42,
            body: Some(response::Body::DishGetObstructionMap(
                DishGetObstructionMapResponse {
                    num_rows: 2,
                    num_cols: 3,
                    snr: alloc::vec![-1.0, 1.0, 0.5, 1.0, -1.0, 0.0],
                    max_theta_deg: 80.0,
                    map_reference_frame: 2,
                },
            )),
            ..Response::default()
        };
        let bytes = original.encode_to_vec();
        assert_eq!(Response::decode(&*bytes).expect("decode"), original);
    }

    /// A `dish_get_diagnostics` response round-trips, including nested alerts /
    /// alignment / location, and lands in the 6001 arm.
    #[test]
    fn dish_get_diagnostics_round_trips() {
        let original = Response {
            api_version: 42,
            body: Some(response::Body::DishGetDiagnostics(
                DishGetDiagnosticsResponse {
                    id: "ut-test".to_string(),
                    hardware_version: "mini1_prod2".to_string(),
                    software_version: "2026.06.15.mr81291".to_string(),
                    utc_offset_s: 3600,
                    alerts: Some(DiagnosticsAlerts {
                        slow_ethernet_speeds: true,
                        ..DiagnosticsAlerts::default()
                    }),
                    disablement_code: 1,
                    hardware_self_test: 1,
                    location: Some(DiagnosticsLocation {
                        enabled: false,
                        ..DiagnosticsLocation::default()
                    }),
                    alignment_stats: Some(DiagnosticsAlignmentStats {
                        boresight_azimuth_deg: -166.0,
                        boresight_elevation_deg: 80.9,
                        desired_boresight_azimuth_deg: 175.0,
                        desired_boresight_elevation_deg: 70.0,
                    }),
                    stowed: false,
                    overage_rate_limited: false,
                },
            )),
            ..Response::default()
        };
        let bytes = original.encode_to_vec();
        assert_eq!(Response::decode(&*bytes).expect("decode"), original);
    }

    /// A `wifi_get_status` response (router endpoint) round-trips and lands in
    /// the 3004 arm.
    #[test]
    fn wifi_get_status_round_trips() {
        let original = Response {
            api_version: 128,
            body: Some(response::Body::WifiGetStatus(WifiGetStatusResponse {
                device_info: Some(DeviceInfo {
                    id: "Router-0100".to_string(),
                    ..DeviceInfo::default()
                }),
                device_state: Some(DeviceState { uptime_s: 4680 }),
                ipv4_wan_address: "100.127.191.243/10".to_string(),
                ping_latency_ms: 20.89,
                dish_ping_latency_ms: 0.51,
                pop_ping_latency_ms: 17.52,
                ipv6_wan_addresses: alloc::vec!["2a0d:3341::1/64".to_string()],
                dish_id: "ut40909c84".to_string(),
                hops_from_controller: 0,
                ..WifiGetStatusResponse::default()
            })),
            ..Response::default()
        };
        let bytes = original.encode_to_vec();
        assert_eq!(Response::decode(&*bytes).expect("decode"), original);
    }

    /// A `wifi_get_history` response round-trips, including a 1 Hz ring buffer
    /// and a 15-second-bucket array, and lands in the 3006 arm.
    #[test]
    fn wifi_get_history_round_trips() {
        let original = Response {
            api_version: 128,
            body: Some(response::Body::WifiGetHistory(WifiGetHistoryResponse {
                current: 4756,
                current_index_15s: 250,
                ping_latency_ms: alloc::vec![20.1, 25.3, f32::from_bits(0)],
                ping_drop_rate: alloc::vec![0.0, 1.0, 0.0],
                pop_ipv4_ping_drop_rate_last_15s: alloc::vec![0.0, 0.2],
                cloudflare_ipv4_ping_drop_rate_last_15s: alloc::vec![0.5],
                ..WifiGetHistoryResponse::default()
            })),
            ..Response::default()
        };
        let bytes = original.encode_to_vec();
        assert_eq!(Response::decode(&*bytes).expect("decode"), original);
    }

    /// The `dish_get_obstruction_map` arm sits at tag 2008 — verify the
    /// envelope key bytes so a renumber can't silently decode into a sibling.
    #[test]
    fn dish_get_obstruction_map_arm_tag_is_2008() {
        let resp = Response {
            body: Some(response::Body::DishGetObstructionMap(
                DishGetObstructionMapResponse::default(),
            )),
            ..Response::default()
        };
        // (2008 << 3) | 2 = 16066 → varint [0xC2, 0x7D], then length 0.
        assert_eq!(resp.encode_to_vec(), Vec::from([0xC2u8, 0x7D, 0x00]));
    }

    /// An empty buffer decodes to a default `Response` (all fields absent) —
    /// prost treats a zero-length message as all-defaults, not an error.
    #[test]
    fn empty_buffer_is_default() {
        let decoded = Response::decode(&[][..]).expect("empty decodes");
        assert_eq!(decoded, Response::default());
        assert!(decoded.body.is_none());
    }
}
