use serde_with::{DeserializeFromStr, SerializeDisplay};

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr)]
pub struct EmailAddress(String);

#[derive(Debug, thiserror::Error)]
pub enum ParseEmailAddressError {
    #[error("invalid email address")]
    Invalid,
}

impl std::str::FromStr for EmailAddress {
    type Err = ParseEmailAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if validator::validate_email(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(ParseEmailAddressError::Invalid)
        }
    }
}

impl std::fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::EmailAddress;

    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use quickcheck::Arbitrary;

    #[test]
    fn empty_string_is_rejected() {
        let email = "";
        assert_err!(email.parse::<EmailAddress>());
    }
    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com";
        assert_err!(email.parse::<EmailAddress>());
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com";
        assert_err!(email.parse::<EmailAddress>());
    }

    #[derive(Debug, Clone)]
    struct ValidEmail(String);

    impl Arbitrary for ValidEmail {
        fn arbitrary(_: &mut quickcheck::Gen) -> Self {
            Self(SafeEmail().fake())
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmail) -> bool {
        valid_email.0.parse::<EmailAddress>().is_ok()
    }
}
