use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;

#[tokio::test]
async fn subcriptions_without_token_are_rejected_with_400() {
    let app = TestApp::spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/subscriptions/confirm", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn subscriptions_coming_from_link_are_confirmed() {
    let test_app = TestApp::spawn_app().await;
    let client = reqwest::Client::new();
    let mut body = HashMap::new();

    body.insert("name", "Frank");
    body.insert("email", "frank@test.com");

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    // Get the first request that was sent to the email server
    let received_requests = &test_app.email_server.received_requests().await.unwrap();
    let confirmation_link = test_app.get_confirmation_link(&received_requests[0]).await;

    let response = client.get(confirmation_link.html).send().await.unwrap();

    assert_eq!(response.status(), 200);
}
