//! Meeting platform detection helpers.

/// Detect meeting platform from event title, description, or hangout link.
///
/// Checks indicators in the following order:
/// 1. Google Meet hangout link
/// 2. Zoom URLs or keywords
/// 3. Microsoft Teams URLs or keywords
/// 4. Phone / dial-in keywords
#[must_use]
pub fn detect_meeting_platform(
    title: Option<&str>,
    description: Option<&str>,
    hangout_link: Option<&str>,
) -> Option<String> {
    if hangout_link.is_some() {
        return Some("google_meet".to_string());
    }

    let combined =
        format!("{} {}", title.unwrap_or_default(), description.unwrap_or_default()).to_lowercase();

    if combined.contains("meet.google.com") {
        return Some("google_meet".to_string());
    }
    if combined.contains("zoom.us") || combined.contains("zoom") {
        return Some("zoom".to_string());
    }
    if combined.contains("teams.microsoft.com") || combined.contains("microsoft teams") {
        return Some("teams".to_string());
    }
    if combined.contains("phone") || combined.contains("dial-in") {
        return Some("phone".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::detect_meeting_platform;

    #[test]
    fn detect_by_hangout_link() {
        let detected = detect_meeting_platform(None, None, Some("meet.google.com/xyz"));
        assert_eq!(detected.as_deref(), Some("google_meet"));
    }

    #[test]
    fn detect_zoom_keyword() {
        let detected = detect_meeting_platform(Some("Zoom standup"), None, None);
        assert_eq!(detected.as_deref(), Some("zoom"));
    }

    #[test]
    fn detect_teams_url() {
        let detected = detect_meeting_platform(
            Some("Weekly sync"),
            Some("https://teams.microsoft.com/l/meetup-join/..."),
            None,
        );
        assert_eq!(detected.as_deref(), Some("teams"));
    }

    #[test]
    fn detect_phone_keywords() {
        let detected = detect_meeting_platform(None, Some("Dial-in 555-555"), None);
        assert_eq!(detected.as_deref(), Some("phone"));
    }

    #[test]
    fn unknown_when_no_keywords() {
        let detected = detect_meeting_platform(Some("Project review"), None, None);
        assert!(detected.is_none());
    }
}
