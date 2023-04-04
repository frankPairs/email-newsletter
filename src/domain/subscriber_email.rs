use validator::validate_email;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(email: String) -> Result<SubscriberEmail, String> {
        let is_valid_email = validate_email(&email);

        if !is_valid_email {
            return Err(format!("{} email is not valid", email));
        }

        Ok(Self(email))
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
    use claim::{assert_err, assert_ok};
    use fake::{faker::internet::en::SafeEmail, Fake};

    #[test]
    fn empty_email_is_rejected() {
        let email = "".to_string();

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "franktest.com".to_string();

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@test.com".to_string();

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_valid_is_accepted() {
        let email = SafeEmail().fake();

        assert_ok!(SubscriberEmail::parse(email));
    }
}
