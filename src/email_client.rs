use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use std::time;

use crate::domain::subscriber_email::SubscriberEmail;

const REQUEST_TIMEOUT: time::Duration = time::Duration::from_secs(10);

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    api_key: Secret<String>,
}

#[derive(serde::Serialize)]
pub struct SendEmailBody {
    personalizations: Vec<SengridPersonalization>,
    from: SengridEmail,
    subject: String,
    content: Vec<SengridContent>,
}

#[derive(serde::Serialize)]
struct SengridEmail {
    email: String,
}

#[derive(serde::Serialize)]
struct SengridPersonalization {
    to: Vec<SengridEmail>,
}

#[derive(serde::Serialize)]
struct SengridContent {
    content_type: String,
    value: String,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        api_key: Secret<String>,
        timeout: Option<time::Duration>,
    ) -> EmailClient {
        let http_client = Client::builder()
            .timeout(timeout.unwrap_or(REQUEST_TIMEOUT))
            .build()
            .unwrap();

        EmailClient {
            http_client,
            base_url,
            sender,
            api_key,
        }
    }

    pub async fn send_email(
        &self,
        recipent: SubscriberEmail,
        subject: &str,
        html_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/mail/send", self.base_url);
        let body = SendEmailBody {
            from: SengridEmail {
                email: String::from(self.sender.as_ref()),
            },
            personalizations: vec![SengridPersonalization {
                to: vec![SengridEmail {
                    email: String::from(recipent.as_ref()),
                }],
            }],
            subject: String::from(subject),
            content: vec![SengridContent {
                content_type: String::from("text/html"),
                value: String::from(html_content),
            }],
        };

        self.http_client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.api_key.expose_secret()),
            )
            .json(&body)
            .send()
            .await?
            .error_for_status()?; // return an error when server response status code is 4xx or 5xx

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct SendBodyMatcher;

    impl wiremock::Match for SendBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                return body.get("from").is_some()
                    && body.get("personalizations").is_some()
                    && body.get("subject").is_some()
                    && body.get("content").is_some();
            }

            false
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client =
            EmailClient::new(mock_server.uri(), sender, Secret::new(Faker.fake()), None);

        Mock::given(header_exists("Authorization"))
            .and(method("POST"))
            .and(path("/mail/send"))
            .and(header("Content-Type", "application/json"))
            .and(SendBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let response = email_client
            .send_email(subscriber_email, &subject, &content)
            .await;

        assert_ok!(response);
    }

    #[tokio::test]
    async fn send_email_fails_if_server_returns_500() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client =
            EmailClient::new(mock_server.uri(), sender, Secret::new(Faker.fake()), None);

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let response = email_client
            .send_email(subscriber_email, &subject, &content)
            .await;

        assert_err!(response);
    }

    #[tokio::test]
    async fn send_email_fails_if_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake()),
            Some(time::Duration::from_millis(100)),
        );

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(time::Duration::from_millis(120)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let response = email_client
            .send_email(subscriber_email, &subject, &content)
            .await;

        assert_err!(response);
    }
}
