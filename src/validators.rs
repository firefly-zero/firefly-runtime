use core::str::Bytes;

/// Validate the full app ID.
///
/// The ID must contain the author ID and the short app ID separated by dot.
/// Both parts should have at least one character and may contain only
/// alphanumeric ASCII characters and hyphen.
///
/// Without ID validation, an app may use a malformed ID (like "../../../")
/// to gain access to arbitrary files of other apps, including secrets.
pub(crate) fn valid_full_id(s: &str) -> bool {
    let mut b = s.bytes();
    // validate author ID
    if !valid_id_part(&mut b) {
        return false;
    }
    // validate app ID
    if !valid_id_part(&mut b) {
        return false;
    }
    if s.ends_with('.') {
        return false;
    }
    // all bytes should be consumed (false if there is more than one dot)
    b.next().is_none()
}

fn valid_id_part(b: &mut Bytes<'_>) -> bool {
    let mut alpha_found = false;
    let mut prev_is_hyphen = false;
    for c in b {
        // stop consuming when dot is encountered
        if c == b'.' {
            break;
        }
        if c == b'-' {
            // forbid starting with hyphen
            if !alpha_found {
                return false;
            }
            // forbid two consecutive hyphens
            if prev_is_hyphen {
                return false;
            }
            prev_is_hyphen = true;
            continue;
        }
        prev_is_hyphen = false;
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() {
            return false;
        }
        alpha_found = true;
    }
    // forbid hyphen-only, empty names, or ending with hyphen
    alpha_found && !prev_is_hyphen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_full_id() {
        assert!(valid_full_id("user.app"));
        assert!(valid_full_id("some-user.some-app"));
        assert!(valid_full_id("user-name.app"));
        assert!(valid_full_id("user.app-name"));
        assert!(valid_full_id("user.relatively-long-app-name"));
        assert!(valid_full_id("relatively-long-user-name.app-name"));
        assert!(valid_full_id("a.b"));

        assert!(!valid_full_id("user.name.app")); // too many dots
        assert!(!valid_full_id("user_name.app")); // underscore is not allowed
        assert!(!valid_full_id("user name.app")); // whitespace is not allowed
        assert!(!valid_full_id("user.app_name")); // underscore is not allowed
        assert!(!valid_full_id("User.app")); // uppercase letters in author
        assert!(!valid_full_id("user.App")); // uppercase letters in app
        assert!(!valid_full_id("a")); // too short
        assert!(!valid_full_id("a.")); // too short
        assert!(!valid_full_id(".a")); // too short
        assert!(!valid_full_id("authorgame")); // no dot
        assert!(!valid_full_id("author-game")); // no dot
        assert!(!valid_full_id(".gamename")); // no author ID
        assert!(!valid_full_id("authorname.")); // no app ID
        assert!(!valid_full_id("author.game.")); // ends with dot
        assert!(!valid_full_id(".author.game")); // starts with dot
        assert!(!valid_full_id("author.name.game")); // too many dots
        assert!(!valid_full_id("author--name.game")); // two consecutive hyphens
        assert!(!valid_full_id("author-.game")); // ends with hyphen
        assert!(!valid_full_id("author.game-")); // ends with hyphen
        assert!(!valid_full_id("-author.game")); // starts with hyphen
        assert!(!valid_full_id("author.-game")); // starts with hyphen
    }
}
