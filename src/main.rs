use email_newsletter::config::get_configuration;
use email_newsletter::startup::Application;
use email_newsletter::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber(String::from("email_newsletter"), String::from("debug"));

    init_subscriber(subscriber);

    let config = get_configuration().expect("Missing configuration file");
    let application = Application::build(config.clone())
        .await
        .expect("Failed to build the application.");

    tracing::info!("Server listening on {}", config.get_address());

    application.run_until_stop().await?;

    Ok(())
}
