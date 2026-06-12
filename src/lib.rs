//! gitid — switch between git identities per directory tree.
//!
//! The library exposes the pure domain core (paths, stores, gitconfig
//! generation) plus the command implementations so they can be integration
//! tested. The binary in `main.rs` is a thin wrapper.

pub mod activation;
pub mod cli;
pub mod cmd;
pub mod gitconfig;
pub mod output;
pub mod paths;
pub mod shell;
pub mod store;
pub mod sync;
pub mod wizard;
