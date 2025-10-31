//! Common data types used throughout the application

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Activity context captured from the operating system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityContext {
    pub timestamp: DateTime<Utc>,
    pub app_name: String,
    pub window_title: String,
    pub url: Option<String>,
    pub document_path: Option<String>,
}

/// Time entry that represents a work period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: i64,
    pub description: String,
    pub project: Option<String>,
    pub wbs_code: Option<String>,
}

/// Activity snapshot stored periodically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub context: ActivityContext,
}

/// Configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_path: String,
    pub log_level: String,
}
