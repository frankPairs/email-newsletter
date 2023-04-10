use sqlx::{postgres::PgRow, Row};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;
use email_newsletter::domain::subscriber::Subscriber;
use email_newsletter::domain::subscriber_email::SubscriberEmail;
use email_newsletter::domain::subscriber_name::SubscriberName;
use email_newsletter::domain::subscriber_status::SubscriberStatus;

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

#[tokio::test]
async fn subscriptions_change_to_confirmed_after_clicking_confirmation_link() {
    let test_app = TestApp::spawn_app().await;
    let client = reqwest::Client::new();
    let mut body = HashMap::new();

    body.insert("name", "Frank");
    body.insert("email", "test@test.com");

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    // Get the first request that was sent to the email server
    let received_requests = &test_app.email_server.received_requests().await.unwrap();
    let confirmation_link = test_app.get_confirmation_link(&received_requests[0]).await;

    client
        .get(confirmation_link.html)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let subscriber =
        sqlx::query("SELECT id, email, name, subscribed_at, status FROM subscriptions;")
            .map(|row: PgRow| Subscriber {
                id: row.get("id"),
                email: SubscriberEmail::parse(row.get("email")).unwrap(),
                name: SubscriberName::parse(row.get("name")).unwrap(),
                subscribed_at: row.get("subscribed_at"),
                status: SubscriberStatus::parse(row.get("status")).unwrap(),
            })
            .fetch_one(&test_app.db_pool)
            .await
            .expect("Failed to fetch saved subscription.");

    assert_eq!(
        subscriber.status.as_ref(),
        SubscriberStatus::Confirmed.as_ref()
    );
}
