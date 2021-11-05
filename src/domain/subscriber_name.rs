use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl TryFrom<&str> for SubscriberName {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        const MAXIMUM_LENGTH: usize = 256;
        const FORBIDDEN_CHARACTERS: &'static [char] =
            &['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let is_empty_or_whitespace = value.trim().is_empty();
        let is_tool_long = value.graphemes(true).count() > MAXIMUM_LENGTH;
        let contains_forbidden_characters =
            value.chars().any(|c| FORBIDDEN_CHARACTERS.contains(&c));

        if is_empty_or_whitespace || is_tool_long || contains_forbidden_characters {
            Err(format!("{} is not a valid subscriber name.", value))
        } else {
            Ok(Self(value.into()))
        }
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
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::try_from(name.as_str()));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::try_from(name.as_str()));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::try_from(name.as_str()));
    }
    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::try_from(name.as_str()));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::try_from(name.as_str()));
        }
    }
    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::try_from(name.as_str()));
    }
}
