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
    #[prost(oneof = "response::Body", tags = "2004, 2006")]
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
        /// `dish_get_status = 2004` — the reply to a `get_status` request.
        #[prost(message, tag = "2004")]
        DishGetStatus(super::DishGetStatusResponse),
        /// `dish_get_history = 2006` — the reply to a `get_history` request.
        #[prost(message, tag = "2006")]
        DishGetHistory(super::DishGetHistoryResponse),
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

    /// An empty buffer decodes to a default `Response` (all fields absent) —
    /// prost treats a zero-length message as all-defaults, not an error.
    #[test]
    fn empty_buffer_is_default() {
        let decoded = Response::decode(&[][..]).expect("empty decodes");
        assert_eq!(decoded, Response::default());
        assert!(decoded.body.is_none());
    }
}
