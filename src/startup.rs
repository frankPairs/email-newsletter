use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::config::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    handle_confirm_subscription, handle_create_subscription, health_check, publish_newsletter,
};

pub struct Application {
    pub port: u16,
    pub server: Server,
}

pub struct ApplicationBaseUrl(pub String);

impl Application {
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
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
            None,
        );
        let redis_client = redis::Client::open(config.get_redis_address())
            .expect("Failed to connect redis server.");

        let listener =
            TcpListener::bind(config.get_address()).expect("Failed to bind the address.");
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            db_pool,
            email_client,
            redis_client,
            config.get_app_base_url(),
        )?;

        Ok(Self { port, server })
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stop(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    redis_client: redis::Client,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let redis_client = web::Data::new(redis_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let server = HttpServer::new(move || {
        // App is where your application logic lives: routing, middlewares, request handler, etc
        App::new()
            // 'wrap' method adds a middleware to the App. This specific middleware provide incoming
            // request logger
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(handle_create_subscription))
            .route(
                "/subscriptions/confirm",
                web::get().to(handle_confirm_subscription),
            )
            .route("/newsletters", web::post().to(publish_newsletter))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(redis_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

pub fn get_connection_db_pool(config: &DatabaseSettings) -> Pool<Postgres> {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.get_db_options())
}
