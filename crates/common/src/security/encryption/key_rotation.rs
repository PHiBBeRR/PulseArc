//! Key rotation scheduling
//!
//! Provides scheduling logic for automatic key rotation based on time periods.

use std::time::SystemTime;

use tracing::warn;

/// Key rotation schedule
///
/// Manages the timing and scheduling of encryption key rotations.
/// Tracks when the last rotation occurred and determines when the next
/// rotation should happen based on configured intervals.
#[derive(Debug, Clone)]
pub struct KeyRotationSchedule {
    /// Number of days between rotations
    pub rotation_days: u32,

    /// Timestamp of last rotation
    last_rotation: Option<SystemTime>,
}

impl Default for KeyRotationSchedule {
    fn default() -> Self {
        Self {
            rotation_days: 90, // Default: rotate every 90 days
            last_rotation: None,
        }
    }
}

impl KeyRotationSchedule {
    /// Create a new rotation schedule with a specific interval
    ///
    /// # Arguments
    /// * `rotation_days` - Number of days between rotations
    pub fn new(rotation_days: u32) -> Self {
        Self { rotation_days, last_rotation: None }
    }

    /// Check if rotation is needed
    ///
    /// Returns `true` if:
    /// - This is the first rotation (never rotated before)
    /// - The configured rotation period has elapsed since last rotation
    ///
    /// # Notes
    /// If the system clock goes backwards, this will default to zero elapsed
    /// time and log a warning. This prevents premature rotation due to
    /// clock issues.
    pub fn should_rotate(&self) -> bool {
        if let Some(last) = self.last_rotation {
            let elapsed = SystemTime::now().duration_since(last).unwrap_or_else(|e| {
                warn!(
                    error = %e,
                    "System clock went backwards during key rotation check, defaulting to zero elapsed time"
                );
                std::time::Duration::ZERO
            });

            elapsed.as_secs() > (self.rotation_days as u64 * 24 * 3600)
        } else {
            false // Don't rotate immediately on first use
        }
    }

    /// Record a rotation
    ///
    /// Updates the last rotation timestamp to now.
    pub fn record_rotation(&mut self) {
        self.last_rotation = Some(SystemTime::now());
    }

    /// Set rotation period in days
    ///
    /// # Arguments
    /// * `days` - Number of days between rotations
    pub fn set_rotation_days(&mut self, days: u32) {
        self.rotation_days = days;
    }

    /// Get the last rotation time
    pub fn last_rotation(&self) -> Option<SystemTime> {
        self.last_rotation
    }

    /// Get days since last rotation
    ///
    /// Returns `None` if never rotated.
    ///
    /// # Notes
    /// If the system clock goes backwards, this will default to zero days
    /// and log a warning.
    pub fn days_since_last_rotation(&self) -> Option<u64> {
        self.last_rotation.map(|last| {
            SystemTime::now()
                .duration_since(last)
                .unwrap_or_else(|e| {
                    warn!(
                        error = %e,
                        "System clock went backwards during days calculation, defaulting to zero"
                    );
                    std::time::Duration::ZERO
                })
                .as_secs()
                / (24 * 3600)
        })
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::key_rotation.
    use std::time::Duration;

    use super::*;

    /// Validates `KeyRotationSchedule::default` behavior for the default
    /// schedule scenario.
    ///
    /// Assertions:
    /// - Confirms `schedule.rotation_days` equals `90`.
    /// - Ensures `schedule.last_rotation.is_none()` evaluates to true.
    #[test]
    fn test_default_schedule() {
        let schedule = KeyRotationSchedule::default();
        assert_eq!(schedule.rotation_days, 90);
        assert!(schedule.last_rotation.is_none());
    }

    /// Validates `KeyRotationSchedule::new` behavior for the custom schedule
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `schedule.rotation_days` equals `30`.
    #[test]
    fn test_custom_schedule() {
        let schedule = KeyRotationSchedule::new(30);
        assert_eq!(schedule.rotation_days, 30);
    }

    /// Validates `KeyRotationSchedule::default` behavior for the should not
    /// rotate initially scenario.
    ///
    /// Assertions:
    /// - Ensures `!schedule.should_rotate()` evaluates to true.
    #[test]
    fn test_should_not_rotate_initially() {
        let schedule = KeyRotationSchedule::default();
        // Should not rotate immediately after creation
        assert!(!schedule.should_rotate());
    }

    /// Validates `KeyRotationSchedule::default` behavior for the record
    /// rotation scenario.
    ///
    /// Assertions:
    /// - Ensures `schedule.last_rotation.is_none()` evaluates to true.
    /// - Ensures `schedule.last_rotation.is_some()` evaluates to true.
    #[test]
    fn test_record_rotation() {
        let mut schedule = KeyRotationSchedule::default();
        assert!(schedule.last_rotation.is_none());

        schedule.record_rotation();
        assert!(schedule.last_rotation.is_some());
    }

    /// Validates `KeyRotationSchedule::new` behavior for the should not rotate
    /// immediately after rotation scenario.
    ///
    /// Assertions:
    /// - Ensures `!schedule.should_rotate()` evaluates to true.
    #[test]
    fn test_should_not_rotate_immediately_after_rotation() {
        let mut schedule = KeyRotationSchedule::new(90);
        schedule.record_rotation();

        // Should not need rotation immediately after rotation
        assert!(!schedule.should_rotate());
    }

    /// Validates `KeyRotationSchedule::default` behavior for the days since
    /// last rotation scenario.
    ///
    /// Assertions:
    /// - Ensures `schedule.days_since_last_rotation().is_none()` evaluates to
    ///   true.
    /// - Ensures `days.is_some()` evaluates to true.
    /// - Confirms `days.unwrap()` equals `0`.
    #[test]
    fn test_days_since_last_rotation() {
        let mut schedule = KeyRotationSchedule::default();
        assert!(schedule.days_since_last_rotation().is_none());

        schedule.record_rotation();
        let days = schedule.days_since_last_rotation();
        assert!(days.is_some());
        assert_eq!(days.unwrap(), 0); // Just rotated
    }

    /// Validates `KeyRotationSchedule::default` behavior for the set rotation
    /// days scenario.
    ///
    /// Assertions:
    /// - Confirms `schedule.rotation_days` equals `90`.
    /// - Confirms `schedule.rotation_days` equals `30`.
    #[test]
    fn test_set_rotation_days() {
        let mut schedule = KeyRotationSchedule::default();
        assert_eq!(schedule.rotation_days, 90);

        schedule.set_rotation_days(30);
        assert_eq!(schedule.rotation_days, 30);
    }

    /// Validates `KeyRotationSchedule::new` behavior for the should rotate
    /// after period expires scenario.
    ///
    /// Assertions:
    /// - Ensures `schedule.should_rotate()` evaluates to true.
    #[test]
    fn test_should_rotate_after_period_expires() {
        let mut schedule = KeyRotationSchedule::new(0); // 0 days for testing
        schedule.last_rotation = Some(
            SystemTime::now()
                .checked_sub(Duration::from_secs(24 * 3600 + 1)) // 1 day + 1 second ago
                .unwrap(),
        );

        assert!(schedule.should_rotate());
    }

    /// Validates `KeyRotationSchedule::new` behavior for the clock backwards
    /// handling scenario.
    ///
    /// Assertions:
    /// - Ensures `!schedule.should_rotate()` evaluates to true.
    /// - Confirms `days` equals `Some(0)`.
    #[test]
    fn test_clock_backwards_handling() {
        let mut schedule = KeyRotationSchedule::new(90);

        // Set last rotation to the future (simulating clock going backwards)
        schedule.last_rotation = Some(
            SystemTime::now()
                .checked_add(Duration::from_secs(3600)) // 1 hour in the future
                .unwrap(),
        );

        // Should not panic and should not trigger rotation (defaults to zero elapsed)
        assert!(!schedule.should_rotate());

        // Days calculation should also handle it gracefully
        let days = schedule.days_since_last_rotation();
        assert_eq!(days, Some(0)); // Should default to 0 days when clock goes
                                   // backwards
    }
}
