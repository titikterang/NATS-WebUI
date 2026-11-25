//! async-nats adapter: one long-lived, authenticated connection per configured server.

mod broker;
mod events;

pub use broker::NatsBroker;
