//! Calendar provider abstraction
//!
//! Defines traits and implementations for different calendar providers
//! (Google Calendar, Microsoft Calendar).

pub mod google;
pub mod microsoft;
pub mod traits;

pub use traits::{create_provider, CalendarProviderTrait, FetchEventsResponse, RawCalendarEvent};
