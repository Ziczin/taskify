//! Хранилище: журнал, кеши, пересчёт, бэкапы.

pub mod aggregates;
pub mod apply;
pub mod backup;
pub mod cache;
pub mod event;
pub mod livespace;
pub mod log;
pub mod replay;
pub mod state;
pub mod store;

pub use aggregates::Aggregates;
pub use apply::Rules;
pub use event::{Event, EventKind};
pub use livespace::{GameSettings, Livespace, LivespaceBuilder};
pub use replay::ReplayResult;
pub use state::{State, StickerPlacement};
pub use store::Store;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod test_support;
