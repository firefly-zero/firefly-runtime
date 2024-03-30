/// Validate the full app ID.
///
/// The ID must contain the author ID and the short app ID separated by dot.
/// Both parts should have at least one character and may contain only
/// alphanumeric ASCII characters and hyphen.
pub(crate) fn valid_full_id(s: &str) -> bool {
    let mut dot_found = false;
    let mut len = 0;
    for c in s.bytes() {
        len += 1;
        if c == b'.' {
            // don't start with dot
            if len == 1 {
                return false;
            }
            // only one dot is allowed
            if dot_found {
                return false;
            }
            dot_found = true;
            continue;
        }
        // TODO: forbid hyphen-only names
        if !c.is_ascii_alphanumeric() && c != b'-' {
            return false;
        }
    }
    // TODO: require dot
    (3..=40).contains(&len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_full_id() {
        assert!(valid_full_id("user.app"));
        assert!(valid_full_id("User.App"));
        assert!(valid_full_id("some-user.some-app"));
        assert!(valid_full_id("Some-User.Some-App"));
        assert!(valid_full_id("user-name.APP"));
        assert!(valid_full_id("a.b"));
        assert!(valid_full_id("A.B"));

        assert!(!valid_full_id("user.name.app")); // too many dots
        assert!(!valid_full_id("user_name.app")); // underscore is not allowed
        assert!(!valid_full_id("user name.app")); // whitespace is not allowed
        assert!(!valid_full_id("user.app_name")); // underscore is not allowed
        assert!(!valid_full_id("a")); // too short
        assert!(!valid_full_id("a.")); // too short
        assert!(!valid_full_id(".a")); // too short
        assert!(!valid_full_id(".gamename")); // starts with dot
    }
}
