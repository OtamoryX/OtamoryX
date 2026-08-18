//! HTTP compatibility facade for the plugin application module.
//!
//! Plugin use cases live under `crate::plugins::application`; this module keeps the existing
//! handler path stable while routes and external callers migrate to the domain boundary.

pub use crate::plugins::application::*;
