const START: &[u8] = b"\nOPENDESK_ONBOARD_V1\n";
const END: &[u8] = b"\nOPENDESK_ONBOARD_END\n";

/// Append a trailing JSON overlay that a wrapper installer can read without
/// rewriting the signed PE image. Callers must not persist the stamped bytes
/// over the operator-provisioned original file.
pub fn stamp_windows_setup(bytes: &[u8], enrollment_token: &str, checkin_url: &str) -> Vec<u8> {
    let json = serde_json::json!({
        "enrollment_token": enrollment_token,
        "checkin_url": checkin_url,
    })
    .to_string();
    let mut out = strip_onboard_overlay(bytes).unwrap_or_else(|| bytes.to_vec());
    out.extend_from_slice(START);
    out.extend_from_slice(json.as_bytes());
    out.extend_from_slice(END);
    out
}

pub fn strip_onboard_overlay(bytes: &[u8]) -> Option<Vec<u8>> {
    let start = bytes
        .windows(START.len())
        .position(|window| window == START)?;
    Some(bytes[..start].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_windows_setup_appends_overlay_once() {
        let stamped = stamp_windows_setup(
            b"MZ-signed",
            "abc123",
            "https://rd.example.com/api/enrollments/check-in",
        );
        assert!(stamped.starts_with(b"MZ-signed"));
        assert!(
            stamped
                .windows(START.len())
                .filter(|window| *window == START)
                .count()
                == 1
        );
        let restamped = stamp_windows_setup(
            &stamped,
            "abc123",
            "https://rd.example.com/api/enrollments/check-in",
        );
        assert_eq!(
            restamped
                .windows(START.len())
                .filter(|window| *window == START)
                .count(),
            1
        );
        assert!(std::str::from_utf8(&stamped)
            .expect("utf8 overlay")
            .contains("\"enrollment_token\":\"abc123\""));
    }
}
