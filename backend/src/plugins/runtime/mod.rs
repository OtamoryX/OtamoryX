//! Runtime support for installed plugins.
//!
//! Plugin contracts and built-in implementations live at `crate::plugins`; this module owns
//! manifest handling, native execution, host APIs, scheduling, and lifecycle support.

pub mod bootstrap;
pub mod event_bus;
pub mod executor;
pub mod host_api;
pub mod manager;
pub mod manifest;
pub mod scheduler;
pub mod security;
