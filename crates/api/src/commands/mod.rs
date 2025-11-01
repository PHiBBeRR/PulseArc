//! Tauri commands - frontend to backend bridge

mod blocks;
mod calendar;
mod database;
mod feature_flags;
mod health;
mod idle;
mod idle_sync;
mod projects;
mod suggestions;
mod tracking;
pub mod user_profile; // Public for integration tests
mod window;

#[cfg(debug_assertions)]
mod seed_snapshots;

pub use blocks::*;
pub use calendar::*;
pub use database::*;
pub use feature_flags::*;
pub use health::*;
pub use idle::*;
pub use idle_sync::*;
pub use projects::*;
#[cfg(debug_assertions)]
pub use seed_snapshots::*;
pub use suggestions::*;
pub use tracking::*;
pub use user_profile::*;
pub use window::*;
