use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;

use email_newsletter::config::get_configuration;
use email_newsletter::startup::run;
use email_newsletter::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber(String::from("email_newsletter"), String::from("debug"));
    init_subscriber(subscriber);

    let config = get_configuration().expect("Missing configuration file.");
    let db_pool = PgPool::connect(&config.get_db_url().expose_secret())
        .await
        .expect("Failed to connect to the Database.");
    let listener = TcpListener::bind(config.get_address()).expect("Failed to bind the address.");

    run(listener, db_pool)?.await
}
