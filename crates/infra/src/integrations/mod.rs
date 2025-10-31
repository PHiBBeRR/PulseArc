//! External service integrations

#[cfg(feature = "calendar")]
pub mod calendar;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "sap")]
pub mod sap;
