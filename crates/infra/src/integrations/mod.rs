//! External service integrations

#[cfg(feature = "calendar")]
pub mod calendar;

pub mod openai;

#[cfg(feature = "sap")]
pub mod sap;
