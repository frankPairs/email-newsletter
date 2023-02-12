use secrecy::ExposeSecret;
use sqlx::{migrate, query, Connection, Executor, PgConnection, PgPool};
use std::collections::HashMap;
use std::net::TcpListener;
use uuid::Uuid;

use email_newsletter::config::{get_configuration, DatabaseSettings};

struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    // We are using port 0 as way to define a different port per each test. Port 0 is a special case that operating systems
    // take into account: when port is 0, the OS will search for the first available port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let mut config = get_configuration().expect("Missing configuration file.");

    config.set_db_name(Uuid::new_v4().to_string());

    let db_pool = configure_db(&config.database).await;
    let port = listener.local_addr().unwrap().port();
    let server =
        email_newsletter::startup::run(listener, db_pool.clone()).expect("Faild to bind address");

    tokio::spawn(server);

    TestApp {
        address: format!("127.0.0.1:{}", port),
        db_pool,
    }
}

async fn configure_db(config: &DatabaseSettings) -> PgPool {
    let mut db_connection =
        PgConnection::connect(&config.get_url_without_db_name().expose_secret())
            .await
            .expect("Failed to connect with the database");

    // Create new database
    db_connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("Failed to create database");

    let db_pool = PgPool::connect(&config.get_url().expose_secret())
        .await
        .expect("Failed to connect with the database");

    // Execute migrations
    migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations.");

    println!("Database created!!");

    db_pool
}

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let url = format!("http://{}/health_check", test_app.address);
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}

#[tokio::test]
async fn subscribe_returns_200_when_body_is_valid() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let url = format!("http://{}/subscriptions", test_app.address);
    let mut body = HashMap::new();

    body.insert("name", "Frank");
    body.insert("email", "frank@test.com");

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");
    let new_subscription = query!("SELECT email, name FROM subscriptions;")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Query to fetch subscriptions failed.");

    assert_eq!(new_subscription.email, "frank@test.com");
    assert_eq!(new_subscription.name, "Frank");
    assert_eq!(201, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_400_when_body_is_not_valid() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let url = format!("http://{}/subscriptions", test_app.address);
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
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&url)
            .json(&invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 status when payload was {}",
            error_message
        );
    }
}
