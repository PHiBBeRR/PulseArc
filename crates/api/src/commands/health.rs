//! Health check command for frontend monitoring

use tauri::State;

use crate::context::AppContext;
use crate::utils::health::HealthStatus;

/// Get application health status
///
/// Returns comprehensive health information including:
/// - Overall health score (0.0 - 1.0)
/// - Individual component health checks
/// - Timestamp of the health check
///
/// # Example Response
/// ```json
/// {
///   "is_healthy": true,
///   "score": 1.0,
///   "message": null,
///   "components": [
///     { "name": "database", "is_healthy": true, "message": null },
///     { "name": "feature_flags", "is_healthy": true, "message": null },
///     { "name": "tracking_service", "is_healthy": true, "message": null }
///   ],
///   "timestamp": 1698765432
/// }
/// ```
#[tauri::command]
pub async fn get_app_health(context: State<'_, AppContext>) -> Result<HealthStatus, String> {
    Ok(context.health_check().await)
}
