//! Command execution helpers for Phase 4 migration
//!
//! Provides utilities to reduce boilerplate when implementing commands with
//! metrics tracking and logging.

use std::sync::Arc;
use std::time::Instant;

use pulsearc_domain::{PulseArcError, Result as DomainResult};

use crate::context::AppContext;
use crate::utils::logging::{error_label, log_command_execution, record_command_metric};

/// Execute a command with automatic metrics recording and logging
///
/// This helper reduces boilerplate for Phase 4 command migration by automatically:
/// - Timing command execution
/// - Logging execution via tracing
/// - Recording metrics to database for validation
/// - Handling metrics recording errors gracefully
///
/// # Example
///
/// ```rust,ignore
/// #[tauri::command]
/// pub async fn my_command(
///     param: String,
///     ctx: State<'_, Arc<AppContext>>,
/// ) -> Result<MyResponse, String> {
///     execute_with_metrics(
///         &ctx,
///         "my_module::my_command",
///         "new", // or "legacy"
///         || async {
///             // Your command logic here
///             ctx.some_service.do_something(&param).await
///                 .map_err(|e| format!("Error: {e}"))
///         }
///     ).await
/// }
/// ```
pub async fn execute_with_metrics<F, Fut, T>(
    ctx: &Arc<AppContext>,
    command_name: &str,
    implementation: &str,
    command_fn: F,
) -> DomainResult<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = DomainResult<T>>,
{
    let start = Instant::now();

    // Execute the command
    let result = command_fn().await;

    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_type = result.as_ref().err().map(error_label);

    // Log to tracing and record metrics to database
    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(ctx, command_name, implementation, elapsed, success, error_type).await;

    result
}

/// Execute a command with String error conversion (common Tauri pattern)
///
/// This is a convenience wrapper for commands that need to return `Result<T, String>`
/// for Tauri compatibility.
///
/// # Example
///
/// ```rust,ignore
/// #[tauri::command]
/// pub async fn my_string_command(
///     ctx: State<'_, Arc<AppContext>>,
/// ) -> Result<MyResponse, String> {
///     execute_with_string_error(
///         &ctx,
///         "my_module::my_string_command",
///         "new",
///         || async {
///             ctx.some_service.do_something().await
///         }
///     ).await
/// }
/// ```
pub async fn execute_with_string_error<F, Fut, T>(
    ctx: &Arc<AppContext>,
    command_name: &str,
    implementation: &str,
    command_fn: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = DomainResult<T>>,
{
    execute_with_metrics(ctx, command_name, implementation, command_fn)
        .await
        .map_err(|e| e.to_string())
}
