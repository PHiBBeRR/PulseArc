//! Calendar integration module (feature: calendar)
//!
//! Provides OAuth2 authentication and event synchronization for:
//! - Google Calendar
//! - Microsoft Calendar (Outlook/365)
//!
//! This module is only compiled when the `calendar` feature is enabled.

pub mod client;
pub mod oauth;
pub mod platform;
pub mod provider_impl;
pub mod providers;
pub mod sync;
pub mod types;

pub use client::CalendarClient;
pub use oauth::{
    extract_email_from_id_token, generate_token_reference_id, CalendarOAuthManager,
    CalendarOAuthSettings, OAuthCallbackServer, OAuthLoginSession, TokenResponse,
};
pub use platform::detect_meeting_platform;
pub use provider_impl::CalendarProviderImpl;
pub use providers::{create_provider, CalendarProviderTrait};
// Re-export parser from domain (for backwards compatibility)
pub use pulsearc_domain::{parse_event_title, ParsedEventTitle};
pub use sync::CalendarSyncWorker;
pub use types::{
    CalendarConnectionStatus, CalendarEvent, CalendarSyncSettings, TimelineCalendarEvent,
};
