use unicode_segmentation::UnicodeSegmentation;

const MAX_CHAR_LENGHT: usize = 256;
const FORBIDDEN_CHARS: [char; 9] = ['/', '{', '}', '"', '>', '<', '\\', '(', ')'];

#[derive(Debug, serde::Serialize)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(name: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > MAX_CHAR_LENGHT;
        let contains_forbidden_chars = name.chars().any(|char| FORBIDDEN_CHARS.contains(&char));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_chars {
            return Err(format!("{} is not a valid subscriber name", name));
        }

        Ok(Self(name))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberName;
    use claim::{assert_err, assert_ok};

    #[test]
    fn test_name_lower_than_256_chars_is_invalid() {
        let name = "a".repeat(255);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn test_name_greater_than_256_chars_is_invalid() {
        let name = "a".repeat(257);

        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn test_name_only_with_whitespaces_is_invalid() {
        let name = String::from("  ");

        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn test_name_empty_is_invalid() {
        let name = String::from("");

        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn test_name_valid() {
        let name = String::from("Frank");

        assert_ok!(SubscriberName::parse(name));
    }
}
