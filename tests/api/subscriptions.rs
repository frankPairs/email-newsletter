use email_newsletter::domain::subscriber::Subscriber;
use sqlx::{postgres::PgRow, Row};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;
use email_newsletter::{
    domain::subscriber_email::SubscriberEmail, domain::subscriber_name::SubscriberName,
    domain::subscriber_status::SubscriberStatus,
};

#[tokio::test]
async fn subscribe_returns_200_when_body_is_valid() {
    let test_app = TestApp::spawn_app().await;
    let mut body = HashMap::new();

    body.insert("name", "Frank");
    body.insert("email", "frank@test.com");

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscription(body).await;

    assert_eq!(201, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = TestApp::spawn_app().await;
    let mut body = HashMap::new();

    body.insert("name", "Test");
    body.insert("email", "test@test.com");

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    let new_subscription: Subscriber =
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
            .expect("Query to fetch subscriptions failed.");

    assert_eq!(new_subscription.email.as_ref(), "test@test.com");
    assert_eq!(new_subscription.name.as_ref(), "Test");
    assert_eq!(new_subscription.status.as_ref(), "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_400_when_body_require_field_is_missing() {
    let test_app = TestApp::spawn_app().await;

    // This is a common practice and it is called table-driven tests. In this case, it simulates different kind of possible request bodies
    // where API should return 400.
    let test_cases: Vec<(HashMap<&str, &str>, &str)> = vec![
        (HashMap::from([]), "mising body parameters"),
        (
            HashMap::from([("name", "Frank")]),
            "missing email parameter",
        ),
        (
            HashMap::from([("email", "frank@test.com")]),
            "missing name parameter",
        ),
        (HashMap::from([("name", "")]), "name cannot be empty"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscription(invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 status when payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_body_is_present_but_not_valid() {
    let test_app = TestApp::spawn_app().await;

    // This is a common practice and it is called table-driven tests. In this case, it simulates different kind of possible request bodies
    // where API should return 400.
    let test_cases: Vec<(HashMap<&str, &str>, &str)> = vec![
        (HashMap::from([]), "mising body parameters"),
        (
            HashMap::from([("name", "{Frank}"), ("email", "test@test.com")]),
            "invalid name parameter",
        ),
        (
            HashMap::from([("name", "Frank"), ("email", "test.com")]),
            "invalid email parameter",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscription(invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 status when payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = TestApp::spawn_app().await;
    let mut body = HashMap::new();

    body.insert("name", "Test");
    body.insert("email", "test@test.com");

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body).await;

    // Get the first request that was sent to the email server
    let received_requests = &test_app.email_server.received_requests().await.unwrap();

    test_app.get_confirmation_link(&received_requests[0]).await;

    assert_eq!(received_requests.len(), 1);
}
