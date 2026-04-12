//! Starlink Device API command-line client.
//!
//! Skeleton only.  Transport, proto types, and argument parsing land in
//! follow-up commits.  The long-term target is `no_std` with `rustix` driving
//! a hand-rolled HTTP/2 client and an embassy-based executor, so nothing in
//! this binary should pull in `tokio`, `hyper`, or `tonic`.

fn main() -> std::process::ExitCode {
    // Deliberately terse: this binary will be rewritten once the transport
    // layer lands.  The point of the v0 commit is the workspace skeleton.
    eprintln!("starlink: not yet implemented — see workspace README for roadmap");
    std::process::ExitCode::from(2)
}
