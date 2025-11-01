//! Tauri commands - frontend to backend bridge

mod calendar;
mod database;
mod feature_flags;
mod health;
mod projects;
mod suggestions;
mod tracking;
pub mod user_profile; // Public for integration tests
mod window;

pub use calendar::*;
pub use database::*;
pub use feature_flags::*;
pub use health::*;
pub use projects::*;
pub use suggestions::*;
pub use tracking::*;
pub use user_profile::*;
pub use window::*;
