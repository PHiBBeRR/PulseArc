//! Activity classification domain

pub mod block_builder;
pub mod evidence_extractor;
pub mod ports;
pub mod project_matcher;
pub mod service;
pub mod signal_extractor;

pub use block_builder::BlockBuilder;
pub use evidence_extractor::EvidenceExtractor;
pub use ports::*;
pub use project_matcher::ProjectMatcher;
pub use service::*;
pub use signal_extractor::SignalExtractor;
