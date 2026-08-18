//! Collection use cases grouped by the collection domain.

#[path = "identity.rs"]
mod identity;
#[path = "query.rs"]
mod query;
#[path = "rebuild.rs"]
mod rebuild;
#[path = "versions.rs"]
mod versions;

pub mod service;

pub use service::*;
