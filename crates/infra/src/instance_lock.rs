//! Single-instance lock using PID files
//!
//! Prevents multiple instances of PulseArc from running simultaneously,
//! which can cause database locking issues.

use std::fs;
use std::path::{Path, PathBuf};

use pulsearc_domain::{PulseArcError, Result};

/// Single-instance lock manager
pub struct InstanceLock {
    pid_file: PathBuf,
}

impl InstanceLock {
    /// Create a new instance lock
    ///
    /// Returns an error if another instance is already running.
    pub fn acquire<P: AsRef<Path>>(lock_dir: P) -> Result<Self> {
        let pid_file = lock_dir.as_ref().join("pulsearc.pid");

        // Check if PID file exists
        if pid_file.exists() {
            if let Ok(content) = fs::read_to_string(&pid_file) {
                if let Ok(pid) = content.trim().parse::<u32>() {
                    if Self::is_process_running(pid) {
                        tracing::warn!(existing_pid = pid, "instance_lock.process_active");
                        return Err(PulseArcError::Database(format!(
                            "Another instance is already running (PID: {}). Please stop it first.",
                            pid
                        )));
                    }
                    tracing::warn!(stale_pid = pid, "instance_lock.stale_pid_file_detected");
                }
            }
            if let Err(err) = fs::remove_file(&pid_file) {
                tracing::warn!(error = %err, path = %pid_file.display(), "instance_lock.remove_stale_pid_failed");
            }
        }

        // Write current PID
        let current_pid = std::process::id();
        fs::write(&pid_file, current_pid.to_string())
            .map_err(|e| PulseArcError::Database(format!("Failed to create PID file: {}", e)))?;

        tracing::info!(pid = current_pid, path = %pid_file.display(), "instance_lock.acquired");

        Ok(Self { pid_file })
    }

    /// Check if a process is running on macOS
    #[cfg(target_os = "macos")]
    fn is_process_running(pid: u32) -> bool {
        use std::process::Command;

        // Use `kill -0` to check if process exists without sending a signal
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check if a process is running on other platforms
    #[cfg(not(target_os = "macos"))]
    fn is_process_running(pid: u32) -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::path::Path;

            let proc_path = Path::new("/proc").join(pid.to_string());
            return proc_path.exists();
        }

        #[cfg(target_os = "windows")]
        {
            tracing::warn!(pid = pid, "instance_lock.process_check_unsupported");
            return false;
        }

        #[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
        {
            tracing::warn!(pid = pid, "instance_lock.process_check_unknown_platform");
            false
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Clean up PID file when dropped
        if let Err(e) = fs::remove_file(&self.pid_file) {
            tracing::warn!(error = %e, path = %self.pid_file.display(), "instance_lock.remove_pid_failed");
        } else {
            tracing::info!(path = %self.pid_file.display(), "instance_lock.released");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    /// Validates `InstanceLock::acquire` behavior for the single instance
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `lock1.is_ok()` evaluates to true.
    /// - Ensures `lock2.is_err()` evaluates to true.
    /// - Ensures `lock3.is_ok()` evaluates to true.
    #[test]
    fn test_single_instance() {
        let temp_dir = env::temp_dir().join("pulsearc_test");
        fs::create_dir_all(&temp_dir).unwrap();

        // First instance should succeed
        let lock1 = InstanceLock::acquire(&temp_dir);
        assert!(lock1.is_ok());

        // Second instance should fail
        let lock2 = InstanceLock::acquire(&temp_dir);
        assert!(lock2.is_err());

        // Drop first lock
        drop(lock1);

        // Now second instance should succeed
        let lock3 = InstanceLock::acquire(&temp_dir);
        assert!(lock3.is_ok());

        // Cleanup
        drop(lock3);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
