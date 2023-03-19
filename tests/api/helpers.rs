use std::collections::HashMap;

use reqwest::Response;
use sqlx::{migrate, Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use email_newsletter::{
    config::{get_configuration, DatabaseSettings},
    startup::{get_connection_db_pool, Application},
};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

impl TestApp {
    pub async fn spawn_app() -> TestApp {
        let mut config = get_configuration().expect("Missing configuration file.");

        config.set_db_name(Uuid::new_v4().to_string());
        // We are using port 0 as way to define a different port per each test. Port 0 is a special case that operating systems
        // take into account: when port is 0, the OS will search for the first available port
        config.set_app_port(0);

        configure_db(&config.database).await;

        let application = Application::build(config.clone())
            .await
            .expect("Failed to build application.");

        let address = format!("http://127.0.0.1: {}", application.get_port());

        tokio::spawn(application.run_until_stop());

        TestApp {
            address,
            db_pool: get_connection_db_pool(&config.database),
        }
    }

    pub async fn post_subscription(&self, body: HashMap<&str, &str>) -> Response {
        let client = reqwest::Client::new();
        let url = format!("http://{}/subscriptions", self.address);

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.");

        response
    }
}

async fn configure_db(config: &DatabaseSettings) -> PgPool {
    let mut db_connection = PgConnection::connect_with(&config.get_db_options_without_name())
        .await
        .expect("Failed to connect with the database");

    // Create new database
    db_connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("Failed to create database");

    let db_pool = PgPool::connect_with(config.get_db_options())
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
