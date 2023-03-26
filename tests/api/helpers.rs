use reqwest::Response;
use sqlx::{migrate, PgPool};
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use uuid::Uuid;
use wiremock::MockServer;

use email_newsletter::{
    config::{get_configuration, DatabaseSettings, Settings},
    startup::{get_connection_db_pool, Application},
};

const TEST_DB_CONTAINER_NAME: &str = "email_newsletter_db";

pub struct TestApp {
    pub config: Settings,
    pub address: String,
    pub db_pool: PgPool,
    pub db_test_name: String,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn spawn_app() -> TestApp {
        let mut config = get_configuration().expect("Missing configuration file.");
        let db_test_name = format!("a_{}", Uuid::new_v4().to_string().replace('-', "_"));
        let email_server = MockServer::start().await;

        // We are using port 0 as way to define a different port per each test. Port 0 is a special case that operating systems
        // take into account: when port is 0, the OS will search for the first available port
        config.set_app_port(0);
        config.set_email_client_base_url(email_server.uri());

        configure_db(&config.database, db_test_name.clone()).await;

        let application = Application::build(config.clone())
            .await
            .expect("Failed to build application.");

        let address = format!("http://127.0.0.1:{}", application.get_port());

        tokio::spawn(application.run_until_stop());

        TestApp {
            address,
            config: config.clone(),
            db_pool: get_connection_db_pool(&config.database),
            db_test_name,
            email_server,
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
            .expect("Failed to execute request.");

        response
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let remove_db_command = format!(
            r#"docker exec {} psql -U {} {} -c "DROP DATABASE {};""#,
            TEST_DB_CONTAINER_NAME,
            self.config.get_db_username(),
            self.config.get_db_name(),
            self.db_test_name
        );
        let output = Command::new("/bin/bash")
            .args(["-c", remove_db_command.as_str()])
            .output()
            .expect("Failed to remove a test database");

        if !output.status.success() {
            std::io::stderr().write_all(&output.stderr).unwrap();
            panic!("Failed to remove a test database.");
        }

        println!("Database {} removed!!", self.db_test_name);
    }
}

async fn configure_db(config: &DatabaseSettings, db_test_name: String) -> PgPool {
    let create_db_command = format!(
        r#"docker exec {} psql -U {} {} -c "CREATE DATABASE {};""#,
        TEST_DB_CONTAINER_NAME,
        config.get_username(),
        config.get_name(),
        db_test_name
    );
    let output = Command::new("/bin/bash")
        .args(["-c", create_db_command.as_str()])
        .output()
        .expect("Failed to create a test database");

    if !output.status.success() {
        std::io::stderr().write_all(&output.stderr).unwrap();
        panic!("Failed to create a test database.");
    }

    let db_pool = PgPool::connect_with(config.get_db_options())
        .await
        .expect("Failed to connect with the database");

    // Execute migrations
    migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations.");

    println!("Database {} created!!", db_test_name);

    db_pool
}
