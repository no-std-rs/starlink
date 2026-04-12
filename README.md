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

**v0.2 — encoders on prost.**  Workspace compiles clean, clippy-pedantic
green, 13 unit tests + 1 doc-test passing:

- `starlink-core::{frame, unframe, Transport, FrameError}` — gRPC
  length-prefix framing and the transport trait.  Still zero-dep.
  (7 unit tests.)
- `starlink-proto::device` — `Request` and ten request-arm sub-messages
  (`GetStatusRequest`, `GetHistoryRequest`, …) derived from
  `prost::Message`, with convenience constructors:
  `Request::get_status()`, `Request::get_history()`, etc.  Plus the
  `HANDLE_PATH` HTTP/2 `:path` constant.  (8 unit tests.)
- `starlink-proto::reflection` — `ServerReflectionRequest` with three
  convenience constructors (`list_services`, `file_containing_symbol`,
  `file_by_filename`) and the `v1` / `v1alpha` path constants.  (5 unit
  tests.)
- `cli/` — still a stub `main` that prints a TODO and exits 2.

## Roadmap

1. **Response decoders.**  Add prost-derived response types for the arms
   we actually read (`DishGetStatusResponse`, `DishGetHistoryResponse`,
   `DishGetObstructionMapResponse`, `GetDeviceInfoResponse`,
   `GetDiagnosticsResponse`) and a `Response` envelope with the oneof arms
   we care about.  Blocked on capturing the `Response` oneof tag numbers
   from a live reflection dump.
2. **HTTP/2 transport.**  Implement a minimal single-stream HTTP/2 client
   on top of a `rustix` TCP socket, behind the existing `Transport` trait.
   Only unary (and single-message bidi for reflection) needs to work.
3. **Executor.**  Wire `embassy-executor` into `cli/` as the async runtime.
4. **CLI subcommands.**  Flesh out `starlink status`, `starlink history`,
   `starlink device-info`, `starlink diagnostics`,
   `starlink obstruction-map`, `starlink reflect list`,
   `starlink reflect dump <file>`.
5. **Flip `cli/` to `#![no_std]`.**  The CLI binary is the last `std`
   consumer — once the transport is wired up we can drop the standard
   library entirely, paving the way for the embedded port.

## Why not just use `tonic`?

`tonic` is the natural choice for a desktop CLI but it is, at every layer,
built on `tokio` + `hyper` + `tower`, all of which require `std`.  Accepting
`tonic` even "just for the CLI" would mean maintaining two parallel client
stacks — one `tonic`-based for Linux and a second hand-rolled one for
embedded — with subtly different bugs.  Investing the `~500–1000` lines of
HTTP/2 framing once and sharing it across targets is the smaller long-term
bet.
