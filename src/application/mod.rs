//! Application layer: use cases. Services depend only on the domain and on the port traits in
//! [`ports`]; the concrete adapters (SQLite, NATS, HTTP monitoring) are injected by `main.rs`.

pub mod clients;
pub mod inspect;
pub mod ports;
pub mod publish;
pub mod servers;
pub mod status;
pub mod subscriptions;

#[cfg(test)]
pub(crate) mod fakes;
