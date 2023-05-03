use linkify::{LinkFinder, LinkKind};
use reqwest::Response;
use reqwest::Url;
use sqlx::{migrate, Connection, Executor, PgConnection, PgPool};
use std::collections::HashMap;
use uuid::Uuid;
use wiremock::MockServer;

use email_newsletter::{
    config::{get_configuration, DatabaseSettings, Settings},
    email_client::SendEmailBody,
    startup::{get_connection_db_pool, Application},
};

pub struct ConfirmationLink {
    pub html: reqwest::Url,
}

pub struct TestApp {
    pub config: Settings,
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

impl TestApp {
    pub async fn spawn_app() -> TestApp {
        let mut config = get_configuration().expect("Missing configuration file.");

        let email_server = MockServer::start().await;

        // We are using port 0 as way to define a different port per each test. Port 0 is a special case that operating systems
        // take into account: when port is 0, the OS will search for the first available port
        config.set_app_port(0);
        config.set_email_client_base_url(email_server.uri());

        let db_pool = configure_db(&mut config.database).await;

        let application = Application::build(config.clone())
            .await
            .expect("Failed to build application.");
        let application_port = application.get_port();

        let address = format!("http://127.0.0.1:{}", application_port);

        tokio::spawn(application.run_until_stop());

        TestApp {
            address,
            config: config.clone(),
            db_pool,
            email_server,
            port: application_port,
        }
    }

    pub async fn post_subscription(&self, body: HashMap<&str, &str>) -> Response {
        let client = reqwest::Client::new();
        let url = format!("{}/subscriptions", self.address);

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .expect("Failed to execute post subscription request.");

        response
    }

    pub async fn post_newsletter(&self, body: serde_json::Value) -> Response {
        let client = reqwest::Client::new();
        let url = format!("{}/newsletters", self.address);

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .expect("Failed to execute post newsletter request.");

        response
    }

    pub async fn get_confirmation_link(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLink {
        let body: &SendEmailBody = &email_request.body_json().unwrap();
        let links: Vec<_> = LinkFinder::new()
            .links(body.content[0].value.as_str())
            .filter(|l| *l.kind() == LinkKind::Url)
            .collect();
        let raw_confirmation_link = links[0].as_str();
        let mut confirmation_link = Url::parse(raw_confirmation_link).unwrap();

        assert_eq!(confirmation_link.host_str().unwrap(), "localhost");

        confirmation_link.set_port(Some(self.port)).unwrap();

        ConfirmationLink {
            html: confirmation_link,
        }
    }
}

async fn configure_db(db_config: &mut DatabaseSettings) -> PgPool {
    let db_test_name = format!("db_{}", Uuid::new_v4().to_string().replace('-', "_"));

    // Create database
    let mut connection = PgConnection::connect_with(&db_config.get_db_options())
        .await
        .expect("Failed to connect to Postgres.");

    connection
        .execute(&*format!(r#"CREATE DATABASE "{}";"#, db_test_name))
        .await
        .expect("Failed to create database.");

    connection
        .close()
        .await
        .expect("Failed to close connection.");

    // Execute migrations
    db_config.set_name(db_test_name.clone());

    let db_pool = get_connection_db_pool(db_config);

    migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations.");

    println!("Database {} created!!", db_test_name);

    db_pool
}
