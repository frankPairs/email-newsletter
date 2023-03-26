use sqlx::{postgres::PgRow, Row};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;
use email_newsletter::{
    domain::new_subscriber::NewSubscriber, domain::subscriber_email::SubscriberEmail,
    domain::subscriber_name::SubscriberName,
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
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscription(body).await;
    let new_subscription = sqlx::query("SELECT email, name FROM subscriptions;")
        .map(|row: PgRow| NewSubscriber {
            email: SubscriberEmail::parse(row.get("email")).unwrap(),
            name: SubscriberName::parse(row.get("name")).unwrap(),
        })
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Query to fetch subscriptions failed.");

    assert_eq!(new_subscription.email.as_ref(), "frank@test.com");
    assert_eq!(new_subscription.name.as_ref(), "Frank");
    assert_eq!(201, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_400_when_body_is_not_valid() {
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
