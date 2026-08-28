pub mod cli;
pub mod client;
pub mod connect_api;
mod validation;

/// Generated ConnectRPC message types, service traits, and clients.
///
/// The code is produced by `build.rs` (via `connectrpc-build`) into `$OUT_DIR`;
/// the `connect.rs` include file emitted there wires up the per-package
/// modules (e.g. `bountyhub_project_v1`, `bountyhub_runner_v1`).
pub mod connect {
    include!(concat!(env!("OUT_DIR"), "/connect.rs"));
}
