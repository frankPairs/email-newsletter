use email_newsletter::email_client::EmailClient;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

use email_newsletter::config::get_configuration;
use email_newsletter::startup::run;
use email_newsletter::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber(String::from("email_newsletter"), String::from("debug"));

    init_subscriber(subscriber);

    let config = get_configuration().expect("Missing configuration file.");
    let db_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.get_db_options());

    let sender_email = config
        .get_email_client_sender()
        .expect("Sender email is not valid");
    let email_client = EmailClient::new(
        config.get_email_client_base_url(),
        sender_email,
        config.get_email_client_api(),
    );
    let listener = TcpListener::bind(config.get_address()).expect("Failed to bind the address.");

    tracing::info!("Server listening on {}", config.get_address());

    run(listener, db_pool, email_client)?.await
}
