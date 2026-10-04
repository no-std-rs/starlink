//! Starlink Device API command-line client.
//!
//! Talks to the dish's `SpaceX.API.Device.Device/Handle` unary RPC over a
//! hand-rolled plaintext HTTP/2 client (see [`h2`]).  No `tokio`, `hyper`, or
//! `tonic`: the binary is `std` today but everything it leans on has a
//! `no_std` path so the embedded port stays open.
//!
//! Commands fall into two groups by endpoint.  Dish commands (default
//! `192.168.100.1:9200`): `status`, `align`, `history`, `devices`,
//! `obstruction-map` (draws the sky-view grid), `diagnostics`, `device-info`,
//! `location`, `gnss`, `dish-config`, `dish-context`, `time`, `ping`,
//! `ping-host`, `connections`, `interfaces`, `transceiver`,
//! `transceiver-telemetry`.  Router commands (default `192.168.1.1:9000`):
//! `clients`, `client-history`, `wifi-status`, `wifi-history`, `ping-metrics`,
//! `radio-stats`.  Run `starlink help` for the full list.
//!
//! Many arms exist in the schema but are `Unimplemented` or `PermissionDenied`
//! on a given firmware/hardware; the dish reports which through the gRPC
//! trailer `grpc-status`, which [`h2`] surfaces as a clean error.
//!
//! ```text
//! starlink status            [--addr HOST:PORT]
//! starlink obstruction-map   [--addr HOST:PORT] [--width N]
//! starlink wifi-status       [--addr HOST:PORT]
//! ```

// These fire throughout the human-readable formatting below (seconds as
// f32→u64, sample counts as f64, ns as f64, …).  Every cast here is for
// display rounding, never for correctness, so allow them crate-wide rather
// than peppering call sites.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod h2;

use core::future::Future;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use prost::Message;
use starlink_core::{frame, unframe, Transport};
use starlink_proto::device::{Request, HANDLE_PATH};
use starlink_proto::response::{
    attitude_estimation_state_name, disablement_code_name, dish_state_name, gnss_system_name,
    has_actuators_name, outage_cause_name, position_source_name, response::Body, router_role_name,
    test_result_name, transceiver_state_name, wifi_interface_name, wifi_role_name,
    DishGetConfigResponse, DishGetDiagnosticsResponse, DishGetHistoryResponse,
    DishGetObstructionMapResponse, DishGetStatusResponse, GetDeviceInfoResponse,
    GetNetworkInterfacesResponse, GetRadioStatsResponse, PingHostResponse, Response,
    WifiGetClientHistoryResponse, WifiGetClientsResponse, WifiGetPingMetricsResponse,
};

/// Default dish endpoint on the standard Starlink management subnet.
const DISH_ADDR: &str = "192.168.100.1:9200";
/// Default router endpoint (the LAN gateway) for Wi-Fi/client queries.
const ROUTER_ADDR: &str = "192.168.1.1:9000";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let rest = &args[args.len().min(2)..];
    let cmd = args.get(1).map(String::as_str);

    // Router-served commands default to the LAN gateway; the rest to the dish.
    let default_addr = if matches!(
        cmd,
        Some(
            "clients"
                | "client-history"
                | "wifi-status"
                | "wifi-history"
                | "ping-metrics"
                | "radio-stats"
        )
    ) {
        ROUTER_ADDR
    } else {
        DISH_ADDR
    };
    let addr = parse_opt(rest, "--addr").unwrap_or_else(|| default_addr.to_string());

    let result = match cmd {
        Some("status") => cmd_status(&addr),
        Some("align") => cmd_align(&addr),
        Some("history") => {
            let samples = parse_opt(rest, "--samples")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(900);
            cmd_history(&addr, samples)
        }
        Some("devices" | "nodes") => cmd_devices(&addr),
        Some("clients") => cmd_clients(&addr),
        Some("client-history") => {
            let samples = parse_opt(rest, "--samples")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(900);
            cmd_client_history(&addr, samples)
        }
        Some("obstruction-map" | "obstructions") => {
            let width = parse_opt(rest, "--width")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(60);
            cmd_obstruction_map(&addr, width)
        }
        Some("diagnostics" | "diag") => cmd_diagnostics(&addr),
        Some("device-info") => cmd_device_info(&addr),
        Some("location") => cmd_location(&addr),
        Some("gnss") => cmd_gnss(&addr),
        Some("dish-config") => cmd_dish_config(&addr),
        Some("dish-context") => cmd_dish_context(&addr),
        Some("time") => cmd_time(&addr),
        Some("ping") => cmd_ping(&addr),
        Some("ping-host") => match rest.iter().find(|a| !a.starts_with('-')) {
            Some(host) => cmd_ping_host(&addr, host),
            None => Err("usage: starlink ping-host <HOST> [--addr HOST:PORT]".to_string()),
        },
        Some("connections") => cmd_connections(&addr),
        Some("interfaces") => cmd_interfaces(&addr),
        Some("transceiver") => cmd_transceiver(&addr),
        Some("transceiver-telemetry") => cmd_transceiver_telemetry(&addr),
        Some("wifi-status") => cmd_wifi_status(&addr),
        Some("wifi-history") => {
            let samples = parse_opt(rest, "--samples")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(900);
            cmd_wifi_history(&addr, samples)
        }
        Some("ping-metrics") => cmd_ping_metrics(&addr),
        Some("radio-stats") => cmd_radio_stats(&addr),
        Some("-h" | "--help" | "help") => {
            print_usage();
            return ExitCode::SUCCESS;
        }
        _ => {
            print_usage();
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("starlink: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage: starlink <command> [--addr HOST:PORT]");
    eprintln!();
    eprintln!("commands (dish endpoint):");
    eprintln!("  status              identity, link, obstruction, alignment, nodes");
    eprintln!("  align               how to aim the dish (current vs desired boresight)");
    eprintln!("  history [--samples N]   latency / throughput / power / outage history");
    eprintln!("  devices             downstream routers / mesh nodes");
    eprintln!("  obstruction-map [--width N]   draw the sky-view obstruction map");
    eprintln!("  diagnostics         alerts, self-test, disablement, alignment");
    eprintln!("  device-info         hardware / software identity");
    eprintln!("  location            GPS position (needs owner opt-in)");
    eprintln!("  gnss                raw per-satellite GNSS measurements");
    eprintln!("  dish-config         configured power-save / snow-melt / level mode");
    eprintln!("  dish-context        cell / PoP / beam / obstruction context");
    eprintln!("  time                dish wall-clock time");
    eprintln!("  ping                PoP / gateway ping results");
    eprintln!("  ping-host <HOST>    ask the dish to ping a host");
    eprintln!("  connections         backend service connections");
    eprintln!("  interfaces          network interface counters");
    eprintln!("  transceiver         transceiver modulator / temp state");
    eprintln!("  transceiver-telemetry   antenna attitude / SNR / satellite ids");
    eprintln!();
    eprintln!("commands (router endpoint):");
    eprintln!("  clients             attached Wi-Fi / Ethernet clients and their stats");
    eprintln!("  client-history [--samples N]   per-client download/upload throughput history");
    eprintln!("  wifi-status         WAN address, dish/PoP/internet ping, alerts");
    eprintln!("  wifi-history [--samples N]   router ping drop/latency + multi-target history");
    eprintln!("  ping-metrics        internet ping metrics (needs auth)");
    eprintln!("  radio-stats         per-band Wi-Fi radio thermal / antenna stats");
    eprintln!();
    eprintln!("note: many arms are Unimplemented or PermissionDenied depending on");
    eprintln!("      firmware / hardware; the dish reports which via grpc-status.");
    eprintln!();
    eprintln!("default addr {DISH_ADDR} (dish), {ROUTER_ADDR} (router commands)");
}

/// Pull `--name VALUE` out of `args`, if present.
fn parse_opt(args: &[String], name: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == name {
            return it.next().cloned();
        }
    }
    None
}

/// Send one `Request` and decode the `Response` envelope.
fn fetch(addr: &str, request: &Request) -> Result<Response, String> {
    let body = frame(&request.encode_to_vec());
    let mut transport = h2::H2Transport::connect(addr).map_err(|e| e.to_string())?;
    let framed = block_on(transport.unary(HANDLE_PATH, &body)).map_err(|e| e.to_string())?;
    let payload = unframe(&framed).map_err(|e| e.to_string())?;
    Response::decode(payload).map_err(|e| format!("protobuf decode failed: {e}"))
}

/// Pull the `dish_get_status` arm out of a response, or explain its absence.
fn dish_status(response: &Response) -> Result<&DishGetStatusResponse, String> {
    match &response.body {
        Some(Body::DishGetStatus(s)) => Ok(s),
        _ => Err(status_or(
            "response carried no dish_get_status arm",
            response,
        )),
    }
}

/// Pull the `dish_get_history` arm out of a response, or explain its absence.
fn dish_history(response: &Response) -> Result<&DishGetHistoryResponse, String> {
    match &response.body {
        Some(Body::DishGetHistory(h)) => Ok(h),
        _ => Err(status_or(
            "response carried no dish_get_history arm",
            response,
        )),
    }
}

/// Pull the `wifi_get_clients` arm out of a response, or explain its absence.
fn wifi_clients(response: &Response) -> Result<&WifiGetClientsResponse, String> {
    match &response.body {
        Some(Body::WifiGetClients(c)) => Ok(c),
        _ => Err(status_or(
            "response carried no wifi_get_clients arm (is --addr the router?)",
            response,
        )),
    }
}

/// Pull the `wifi_get_client_history` arm out of a response.
fn wifi_client_history(response: &Response) -> Result<&WifiGetClientHistoryResponse, String> {
    match &response.body {
        Some(Body::WifiGetClientHistory(h)) => Ok(h),
        _ => Err(status_or(
            "response carried no wifi_get_client_history arm",
            response,
        )),
    }
}

/// Prefer the dish's own error message (if it set a non-zero status) over our
/// generic "missing arm" fallback.
fn status_or(fallback: &str, response: &Response) -> String {
    match &response.status {
        Some(s) if s.code != 0 => format!("dish error {}: {}", s.code, s.message),
        _ => fallback.to_string(),
    }
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_status())?;
    let s = dish_status(&response)?;

    println!("api_version:      {}", response.api_version);
    if let Some(di) = &s.device_info {
        println!("device id:        {}", di.id);
        println!("hardware:         {}", di.hardware_version);
        println!("software:         {}", di.software_version);
        println!("country:          {}", di.country_code);
        println!("bootcount:        {}", di.bootcount);
    }
    if let Some(ds) = &s.device_state {
        println!("uptime:           {}", human_duration(ds.uptime_s));
    }

    println!(
        "downlink:         {:.2} Mbps",
        mbps(s.downlink_throughput_bps)
    );
    println!(
        "uplink:           {:.2} Mbps",
        mbps(s.uplink_throughput_bps)
    );
    println!("pop ping latency: {:.2} ms", s.pop_ping_latency_ms);
    println!("pop ping drop:    {:.2}%", s.pop_ping_drop_rate * 100.0);
    println!("eth speed:        {} Mbps", s.eth_speed_mbps);

    if let Some(o) = &s.obstruction_stats {
        println!(
            "obstruction:      {:.3}% (currently {}; obstructed {} of {:.0}s valid; {} patches)",
            o.fraction_obstructed * 100.0,
            if o.currently_obstructed { "YES" } else { "no" },
            human_duration(o.time_obstructed as u64),
            o.valid_s,
            o.patches_valid,
        );
    }

    if let Some(a) = &s.alignment_stats {
        println!(
            "alignment:        az {:.1}° el {:.1}° (tilt {:.1}°), {} ±{:.2}°",
            a.boresight_azimuth_deg,
            a.boresight_elevation_deg,
            a.tilt_angle_deg,
            attitude_estimation_state_name(a.attitude_estimation_state),
            a.attitude_uncertainty_deg,
        );
    }

    if let Some(g) = &s.gps_stats {
        println!(
            "gps:              valid={} sats={}",
            g.gps_valid, g.gps_sats
        );
    }

    if !s.connected_routers.is_empty() {
        println!("mesh nodes:       {}", s.connected_routers.len());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// align
// ---------------------------------------------------------------------------

fn cmd_align(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_status())?;
    let s = dish_status(&response)?;
    let a = s
        .alignment_stats
        .as_ref()
        .ok_or("status carried no alignment_stats")?;

    println!(
        "attitude filter:  {}",
        attitude_estimation_state_name(a.attitude_estimation_state)
    );
    println!("attitude uncert.: ±{:.2}°", a.attitude_uncertainty_deg);
    println!("actuators:        {}", has_actuators_name(a.has_actuators));
    println!();
    println!(
        "current boresight: az {:7.2}°  el {:6.2}°  (tilt {:.2}°)",
        a.boresight_azimuth_deg, a.boresight_elevation_deg, a.tilt_angle_deg
    );
    println!(
        "desired boresight: az {:7.2}°  el {:6.2}°",
        a.desired_boresight_azimuth_deg, a.desired_boresight_elevation_deg
    );

    let d_az = norm_deg(a.desired_boresight_azimuth_deg - a.boresight_azimuth_deg);
    let d_el = a.desired_boresight_elevation_deg - a.boresight_elevation_deg;
    println!();
    println!("to align, move the dish:");
    if d_az.abs() < 1.0 {
        println!("  azimuth:   on target ({d_az:+.1}°)");
    } else {
        let dir = if d_az > 0.0 {
            "clockwise (right)"
        } else {
            "counter-clockwise (left)"
        };
        println!("  azimuth:   rotate {dir} by {:.1}°", d_az.abs());
    }
    if d_el.abs() < 1.0 {
        println!("  elevation: on target ({d_el:+.1}°)");
    } else {
        let dir = if d_el > 0.0 { "up" } else { "down" };
        println!("  elevation: tilt {dir} by {:.1}°", d_el.abs());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// history
// ---------------------------------------------------------------------------

fn cmd_history(addr: &str, samples: usize) -> Result<(), String> {
    let response = fetch(addr, &Request::get_history())?;
    let h = dish_history(&response)?;

    let n = h.pop_ping_latency_ms.len();
    let window = samples.min(n);
    println!(
        "history window:   {window} of {n} samples (~{} at 1 Hz), index {}",
        human_duration(window as u64),
        h.current
    );
    println!();

    report(
        "latency (ms)",
        &recent(&h.pop_ping_latency_ms, h.current, window),
        2,
    );
    report_pct(
        "ping drop",
        &recent(&h.pop_ping_drop_rate, h.current, window),
    );
    report(
        "downlink (Mbps)",
        &recent(&h.downlink_throughput_bps, h.current, window)
            .iter()
            .map(|b| mbps(*b))
            .collect::<Vec<_>>(),
        1,
    );
    report(
        "uplink (Mbps)",
        &recent(&h.uplink_throughput_bps, h.current, window)
            .iter()
            .map(|b| mbps(*b))
            .collect::<Vec<_>>(),
        2,
    );
    report("power draw (W)", &recent(&h.power_in, h.current, window), 1);

    println!();
    if h.outages.is_empty() {
        println!("outages:          none in buffer");
    } else {
        // NB: DishOutage.start_timestamp_ns is on the dish's monotonic clock,
        // not the Unix epoch, so we report duration only (it is meaningful)
        // and skip a wall-clock "ago".
        println!("outages:          {} in buffer", h.outages.len());
        for o in &h.outages {
            println!(
                "  - {:<16} lasted {:>6.1}s{}",
                outage_cause_name(o.cause),
                o.duration_ns as f64 / 1e9,
                if o.did_switch { " (sat handover)" } else { "" },
            );
        }
    }
    Ok(())
}

/// Print min / mean / max / p95 for a metric, to `decimals` places.
fn report(label: &str, v: &[f32], decimals: usize) {
    match stats(v) {
        Some(st) => println!(
            "{label:<17} min {:.*}  avg {:.*}  max {:.*}  p95 {:.*}",
            decimals, st.min, decimals, st.mean, decimals, st.max, decimals, st.p95
        ),
        None => println!("{label:<17} (no data)"),
    }
}

/// Like [`report`] but renders a `0.0..=1.0` fraction as a percentage.
fn report_pct(label: &str, v: &[f32]) {
    match stats(v) {
        Some(st) => println!(
            "{label:<17} avg {:.2}%  max {:.2}%",
            st.mean * 100.0,
            st.max * 100.0
        ),
        None => println!("{label:<17} (no data)"),
    }
}

// ---------------------------------------------------------------------------
// devices
// ---------------------------------------------------------------------------

fn cmd_devices(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_status())?;
    let s = dish_status(&response)?;

    if s.connected_routers.is_empty() && s.downstream_routers.is_empty() {
        println!("no downstream routers / mesh nodes reported");
        return Ok(());
    }

    println!("downstream routers / mesh nodes:");
    let now_ns = now_unix_ns();
    for id in &s.connected_routers {
        match s.downstream_routers.get(id) {
            Some(info) => {
                let ago = (now_ns - info.last_seen) as f64 / 1e9;
                println!(
                    "  - {id}  role {}  (last seen {} ago)",
                    router_role_name(info.role),
                    human_duration(ago.max(0.0) as u64),
                );
            }
            None => println!("  - {id}"),
        }
    }
    // Any routers present in the map but not in connected_routers.
    for (id, info) in &s.downstream_routers {
        if !s.connected_routers.contains(id) {
            println!(
                "  - {id}  role {} (not currently connected)",
                router_role_name(info.role)
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// clients (router endpoint)
// ---------------------------------------------------------------------------

fn cmd_clients(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::wifi_get_clients())?;
    let c = wifi_clients(&response)?;

    println!("attached clients: {}", c.clients.len());
    for cl in &c.clients {
        let name = if cl.name.is_empty() {
            "(unnamed)"
        } else {
            cl.name.as_str()
        };
        let ip = if cl.ip_address.is_empty() {
            "-"
        } else {
            cl.ip_address.as_str()
        };

        println!();
        println!(
            "  {name:<22} {:<8} {:<11} {ip}",
            wifi_interface_name(cl.iface),
            wifi_role_name(cl.role),
        );

        let mut parts: Vec<String> = Vec::new();
        if !cl.mac_address.is_empty() {
            parts.push(cl.mac_address.clone());
        }
        // Signal/SNR are only meaningful for wireless clients.
        if cl.signal_strength != 0.0 || cl.snr != 0.0 {
            parts.push(format!("{:.0} dBm / snr {:.0}", cl.signal_strength, cl.snr));
        }
        let rx_rate = cl.rx_stats.as_ref().map_or(0, |r| r.rate_mbps);
        let tx_rate = cl.tx_stats.as_ref().map_or(0, |t| t.rate_mbps);
        if rx_rate != 0 || tx_rate != 0 {
            parts.push(format!("rate ↓{rx_rate} ↑{tx_rate} Mbps"));
        }
        let rx_bytes = cl.rx_stats.as_ref().map_or(0, |r| r.bytes);
        let tx_bytes = cl.tx_stats.as_ref().map_or(0, |t| t.bytes);
        if rx_bytes != 0 || tx_bytes != 0 {
            parts.push(format!(
                "data ↓{} ↑{}",
                human_bytes(rx_bytes),
                human_bytes(tx_bytes)
            ));
        }
        if cl.associated_time_s != 0 {
            parts.push(format!(
                "up {}",
                human_duration(u64::from(cl.associated_time_s))
            ));
        }
        if cl.hops_from_controller != 0 {
            parts.push(format!(
                "{} hop(s) from controller",
                cl.hops_from_controller
            ));
        }
        if !parts.is_empty() {
            println!("    {}", parts.join("  "));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// client-history (router endpoint)
// ---------------------------------------------------------------------------

fn cmd_client_history(addr: &str, samples: usize) -> Result<(), String> {
    // First enumerate clients to learn their ids and names...
    let list = fetch(addr, &Request::wifi_get_clients())?;
    let clients = wifi_clients(&list)?;

    let mut any = false;
    for cl in &clients.clients {
        // Clients with no id (e.g. the controller) have no per-client history.
        if cl.client_id == 0 {
            continue;
        }
        let name = if cl.name.is_empty() {
            cl.mac_address.as_str()
        } else {
            cl.name.as_str()
        };

        // ...then pull each one's history on its own connection.
        let resp = fetch(addr, &Request::wifi_get_client_history(cl.client_id))?;
        let h = wifi_client_history(&resp)?;
        any = true;

        let n = h.tx_throughput_mbps.len().max(h.rx_throughput_mbps.len());
        let window = if n == 0 { 0 } else { samples.min(n) };
        println!();
        println!(
            "{name}  [{}]  — {window} of {n} samples (~{})",
            wifi_interface_name(cl.iface),
            human_duration(window as u64),
        );
        if n == 0 {
            println!("    (no history samples)");
            continue;
        }
        report(
            "  download (Mbps)",
            &recent(&h.tx_throughput_mbps, h.current, window),
            3,
        );
        report(
            "  upload (Mbps)",
            &recent(&h.rx_throughput_mbps, h.current, window),
            3,
        );
        if !h.rx_rate_mbps.is_empty() {
            report(
                "  rx PHY (Mbps)",
                &recent(&h.rx_rate_mbps, h.current, window),
                0,
            );
        }
    }

    if !any {
        println!("no clients with per-client history");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// obstruction-map (dish) — draw the sky-view obstruction grid
// ---------------------------------------------------------------------------

fn obstruction_map(response: &Response) -> Result<&DishGetObstructionMapResponse, String> {
    match &response.body {
        Some(Body::DishGetObstructionMap(m)) => Ok(m),
        _ => Err(status_or("response carried no dish_get_obstruction_map arm", response)),
    }
}

fn cmd_obstruction_map(addr: &str, width: usize) -> Result<(), String> {
    let response = fetch(addr, &Request::dish_get_obstruction_map())?;
    let m = obstruction_map(&response)?;

    let rows = m.num_rows as usize;
    let cols = m.num_cols as usize;
    if rows == 0 || cols == 0 || m.snr.len() < rows * cols {
        return Err(format!(
            "malformed obstruction map ({rows}x{cols}, {} values)",
            m.snr.len()
        ));
    }

    // A cell is "observed" when its value is >= 0; -1 marks outside the field
    // of view / not-yet-mapped sky.  For observed cells, value 1.0 is clear
    // sky and lower values are progressively more obstructed.
    let (mut observed, mut obstructed) = (0usize, 0usize);
    for &v in &m.snr {
        if v >= 0.0 {
            observed += 1;
            if v < 0.99 {
                obstructed += 1;
            }
        }
    }

    println!(
        "obstruction map:  {rows}x{cols}, {}° radius from boresight ({})",
        m.max_theta_deg as i32,
        ref_frame_name(m.map_reference_frame),
    );
    println!(
        "observed sky:     {observed} cells, {obstructed} obstructed ({:.2}%)",
        if observed == 0 {
            0.0
        } else {
            obstructed as f64 / observed as f64 * 100.0
        }
    );
    println!();

    draw_obstruction_map(&m.snr, rows, cols, width.clamp(20, cols.max(20)));

    println!();
    println!("legend: ` ` outside FOV   `·` clear   `░▒▓` partial   `█` obstructed");
    Ok(())
}

/// Map an `ObstructionMapReferenceFrame` enum value to its name.
fn ref_frame_name(v: i32) -> &'static str {
    match v {
        1 => "FRAME_EARTH (compass-aligned)",
        2 => "FRAME_UT (dish-aligned)",
        _ => "FRAME_UNKNOWN",
    }
}

/// Downsample the `rows×cols` obstruction grid to `out_w` columns (and a
/// height that keeps a roughly circular aspect, since terminal cells are about
/// twice as tall as wide) and print it with a shaded ramp.
fn draw_obstruction_map(snr: &[f32], rows: usize, cols: usize, out_w: usize) {
    let scale = cols as f64 / out_w as f64;
    // Halve the vertical resolution to correct for ~2:1 character aspect.
    let out_h = ((rows as f64 / scale) * 0.5).round().max(1.0) as usize;

    let mut lines: Vec<String> = Vec::with_capacity(out_h);
    for oy in 0..out_h {
        let mut line = String::with_capacity(out_w);
        for ox in 0..out_w {
            // The input block this output cell covers.
            let x0 = (ox as f64 * scale) as usize;
            let x1 = (((ox + 1) as f64 * scale) as usize).max(x0 + 1).min(cols);
            let y0 = (oy as f64 * scale * 2.0) as usize;
            let y1 = (((oy + 1) as f64 * scale * 2.0) as usize).max(y0 + 1).min(rows);

            let (mut sum, mut seen) = (0.0f64, 0usize);
            for y in y0..y1 {
                for x in x0..x1 {
                    let v = snr[y * cols + x];
                    if v >= 0.0 {
                        sum += f64::from(v);
                        seen += 1;
                    }
                }
            }
            line.push(if seen == 0 {
                ' '
            } else {
                // obstruction fraction: 0 clear .. 1 fully obstructed
                shade(1.0 - sum / seen as f64)
            });
        }
        // Trim trailing blanks so the disk isn't boxed by whitespace.
        lines.push(line.trim_end().to_string());
    }
    // Drop fully-blank rows at the top and bottom so the disk sits snug.
    let first = lines.iter().position(|l| !l.is_empty()).unwrap_or(0);
    let last = lines.iter().rposition(|l| !l.is_empty()).unwrap_or(0);
    for line in &lines[first..=last] {
        println!("{line}");
    }
}

/// Pick a glyph for an obstruction fraction in `0.0..=1.0`.
fn shade(f: f64) -> char {
    match f {
        x if x < 0.02 => '·',
        x if x < 0.25 => '░',
        x if x < 0.50 => '▒',
        x if x < 0.80 => '▓',
        _ => '█',
    }
}

// ---------------------------------------------------------------------------
// diagnostics (dish)
// ---------------------------------------------------------------------------

fn dish_diagnostics(response: &Response) -> Result<&DishGetDiagnosticsResponse, String> {
    match &response.body {
        Some(Body::DishGetDiagnostics(d)) => Ok(d),
        _ => Err(status_or("response carried no dish_get_diagnostics arm", response)),
    }
}

fn cmd_diagnostics(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_diagnostics())?;
    let d = dish_diagnostics(&response)?;

    println!("device id:        {}", d.id);
    println!("hardware:         {}", d.hardware_version);
    println!("software:         {}", d.software_version);
    println!("disablement:      {}", disablement_code_name(d.disablement_code));
    println!("hardware self-test: {}", test_result_name(d.hardware_self_test));
    println!("stowed:           {}", d.stowed);
    println!("overage limited:  {}", d.overage_rate_limited);

    if let Some(a) = &d.alignment_stats {
        println!(
            "alignment:        current az {:.1}° el {:.1}°  desired az {:.1}° el {:.1}°",
            a.boresight_azimuth_deg,
            a.boresight_elevation_deg,
            a.desired_boresight_azimuth_deg,
            a.desired_boresight_elevation_deg,
        );
    }
    if let Some(l) = &d.location {
        if l.enabled {
            println!(
                "location:         {:.5}, {:.5}  alt {:.0} m",
                l.latitude, l.longitude, l.altitude_meters
            );
        } else {
            println!("location:         (sharing disabled)");
        }
    }

    let alerts = active_diag_alerts(d.alerts.as_ref());
    if alerts.is_empty() {
        println!("alerts:           none");
    } else {
        println!("alerts:           {}", alerts.join(", "));
    }
    Ok(())
}

/// Collect the names of the diagnostics alert flags that are set.
fn active_diag_alerts(
    a: Option<&starlink_proto::response::DiagnosticsAlerts>,
) -> Vec<&'static str> {
    let Some(a) = a else { return Vec::new() };
    let mut out = Vec::new();
    for (set, name) in [
        (a.dish_is_heating, "dish_is_heating"),
        (a.dish_thermal_throttle, "dish_thermal_throttle"),
        (a.dish_thermal_shutdown, "dish_thermal_shutdown"),
        (a.power_supply_thermal_throttle, "power_supply_thermal_throttle"),
        (a.motors_stuck, "motors_stuck"),
        (a.mast_not_near_vertical, "mast_not_near_vertical"),
        (a.slow_ethernet_speeds, "slow_ethernet_speeds"),
        (a.software_install_pending, "software_install_pending"),
        (a.obstructed, "obstructed"),
    ] {
        if set {
            out.push(name);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// device-info (dish)
// ---------------------------------------------------------------------------

fn cmd_device_info(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_device_info())?;
    let Some(Body::GetDeviceInfo(GetDeviceInfoResponse { device_info: Some(di) })) = &response.body else {
        return Err(status_or("response carried no get_device_info arm", &response));
    };
    println!("device id:        {}", di.id);
    println!("hardware:         {}", di.hardware_version);
    println!("software:         {}", di.software_version);
    println!("build id:         {}", di.build_id);
    println!("country:          {}", di.country_code);
    println!("utc offset:       {}s", di.utc_offset_s);
    println!("bootcount:        {}", di.bootcount);
    println!("generation:       {}", di.generation_number);
    Ok(())
}

// ---------------------------------------------------------------------------
// location (dish)
// ---------------------------------------------------------------------------

fn cmd_location(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_location())?;
    let Some(Body::GetLocation(l)) = &response.body else {
        return Err(status_or("response carried no get_location arm", &response));
    };
    if let Some(p) = &l.lla {
        println!("position:         {:.6}, {:.6}", p.lat, p.lon);
        println!("altitude:         {:.1} m", p.alt);
    }
    println!("source:           {}", position_source_name(l.source));
    println!("uncertainty:      ±{:.1} m", l.sigma_m);
    println!(
        "speed:            {:.2} m/s horiz, {:.2} m/s vert",
        l.horizontal_speed_mps, l.vertical_speed_mps
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// gnss (dish)
// ---------------------------------------------------------------------------

fn cmd_gnss(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_gnss_measurement())?;
    let Some(Body::GetGnssMeasurement(g)) = &response.body else {
        return Err(status_or("response carried no get_gnss_measurement arm", &response));
    };
    println!("device id:        {}", g.device_id);
    println!("satellites:       {}", g.measurements.len());
    for mz in &g.measurements {
        println!(
            "  {:<8} PRN {:>3}",
            gnss_system_name(mz.satellite_system),
            mz.prn
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// dish-config (dish)
// ---------------------------------------------------------------------------

fn cmd_dish_config(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::dish_get_config())?;
    let Some(Body::DishGetConfig(DishGetConfigResponse { dish_config: Some(c) })) = &response.body else {
        return Err(status_or("response carried no dish_get_config arm", &response));
    };
    println!("snow melt mode:   {}", snow_melt_name(c.snow_melt_mode));
    println!("level dish mode:  {}", level_dish_name(c.level_dish_mode));
    println!(
        "location request: {}",
        if c.location_request_mode == 1 { "LOCAL" } else { "NONE" }
    );
    println!("power save:       {}", if c.power_save_mode { "on" } else { "off" });
    if c.power_save_mode {
        println!(
            "  window:         {} for {} min",
            minutes_of_day(c.power_save_start_minutes),
            c.power_save_duration_minutes
        );
    }
    println!(
        "swupdate defer 3d: {}",
        if c.swupdate_three_day_deferral_enabled { "on" } else { "off" }
    );
    println!("swupdate reboot:  hour {}", c.swupdate_reboot_hour);
    println!("asset class:      {}", c.asset_class);
    Ok(())
}

fn snow_melt_name(v: i32) -> &'static str {
    match v {
        1 => "ALWAYS_ON",
        2 => "ALWAYS_OFF",
        _ => "AUTO",
    }
}

fn level_dish_name(v: i32) -> &'static str {
    match v {
        1 => "FORCE_LEVEL",
        _ => "TILT_LIKE_NORMAL",
    }
}

/// Render minutes-past-midnight as `HH:MM`.
fn minutes_of_day(m: u32) -> String {
    format!("{:02}:{:02}", m / 60 % 24, m % 60)
}

// ---------------------------------------------------------------------------
// dish-context (dish)
// ---------------------------------------------------------------------------

fn cmd_dish_context(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::dish_get_context())?;
    let Some(Body::DishGetContext(c)) = &response.body else {
        return Err(status_or("response carried no dish_get_context arm", &response));
    };
    println!("cell id:          {}", c.cell_id);
    println!("pop rack id:      {}", c.pop_rack_id);
    println!("initial sat:      {}", c.initial_satellite_id);
    println!("initial gateway:  {}", c.initial_gateway_id);
    println!("on backup beam:   {}", c.on_backup_beam);
    println!("slot ends in:     {:.1}s", c.seconds_to_slot_end);
    println!(
        "obstruction:      {:.3}% ({:.0}s valid, {:.0}s obstructed)",
        c.obstruction_fraction * 100.0,
        c.obstruction_valid_s,
        c.obstruction_time,
    );
    println!(
        "pop ping (15s):   {:.2} ms, {:.2}% drop",
        c.pop_ping_latency_ms_15s_mean,
        c.pop_ping_drop_rate_15s_mean * 100.0,
    );
    println!("ku mac active:    {:.1}%", c.ku_mac_active_ratio * 100.0);
    Ok(())
}

// ---------------------------------------------------------------------------
// time (dish)
// ---------------------------------------------------------------------------

fn cmd_time(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_time())?;
    let Some(Body::GetTime(t)) = &response.body else {
        return Err(status_or("response carried no time arm", &response));
    };
    let secs = t.unix_nano / 1_000_000_000;
    println!("dish time:        {} (unix {secs})", iso_utc(secs));
    let skew = secs - now_unix_ns() / 1_000_000_000;
    println!("skew vs host:     {skew:+}s");
    Ok(())
}

// ---------------------------------------------------------------------------
// ping / ping-host (dish)
// ---------------------------------------------------------------------------

fn cmd_ping(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_ping())?;
    let Some(Body::GetPing(p)) = &response.body else {
        return Err(status_or("response carried no get_ping arm", &response));
    };
    if p.results.is_empty() {
        println!("no ping results");
    }
    for (target, r) in &p.results {
        println!(
            "  {target:<16} {:.2} ms  {:.2}% drop",
            r.latency_ms,
            r.drop_rate * 100.0
        );
    }
    Ok(())
}

fn cmd_ping_host(addr: &str, host: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::ping_host(host))?;
    let Some(Body::PingHost(PingHostResponse { result: Some(r) })) = &response.body else {
        return Err(status_or("response carried no ping_host arm", &response));
    };
    println!(
        "{host}: {:.2} ms, {:.2}% drop",
        r.latency_ms,
        r.drop_rate * 100.0
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// connections (dish)
// ---------------------------------------------------------------------------

fn cmd_connections(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_connections())?;
    let Some(Body::GetConnections(c)) = &response.body else {
        return Err(status_or("response carried no get_connections arm", &response));
    };
    if c.services.is_empty() {
        println!("no service connections");
    }
    for (name, svc) in &c.services {
        println!(
            "  {name:<24} {}  ({}s since success)",
            svc.address, svc.seconds_since_success
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// interfaces (dish)
// ---------------------------------------------------------------------------

fn cmd_interfaces(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_network_interfaces())?;
    let Some(Body::GetNetworkInterfaces(GetNetworkInterfacesResponse {
        network_interfaces: ifs,
    })) = &response.body
    else {
        return Err(status_or("response carried no get_network_interfaces arm", &response));
    };
    if ifs.is_empty() {
        println!("no network interfaces");
    }
    for ni in ifs {
        println!();
        println!(
            "  {:<10} {}  {}",
            ni.name,
            if ni.up { "up" } else { "down" },
            ni.mac_address
        );
        if !ni.ipv4_addresses.is_empty() {
            println!("    ipv4 {}", ni.ipv4_addresses.join(", "));
        }
        if !ni.ipv6_addresses.is_empty() {
            println!("    ipv6 {}", ni.ipv6_addresses.join(", "));
        }
        let rx = ni.rx_stats.as_ref().map_or(0, |s| s.bytes);
        let tx = ni.tx_stats.as_ref().map_or(0, |s| s.bytes);
        println!("    rx {}  tx {}", human_bytes(rx), human_bytes(tx));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// transceiver / transceiver-telemetry (dish)
// ---------------------------------------------------------------------------

fn cmd_transceiver(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::transceiver_get_status())?;
    let Some(Body::TransceiverGetStatus(t)) = &response.body else {
        return Err(status_or("response carried no transceiver_get_status arm", &response));
    };
    println!("dish state:       {}", dish_state_name(t.state));
    println!("modulator:        {}", transceiver_state_name(t.mod_state));
    println!("demodulator:      {}", transceiver_state_name(t.demod_state));
    println!("tx:               {}", transceiver_state_name(t.tx_state));
    println!("rx:               {}", transceiver_state_name(t.rx_state));
    println!("modem asic temp:  {:.1} °C", t.modem_asic_temp);
    println!("tx if temp:       {:.1} °C", t.tx_if_temp);
    Ok(())
}

fn cmd_transceiver_telemetry(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::transceiver_get_telemetry())?;
    let Some(Body::TransceiverGetTelemetry(t)) = &response.body else {
        return Err(status_or("response carried no transceiver_get_telemetry arm", &response));
    };
    println!(
        "attitude:         pitch {:.1}° roll {:.1}° heading {:.1}°",
        t.antenna_pitch, t.antenna_roll, t.antenna_true_heading
    );
    println!("snr:              {:.1} dB (L1 avg {:.1} dB)", t.snr_db, t.l1_snr_avg_db);
    println!("wb rssi peak:     {:.1} dB", t.wb_rssi_peak_mag_db);
    println!("pop ping drop:    {:.2}%", t.pop_ping_drop_rate * 100.0);
    println!("cell id:          {}", t.current_cell_id);
    println!(
        "satellite:        serving {}  target {}",
        t.lmac_satellite_id, t.target_satellite_id
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// wifi-status (router)
// ---------------------------------------------------------------------------

fn cmd_wifi_status(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_status())?;
    let Some(Body::WifiGetStatus(w)) = &response.body else {
        return Err(status_or("response carried no wifi_get_status arm (is --addr the router?)", &response));
    };
    if let Some(di) = &w.device_info {
        println!("router id:        {}", di.id);
        println!("software:         {}", di.software_version);
    }
    if let Some(ds) = &w.device_state {
        println!("uptime:           {}", human_duration(ds.uptime_s));
    }
    println!("paired dish:      {}", w.dish_id);
    println!("wan ipv4:         {}", non_empty(&w.ipv4_wan_address));
    for a in &w.ipv6_wan_addresses {
        println!("wan ipv6:         {a}");
    }
    if w.secs_since_last_public_ipv4_change > 0.0 {
        println!(
            "ipv4 stable for:  {}",
            human_duration(w.secs_since_last_public_ipv4_change as u64)
        );
    }
    println!("no wan link:      {}", w.no_wan_link);
    println!("hops from ctrl:   {}", w.hops_from_controller);
    println!();
    println!("ping (drop / latency):");
    println!(
        "  → dish:         {:.2}% / {:.2} ms",
        w.dish_ping_drop_rate * 100.0,
        w.dish_ping_latency_ms
    );
    println!(
        "  → pop:          {:.2}% / {:.2} ms  (5m drop {:.2}%)",
        w.pop_ping_drop_rate * 100.0,
        w.pop_ping_latency_ms,
        w.pop_ping_drop_rate_5m * 100.0,
    );
    println!(
        "  → internet:     {:.2}% / {:.2} ms  (5m drop {:.2}%)",
        w.ping_drop_rate * 100.0,
        w.ping_latency_ms,
        w.ping_drop_rate_5m * 100.0,
    );

    let alerts = active_wifi_alerts(w.alerts.as_ref());
    println!();
    if alerts.is_empty() {
        println!("alerts:           none");
    } else {
        println!("alerts:           {}", alerts.join(", "));
    }
    Ok(())
}

fn active_wifi_alerts(a: Option<&starlink_proto::response::WifiAlerts>) -> Vec<&'static str> {
    let Some(a) = a else { return Vec::new() };
    let mut out = Vec::new();
    for (set, name) in [
        (a.thermal_throttle, "thermal_throttle"),
        (a.install_pending, "install_pending"),
        (a.lan_eth_slow_link_10, "lan_eth_slow_link_10"),
        (a.lan_eth_slow_link_100, "lan_eth_slow_link_100"),
        (a.wan_eth_poor_connection, "wan_eth_poor_connection"),
        (a.mesh_unreliable_backhaul, "mesh_unreliable_backhaul"),
        (a.high_cable_ping_drop_rate, "high_cable_ping_drop_rate"),
    ] {
        if set {
            out.push(name);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// wifi-history (router)
// ---------------------------------------------------------------------------

fn cmd_wifi_history(addr: &str, samples: usize) -> Result<(), String> {
    let response = fetch(addr, &Request::get_history())?;
    let Some(Body::WifiGetHistory(h)) = &response.body else {
        return Err(status_or("response carried no wifi_get_history arm (is --addr the router?)", &response));
    };

    let n = h.ping_latency_ms.len();
    let window = samples.min(n);
    println!(
        "history window:   {window} of {n} 1 Hz samples (~{}), index {}",
        human_duration(window as u64),
        h.current
    );
    println!();
    report(
        "internet latency (ms)",
        &recent(&h.ping_latency_ms, h.current, window),
        2,
    );
    report_pct(
        "internet drop",
        &recent(&h.ping_drop_rate, h.current, window),
    );

    // The *_last_15s arrays are coarse 15-second buckets on their own index.
    println!();
    println!("15-second-bucket drop rates (avg / max over last {} buckets):", h.pop_ipv4_ping_drop_rate_last_15s.len());
    report15("  pop ipv4", &h.pop_ipv4_ping_drop_rate_last_15s);
    report15("  pop ipv6", &h.pop_ipv6_ping_drop_rate_last_15s);
    report15("  google ipv4", &h.google_ipv4_ping_drop_rate_last_15s);
    report15("  google ipv6", &h.google_ipv6_ping_drop_rate_last_15s);
    report15("  cloudflare ipv4", &h.cloudflare_ipv4_ping_drop_rate_last_15s);
    report15("  cloudflare ipv6", &h.cloudflare_ipv6_ping_drop_rate_last_15s);
    Ok(())
}

/// Print avg/max for a 15-second-bucket drop-rate series (already a complete
/// ring; order does not matter for avg/max).
fn report15(label: &str, v: &[f32]) {
    match stats(v) {
        Some(st) => println!(
            "{label:<18} avg {:.2}%  max {:.2}%",
            st.mean * 100.0,
            st.max * 100.0
        ),
        None => println!("{label:<18} (no data)"),
    }
}

// ---------------------------------------------------------------------------
// ping-metrics (router)
// ---------------------------------------------------------------------------

fn cmd_ping_metrics(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::wifi_get_ping_metrics())?;
    let Some(Body::WifiGetPingMetrics(WifiGetPingMetricsResponse { internet: Some(m) })) = &response.body else {
        return Err(status_or("response carried no wifi_get_ping_metrics arm", &response));
    };
    println!("internet ping metrics:");
    println!(
        "  latency:        {:.2} ms (±{:.2})",
        m.latency_mean_ms, m.latency_stddev_ms
    );
    println!(
        "  drop rate:      now {:.2}%  5m {:.2}%  1h {:.2}%",
        m.drop_rate * 100.0,
        m.drop_rate_5m * 100.0,
        m.drop_rate_1h * 100.0,
    );
    println!("  last success:   {:.0}s ago", m.seconds_since_last_success);
    Ok(())
}

// ---------------------------------------------------------------------------
// radio-stats (router)
// ---------------------------------------------------------------------------

fn cmd_radio_stats(addr: &str) -> Result<(), String> {
    let response = fetch(addr, &Request::get_radio_stats())?;
    let Some(Body::GetRadioStats(GetRadioStatsResponse { radio_stats: rs })) = &response.body
    else {
        return Err(status_or("response carried no get_radio_stats arm", &response));
    };
    if rs.is_empty() {
        println!("no radio stats");
    }
    for r in rs {
        println!();
        println!("  band {}", r.band);
        if let Some(t) = &r.thermal_status {
            println!(
                "    thermal: level {}  {:.1} °C  power -{}%  duty {}%",
                t.level, t.temp2, t.power_reduction, t.duty_cycle
            );
        }
        if let Some(a) = &r.antenna_status {
            println!(
                "    rssi: {:.0} {:.0} {:.0} {:.0} dBm",
                a.rssi1, a.rssi2, a.rssi3, a.rssi4
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// numeric helpers
// ---------------------------------------------------------------------------

/// Summary statistics over a metric window, ignoring `NaN`.
struct Stats {
    min: f32,
    mean: f32,
    max: f32,
    p95: f32,
}

fn stats(v: &[f32]) -> Option<Stats> {
    let mut xs: Vec<f32> = v.iter().copied().filter(|x| !x.is_nan()).collect();
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let sum: f32 = xs.iter().sum();
    #[allow(clippy::cast_precision_loss)]
    let mean = sum / xs.len() as f32;
    let p95_idx = ((xs.len() as f64) * 0.95).ceil() as usize;
    let p95_idx = p95_idx.saturating_sub(1).min(xs.len() - 1);
    Some(Stats {
        min: xs[0],
        mean,
        max: xs[xs.len() - 1],
        p95: xs[p95_idx],
    })
}

/// The last `k` samples of a ring buffer, in chronological (oldest→newest)
/// order.  `current` is the running write index from the history response.
fn recent(buf: &[f32], current: u64, k: usize) -> Vec<f32> {
    let n = buf.len();
    if n == 0 {
        return Vec::new();
    }
    let k = k.min(n);
    let newest = (current as usize + n - 1) % n;
    let mut out = Vec::with_capacity(k);
    for i in (0..k).rev() {
        out.push(buf[(newest + n - i) % n]);
    }
    out
}

/// Normalize an angle difference into `(-180, 180]` degrees.
fn norm_deg(mut d: f32) -> f32 {
    while d > 180.0 {
        d -= 360.0;
    }
    while d <= -180.0 {
        d += 360.0;
    }
    d
}

/// Bits/s → Mbps.
fn mbps(bps: f32) -> f32 {
    bps / 1e6
}

/// Render a byte count as `1.2 GB` / `41.4 MB` / `512 B` (binary units).
fn human_bytes(bytes: u64) -> String {
    const K: f64 = 1024.0;
    let b = bytes as f64;
    if b >= K * K * K {
        format!("{:.1} GB", b / (K * K * K))
    } else if b >= K * K {
        format!("{:.1} MB", b / (K * K))
    } else if b >= K {
        format!("{:.1} KB", b / K)
    } else {
        format!("{bytes} B")
    }
}

/// Return `s`, or `-` if it is empty (for tidy table cells).
fn non_empty(s: &str) -> &str {
    if s.is_empty() {
        "-"
    } else {
        s
    }
}

/// Render a Unix timestamp (seconds) as an ISO-8601 UTC string.  A tiny
/// epoch-to-civil conversion (Howard Hinnant's algorithm) — no `chrono`.
fn iso_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (rem / 3600, rem / 60 % 60, rem % 60);
    // days since 1970-01-01 → civil y/m/d.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Render a seconds count as `1d 2h 3m` / `2h 3m` / `3m 4s` / `4s`.
fn human_duration(secs: u64) -> String {
    let (d, h, m, s) = (secs / 86400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Wall-clock time in nanoseconds since the Unix epoch (0 if the clock is
/// before the epoch, which never happens in practice).
fn now_unix_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
}

/// Drive a future to completion on the current thread.
///
/// Our [`Transport`] does blocking I/O and returns an already-ready future,
/// so a no-op waker that never actually parks is all we need.  Keeping a real
/// `block_on` (rather than calling `.call()` directly) means the day the
/// transport becomes genuinely async, only this function changes.
fn block_on<F: Future>(future: F) -> F::Output {
    use core::task::{Context, Poll};

    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut future = core::pin::pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_utc_known_epochs() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn shade_ramps_from_clear_to_obstructed() {
        assert_eq!(shade(0.0), '·'); // clear sky
        assert_eq!(shade(0.1), '░');
        assert_eq!(shade(0.4), '▒');
        assert_eq!(shade(0.7), '▓');
        assert_eq!(shade(1.0), '█'); // fully obstructed
    }

    #[test]
    fn recent_returns_chronological_tail_of_ring() {
        // current=3 over a 3-slot ring: newest sample is at (3-1)%3 = 2.
        let buf = [10.0f32, 20.0, 30.0];
        assert_eq!(recent(&buf, 3, 2), [20.0, 30.0]);
    }

    #[test]
    fn minutes_of_day_wraps_and_pads() {
        assert_eq!(minutes_of_day(0), "00:00");
        assert_eq!(minutes_of_day(135), "02:15");
        assert_eq!(minutes_of_day(1_440), "00:00");
    }
}
