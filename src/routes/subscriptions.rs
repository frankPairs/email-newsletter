use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rand::Rng;
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
) -> impl Responder {
    let new_subscriber: NewSubscriber = match body.try_into() {
        Ok(subscriber) => subscriber,
        Err(err) => {
            tracing::error!("Validation error: {:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    let subscriber = match create_subscription(&new_subscriber, &db_pool).await {
        Ok(subscriber) => subscriber,
        Err(err) => {
            tracing::error!("Failed to insert new subscriber: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let subscription_token = generate_subscription_token();

    if let Err(err) =
        store_subscription_token(&redis_client, &subscription_token, &subscriber.id).await
    {
        tracing::error!("Failed to store subscription token: {:?}", err);
        return HttpResponse::InternalServerError().finish();
    }

    if let Err(err) = send_confirmation_email(
        &email_client,
        &new_subscriber,
        base_url.0.as_str(),
        subscription_token.as_str(),
    )
    .await
    {
        tracing::error!(
            "Failed to send an email to {}: {:?}",
            new_subscriber.email.as_ref(),
            err
        );
        // return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Created().finish()
}

#[tracing::instrument(
    name = "Insert a new subscriber into the database",
    skip(new_subscriber, db_pool)
)]
async fn create_subscription(
    new_subscriber: &NewSubscriber,
    db_pool: &web::Data<PgPool>,
) -> Result<Subscriber, sqlx::Error> {
    let subscriber = sqlx::query(
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
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    });

    match subscriber {
        Ok(subscriber) => Ok(subscriber),
        Err(err) => {
            tracing::error!("Failed to execute query: {:?}", err);
            Err(err)
        }
    }
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
) -> Result<(), redis::RedisError> {
    let mut redis_conn = redis_client.get_tokio_connection().await?;

    redis::cmd("SET")
        .arg(format!(
            "subscription_token:{}:subscriber_id",
            subscription_token
        ))
        .arg(subscriber_id.to_string())
        .query_async(&mut redis_conn)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = rand::thread_rng();

    std::iter::repeat_with(|| rng.sample(rand::distributions::Alphanumeric))
        .map(char::from)
        .take(30)
        .collect()
}
