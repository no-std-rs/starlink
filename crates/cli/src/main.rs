//! Starlink Device API command-line client.
//!
//! Talks to the dish's `SpaceX.API.Device.Device/Handle` unary RPC over a
//! hand-rolled plaintext HTTP/2 client (see [`h2`]).  No `tokio`, `hyper`, or
//! `tonic`: the binary is `std` today but everything it leans on has a
//! `no_std` path so the embedded port stays open.
//!
//! Commands:
//!
//! ```text
//! starlink status   [--addr HOST:PORT]      # identity, link, obstruction, alignment, nodes
//! starlink align    [--addr HOST:PORT]      # how to aim the dish (current vs desired boresight)
//! starlink history  [--addr HOST:PORT] [--samples N]   # latency / throughput / power / outages
//! starlink devices  [--addr HOST:PORT]      # downstream routers / mesh nodes
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
    attitude_estimation_state_name, has_actuators_name, outage_cause_name, response::Body,
    router_role_name, DishGetHistoryResponse, DishGetStatusResponse, Response,
};

/// Default dish endpoint on the standard Starlink management subnet.
const DEFAULT_ADDR: &str = "192.168.100.1:9200";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let rest = &args[args.len().min(2)..];
    let addr = parse_opt(rest, "--addr").unwrap_or_else(|| DEFAULT_ADDR.to_string());

    let result = match args.get(1).map(String::as_str) {
        Some("status") => cmd_status(&addr),
        Some("align") => cmd_align(&addr),
        Some("history") => {
            let samples = parse_opt(rest, "--samples")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(900);
            cmd_history(&addr, samples)
        }
        Some("devices" | "nodes") => cmd_devices(&addr),
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
    eprintln!("commands:");
    eprintln!("  status              identity, link, obstruction, alignment, nodes");
    eprintln!("  align               how to aim the dish (current vs desired boresight)");
    eprintln!("  history [--samples N]   latency / throughput / power / outage history");
    eprintln!("  devices             downstream routers / mesh nodes");
    eprintln!();
    eprintln!("default addr {DEFAULT_ADDR}");
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
