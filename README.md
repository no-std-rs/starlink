# starlink-cli

A `no_std`-first Rust client for the Starlink Device gRPC API.

The dish exposes `SpaceX.API.Device.Device` on `192.168.100.1:9200` over plain
HTTP/2 with gRPC reflection enabled.  This workspace builds an
allocation-friendly-but-`std`-free client for that API, with a small CLI
binary on top that will eventually run on embedded targets as well as Linux.

## Workspace layout

```
starlink-cli/
├── Cargo.toml              # workspace manifest (declares `prost` in [workspace.dependencies])
├── rust-toolchain.toml     # pins stable
└── crates/
    ├── starlink-core/      # no_std + alloc — gRPC length-prefix framing
    │                       #   and the Transport trait.  Zero external
    │                       #   dependencies on purpose.
    ├── starlink-proto/     # no_std + alloc — prost-derived Device and
    │                       #   Reflection message types with tag numbers
    │                       #   inline next to the Rust fields.
    └── cli/                # binary crate (`starlink`) — wires a concrete
                            #   Transport to the core layer.  Targets
                            #   `rustix` + a hand-rolled HTTP/2 client, no
                            #   tokio / hyper / tonic.
```

## Design constraints

- **`no_std + alloc` throughout.**  The `core` and `proto` crates are
  `#![no_std]` with `extern crate alloc`.  The CLI binary is `std` today but
  will become `no_std` once the transport layer lands; nothing it depends on
  should make that transition impossible.
- **`prost` with `default-features = false`.**  The production build uses
  `prost` and `prost-derive` as runtime dependencies, but only in
  `alloc`-compatible mode — no `prost-build`, no `prost-reflect`, no
  `protoc`, no `protox`, no build-time code generation.  The few message
  types we need are hand-written Rust structs with `#[prost(...)]` tag
  annotations, which keeps the set of fields we know about visible to
  `grep` and to reviewers.
- **Schema provenance stays in the source.**  Tag numbers were captured
  from reflection-describe output on a running dish (firmware
  `2026.04.03.cr77363`) and cited in inline comments.  When we need to
  refresh, `starlink-proto::reflection` gives us the types to call the
  dish's own `ServerReflectionInfo` and dump raw `FileDescriptorProto`
  bytes to a file — no dynamic message runtime required on the device.
- **Async via `core::future::Future`.**  The `Transport` trait uses
  return-position `impl Trait` on a method, not `async-trait`, so it stays
  allocation-free at the interface boundary.  Executors are a CLI concern —
  planned choice is `embassy-executor`, which runs on both Linux and
  bare-metal targets.
- **No `tonic` / `hyper` / `tokio`.**  These are all std-only and would pull
  us back into the corner we are trying to avoid.  The CLI will use `rustix`
  for syscalls and a small in-tree HTTP/2 client.

## Why not `prost-reflect`?

`prost-reflect` (the Rust equivalent of grpcurl's dynamic message runtime)
is **std-only** — it uses `std::sync::RwLock`, `HashMap`, `thiserror`, and
there is no `no_std` feature flag.  For embedded targets that rules it out.
Porting its dynamic-message layer under `no_std + alloc` would be feasible
(swap `HashMap` → `BTreeMap`, `RwLock` → `Mutex` or single-threaded `RefCell`)
but not cheap — several thousand lines of code for a capability we do not
need, because we know the exact set of request arms we plan to send.

## Status

**v0.3 — end-to-end reads off a live dish.**  Workspace compiles clean,
clippy-pedantic green, 28 unit tests + 1 doc-test passing.  Verified against
a Starlink Mini (`mini1_prod2`, firmware `2026.06.15.mr81291`): every value
the CLI prints matches `grpcurl` captured in the same instant.

- `starlink-core::{frame, unframe, Transport, FrameError}` — gRPC
  length-prefix framing and the transport trait.  Still zero-dep.
- `starlink-proto::device` — `Request` and ten request-arm sub-messages with
  convenience constructors (`Request::get_status()`, `…::get_history()`, …)
  plus the `HANDLE_PATH` `:path` constant.
- `starlink-proto::reflection` — `ServerReflectionRequest` constructors.
- `starlink-proto::response` — `Response` envelope with the `dish_get_status`
  (2004) and `dish_get_history` (2006) oneof arms and the messages they nest
  (`DishGetStatusResponse`, `AlignmentStats`, `DishObstructionStats`,
  `DishGpsStats`, `RouterInfo`, `DishGetHistoryResponse`, `DishOutage`, …),
  with enum-name helpers.  Tags captured from a fresh reflection dump.
- `cli/` — working binary.  A hand-rolled prior-knowledge HTTP/2 (h2c) client
  (`src/h2.rs`, pure `std`, no `tokio`/`hyper`/`tonic`) implements the
  `Transport` trait.  Commands:
  - `starlink status` — identity, link rates, latency/drop, obstruction,
    alignment summary, GPS, mesh-node count.
  - `starlink align` — current vs desired boresight and the azimuth/elevation
    correction to physically aim the dish.
  - `starlink history [--samples N]` — min/avg/max/p95 over the 900-sample
    ring buffers for latency, ping-drop, downlink/uplink throughput, and
    power draw, plus buffered outages.
  - `starlink devices` (alias `nodes`) — downstream routers / mesh nodes with
    role and last-seen age.
  - `starlink clients` — **router** endpoint (`192.168.1.1:9000`, the LAN
    gateway, not the dish): attached Wi-Fi/Ethernet clients with interface,
    role, signal/SNR, PHY rates, byte counters, and mesh hop count.  The dish
    endpoint returns `Unimplemented` for client enumeration; the router serves
    it via the `wifi_get_clients` arm.
  - `starlink client-history [--samples N]` — router endpoint: per-client
    download/upload throughput history (min/avg/max/p95 over each client's
    900-sample ring buffer).  Enumerates clients, then fetches
    `wifi_get_client_history` for each one with a `client_id`.

## Roadmap

1. **More arms.**  `get_device_info`, `get_diagnostics` (location, hardware
   self-test), `dish_get_obstruction_map`, and `reflect list` / `reflect
   dump <file>` on top of the existing `reflection` request types.
2. **`rustix` transport.**  Swap `src/h2.rs`'s `std::net::TcpStream` for a
   `rustix` socket behind the same `Transport` trait; the framing and the
   HTTP/2 logic stay as-is.
3. **Executor.**  Replace the synchronous `block_on` with `embassy-executor`
   once the transport is genuinely async.
4. **Flip `cli/` to `#![no_std]`.**  The CLI binary is the last `std`
   consumer — once the transport and executor land we can drop the standard
   library entirely, paving the way for the embedded port.

## Why not just use `tonic`?

`tonic` is the natural choice for a desktop CLI but it is, at every layer,
built on `tokio` + `hyper` + `tower`, all of which require `std`.  Accepting
`tonic` even "just for the CLI" would mean maintaining two parallel client
stacks — one `tonic`-based for Linux and a second hand-rolled one for
embedded — with subtly different bugs.  Investing the `~500–1000` lines of
HTTP/2 framing once and sharing it across targets is the smaller long-term
bet.
