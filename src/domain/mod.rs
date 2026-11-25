//! Domain layer: entities, value objects and business rules.
//!
//! No framework code lives here (no warp, rusqlite, async-nats or reqwest). Everything in this
//! layer is plain data plus validation, so it can be unit-tested without I/O.

pub mod client;
pub mod error;
pub mod message;
pub mod secret;
pub mod server;
pub mod status;
