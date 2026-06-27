//! `prost`-derived message types for the Starlink dish's gRPC surface.
//!
//! This crate is `no_std + alloc`.  It depends on the `prost` runtime with
//! `default-features = false, features = ["derive"]`, which pulls in the
//! derive macros and the encoding helpers without any `std`-specific code.
//!
//! We intentionally do **not** depend on `protoc`, `prost-build`, `protox`,
//! or `prost-reflect`.  Every type in this crate is a hand-written Rust
//! struct with `#[derive(::prost::Message)]` and explicit `#[prost(...)]`
//! tag annotations — this keeps the set of fields we know about inline with
//! the code, visible to `grep` and to reviewers, and avoids any build-time
//! code generation.
//!
//! Tag numbers originate from reflection-describe output captured on a
//! live dish running firmware `2026.04.03.cr77363`; see inline comments.
//!
//! # Modules
//!
//! * [`device`] — `SpaceX.API.Device.Device` request types (`Request` and
//!   its oneof arms, each an empty sub-message for the read-only methods we
//!   currently care about).
//! * [`reflection`] — `grpc.reflection.v1.ServerReflection` request types,
//!   used to pull `FileDescriptorSet` bytes off a live dish at runtime for
//!   schema auditing.
//! * [`response`] — `SpaceX.API.Device.Response` decode types (the
//!   `dish_get_status` arm and the messages it nests).  Response oneof tags
//!   differ from the request arms and were captured from a fresh reflection
//!   dump on firmware `2026.06.15.mr81291`.
#![no_std]

extern crate alloc;

pub mod device;
pub mod reflection;
pub mod response;
