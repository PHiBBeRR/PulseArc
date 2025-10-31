//! Activity classification domain

pub mod ports;
pub mod service;
pub mod signal_extractor;

pub use ports::*;
pub use service::*;
pub use signal_extractor::SignalExtractor;
