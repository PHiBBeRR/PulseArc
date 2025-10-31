//! Activity classification domain

pub mod evidence_extractor;
pub mod ports;
pub mod service;
pub mod signal_extractor;

pub use evidence_extractor::EvidenceExtractor;
pub use ports::*;
pub use service::*;
pub use signal_extractor::SignalExtractor;
