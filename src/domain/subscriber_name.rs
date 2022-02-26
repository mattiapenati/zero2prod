use serde_with::{DeserializeFromStr, SerializeDisplay};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr)]
pub struct SubscriberName(String);

#[derive(Debug, thiserror::Error)]
pub enum ParseSubscriberNameError {
    #[error("subscriber name cannot be empty or whitespaces")]
    EmptyOrWhitespace,
    #[error("subscriber name too long")]
    TooLong,
    #[error("subscriber name contains forbidden characters")]
    ForbiddenCharacters,
}

impl std::str::FromStr for SubscriberName {
    type Err = ParseSubscriberNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const MAXIMUM_LENGTH: usize = 256;
        const FORBIDDEN_CHARACTERS: &[char] = &['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        if s.trim().is_empty() {
            return Err(ParseSubscriberNameError::EmptyOrWhitespace);
        }
        if s.graphemes(true).count() > MAXIMUM_LENGTH {
            return Err(ParseSubscriberNameError::TooLong);
        }
        if s.chars().any(|c| FORBIDDEN_CHARACTERS.contains(&c)) {
            return Err(ParseSubscriberNameError::ForbiddenCharacters);
        }

        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for SubscriberName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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
        assert_ok!(name.parse::<SubscriberName>());
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(name.parse::<SubscriberName>());
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(name.parse::<SubscriberName>());
    }
    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(name.parse::<SubscriberName>());
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(name.parse::<SubscriberName>());
        }
    }
    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(name.parse::<SubscriberName>());
    }
}
