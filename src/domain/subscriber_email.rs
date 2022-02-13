use std::fmt;

use validator::validate_email;

#[derive(Clone, Debug)]
pub struct SubscriberEmail(String);

impl TryFrom<&str> for SubscriberEmail {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl TryFrom<String> for SubscriberEmail {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if validate_email(&value) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid subscriber email.", value))
        }
    }
}

impl fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;

    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use quickcheck::Arbitrary;

    #[test]
    fn empty_string_is_rejected() {
        let email = "";
        assert_err!(SubscriberEmail::try_from(email));
    }
    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com";
        assert_err!(SubscriberEmail::try_from(email));
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com";
        assert_err!(SubscriberEmail::try_from(email));
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
        SubscriberEmail::try_from(valid_email.0).is_ok()
    }
}
