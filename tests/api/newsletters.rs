use std::collections::HashMap;

use crate::helpers::{ConfirmationLink, TestApp};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = TestApp::spawn_app().await;

    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let body = serde_json::json!({
      "title": "Newsletter title",
      "content": {
        "html": "<p>Newsletter content</p>"
      }
    });
    let response = test_app.post_newsletter(body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = TestApp::spawn_app().await;

    create_confirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let body = serde_json::json!({
      "title": "Newsletter title",
      "content": {
        "html": "<p>Newsletter content</p>"
      }
    });
    let response = test_app.post_newsletter(body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_when_body_is_invalid() {
    let test_app = TestApp::spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
              "content": {
                "html": "<p>Newsletter content</p>"
              }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
            }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_newsletter(invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 status when payload was {}",
            error_message
        );
    }
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLink {
    let mut body: HashMap<&str, &str> = HashMap::new();

    body.insert("name", "Frank");
    body.insert("email", "test@test.com");

    // When executing a mock with the method mount_as_scoped, the mock will stop to listen the /mail/send endpoint when it goes out of scope (so, when the execution of create_unconfirmed_subscriber
    // ends).
    let _mock_guard = Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    let received_requests = &test_app.email_server.received_requests().await.unwrap();

    test_app.get_confirmation_link(&received_requests[0]).await
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(&test_app).await;
    let client = reqwest::Client::new();

    client
        .get(confirmation_link.html)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
