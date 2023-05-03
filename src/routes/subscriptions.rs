use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::Rng;
use reqwest::StatusCode;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::{
    domain::{
        new_subscriber::{NewSubscriber, NewSubscriberBody},
        subscriber::Subscriber,
        subscriber_email::SubscriberEmail,
        subscriber_name::SubscriberName,
        subscriber_status::SubscriberStatus,
    },
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[tracing::instrument(
    name = "Creating a new subscriber handler",
    skip(body, db_pool, email_client, base_url, redis_client),
    fields(
        subscriber_email = %body.email,
        subscriber_name = %body.name

    )
)]
pub async fn handle_create_subscription(
    body: web::Json<NewSubscriberBody>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
    redis_client: web::Data<redis::Client>,
) -> Result<HttpResponse, CreateSubscriptionError> {
    let new_subscriber: NewSubscriber = body
        .try_into()
        .map_err(CreateSubscriptionError::ValidationError)?;
    let subscriber = create_subscription(&new_subscriber, &db_pool)
        .await
        .map_err(CreateSubscriptionError::InsertSubscriptionError)?;
    let subscription_token = generate_subscription_token();

    store_subscription_token(&redis_client, &subscription_token, &subscriber.id).await?;
    send_confirmation_email(
        &email_client,
        &new_subscriber,
        base_url.0.as_str(),
        subscription_token.as_str(),
    )
    .await?;

    Ok(HttpResponse::Created().finish())
}

#[tracing::instrument(
    name = "Insert a new subscriber into the database",
    skip(new_subscriber, db_pool)
)]
async fn create_subscription(
    new_subscriber: &NewSubscriber,
    db_pool: &web::Data<PgPool>,
) -> Result<Subscriber, sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status) 
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        RETURNING id, email, name, subscribed_at, status
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(new_subscriber.email.as_ref())
    .bind(new_subscriber.name.as_ref())
    .bind(Utc::now())
    .map(|row: PgRow| Subscriber {
        id: row.get("id"),
        email: SubscriberEmail::parse(row.get("email")).unwrap(),
        name: SubscriberName::parse(row.get("name")).unwrap(),
        subscribed_at: row.get("subscribed_at"),
        status: SubscriberStatus::parse(row.get("status")).unwrap(),
    })
    .fetch_one(db_pool.get_ref())
    .await
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    fields(
        subscription_token = %subscription_token,
        base_url = %base_url
    ),
    skip(email_client, new_subscriber)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?token={}",
        base_url, subscription_token
    );
    let html_body = format!(
        r#"
            <div>
                <h1>Welcome to our newsletter!</>
                <p>Click <a href="{}">here</a> to confirm your subscription!</p>
            </div>
        "#,
        confirmation_link
    );

    email_client
        .send_email(
            new_subscriber.email.clone(),
            "Welcome to our newsletter",
            html_body.as_str(),
        )
        .await
}

#[tracing::instrument(
    name = "Store a subscription token in Redis",
    skip(redis_client)
    fields(
        subscription_token = %subscription_token,
        subscriber_id = %subscriber_id
    )
)]
async fn store_subscription_token(
    redis_client: &redis::Client,
    subscription_token: &str,
    subscriber_id: &Uuid,
) -> Result<(), StoreTokenError> {
    let mut redis_conn = redis_client.get_tokio_connection().await.map_err(|err| {
        tracing::error!("Failed to connect to Redis: {:?}", err);
        StoreTokenError(err)
    })?;

    redis::cmd("SET")
        .arg(format!(
            "subscription_token:{}:subscriber_id",
            subscription_token
        ))
        .arg(subscriber_id.to_string())
        .query_async(&mut redis_conn)
        .await
        .map_err(StoreTokenError)
}

fn generate_subscription_token() -> String {
    let mut rng = rand::thread_rng();

    std::iter::repeat_with(|| rng.sample(rand::distributions::Alphanumeric))
        .map(char::from)
        .take(30)
        .collect()
}

#[derive(thiserror::Error)]
pub enum CreateSubscriptionError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Failed to store the confirmation token for a new subscriber.")]
    StoreTokenError(#[from] StoreTokenError),
    #[error("Failed to send a confirmation email to a new subscriber.")]
    SendEmailError(#[from] reqwest::Error),
    #[error("Failed to insert a new subscriber into the database.")]
    InsertSubscriptionError(#[source] sqlx::Error),
}

impl std::fmt::Debug for CreateSubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caused by:\n\t({})", self)
    }
}

impl ResponseError for CreateSubscriptionError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub struct StoreTokenError(redis::RedisError);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while storing a subscription token."
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\nCaused by:\n\t({})", self, self.0)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
