//! Adapters for converting between legacy and new architecture types.
//!
//! This module contains adapters that bridge the gap between the new hexagonal
//! architecture (with granular ports and domain types) and legacy monolithic
//! types expected by the frontend during the Phase 4 migration.

pub mod blocks;
pub mod database_stats;
