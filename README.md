# starlink

A `no_std`-first Rust client for the Starlink Device gRPC API.

The dish exposes `SpaceX.API.Device.Device` on `192.168.100.1:9200` over plain
HTTP/2 with gRPC reflection enabled.  This workspace builds an
allocation-friendly-but-`std`-free client for that API, with a small CLI
binary on top that will eventually run on embedded targets as well as Linux.

## Workspace layout

```
starlink/
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

**v0.4 — broad read coverage off a live dish + router.**  Workspace compiles
clean, clippy-pedantic green, 41 unit tests + 1 doc-test passing.  Verified
against a Starlink Mini (`mini1_prod2`, firmware `2026.06.15.mr81291`) and its
router: every value the CLI prints matches `grpcurl` captured in the same
instant, and every arm the firmware does not serve is reported with the dish's
own `grpc-status` (e.g. `UNIMPLEMENTED`, `PERMISSION_DENIED`).

- `starlink-core::{frame, unframe, Transport, FrameError}` — gRPC
  length-prefix framing and the transport trait.  Still zero-dep.
- `starlink-proto::device` — `Request` with convenience constructors for ~20
  read arms (`get_status`, `get_history`, `dish_get_obstruction_map`,
  `get_diagnostics`, `dish_get_config`, `get_gnss_measurement`,
  `transceiver_get_status`, …) plus the `HANDLE_PATH` `:path` constant.
- `starlink-proto::reflection` — `ServerReflectionRequest` constructors.
- `starlink-proto::response` — `Response` envelope with ~22 oneof arms and the
  messages they nest (dish status/history/obstruction-map/diagnostics/config,
  Wi-Fi status/history/ping-metrics/clients, radio stats, GNSS, transceiver, …),
  each modelled as a decode subset with enum-name helpers.  Tags captured from
  a fresh reflection dump.
- `cli/` — working binary.  A hand-rolled prior-knowledge HTTP/2 (h2c) client
  (`src/h2.rs`, pure `std`, no `tokio`/`hyper`/`tonic`) implements the
  `Transport` trait.  It advertises a 32 MiB flow-control window so large
  replies (`wifi-history`'s ~350 KB of ring buffers) are not capped at the
  65535-byte default, and it structurally HPACK-parses the gRPC trailers to
  surface a non-OK `grpc-status` as a clean error.  Commands (dish endpoint
  unless noted):
  - `status` / `align` / `history` / `devices` — link, aiming, ring-buffer
    history, downstream mesh nodes.
  - `obstruction-map [--width N]` — downsamples the `dish_get_obstruction_map`
    grid and **draws it** as a shaded sky-view picture.
  - `diagnostics`, `device-info`, `dish-config`, `dish-context`, `location`,
    `gnss`, `time`, `ping`, `ping-host <HOST>`, `connections`, `interfaces`,
    `transceiver`, `transceiver-telemetry` — one read arm each.
  - `clients` / `client-history` — **router** endpoint (`192.168.1.1:9000`):
    attached Wi-Fi/Ethernet clients and per-client throughput history.
  - `wifi-status` / `wifi-history` — router endpoint: WAN address,
    dish/PoP/internet ping, alerts, and the multi-target (PoP/Google/Cloudflare
    × IPv4/IPv6) ping-drop history.
  - `ping-metrics` / `radio-stats` — router endpoint: internet ping metrics and
    per-band Wi-Fi radio thermal/antenna stats.

## Roadmap

1. **`rustix` transport.**  Swap `src/h2.rs`'s `std::net::TcpStream` for a
   `rustix` socket behind the same `Transport` trait; the framing and the
   HTTP/2 logic stay as-is.
2. **Executor.**  Replace the synchronous `block_on` with `embassy-executor`
   once the transport is genuinely async.
3. **Flip `cli/` to `#![no_std]`.**  The CLI binary is the last `std`
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

## Releases

[Runnerless](.runnerless-ci.ts) in `my-infra` opens a release PR from conventional
commits. Merging that PR creates a `vX.Y.Z` tag and GitHub Release. The
[publication workflow](.github/workflows/publish-release.yml) builds Linux x86_64
and macOS arm64 CLI archives and prepares Cargo upload bodies for `starlink-core`,
`starlink-proto`, and `starlink-cli`. It attaches the packages and their SHA-256
checksums to the GitHub Release without a crates.io credential. Runnerless then
verifies the release, packages, and checksums and uploads each crate as a
separate operation in dependency order. Cargo versions and internal dependency requirements are
aligned with the release tag before packaging.

Runnerless needs a crates.io API token in its Production package publishing
credential set. Connect this repository in the Runnerless application and add a
token on the crates.io All packages row.
The first token must allow publishing new crate names. Rotate it under the
same credential name for later releases; crates.io Trusted Publishing tokens
are tied to GitHub Actions and are not used by this Runnerless upload path.
