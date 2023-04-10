use actix_web::{
    web::{self, Query},
    HttpResponse, Responder,
};
use serde::Deserialize;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{subscriber::Subscriber, subscriber_email::SubscriberEmail, subscriber_name::SubscriberName, subscriber_status::SubscriberStatus};

#[derive(Deserialize, Debug)]
pub struct Parameters {
    pub token: String,
}

#[tracing::instrument(
  name = "Confirm a newsletter subscription",
  skip(redis_client, db_pool), 
  fields(
    token = %parameters.token,
  )
)]
pub async fn handle_confirm_subscription(
    redis_client: web::Data<redis::Client>,
        db_pool: web::Data<PgPool>,
    parameters: Query<Parameters>,
) -> impl Responder {
  let subscription_token = &parameters.token;

  match  get_subscriber_id_from_token(&redis_client, subscription_token).await {
      Ok(Some(subscriber_id)) => {
          match confirm_subscriber(&db_pool, subscriber_id).await {
              Ok(_) => {
                  tracing::info!("Subscriber confirmed.");
                  HttpResponse::Ok().finish()
              }
              Err(err) => {
                  tracing::error!("Failed to confirm subscriber: {}.", err);
                  HttpResponse::InternalServerError().finish()
              }
          }
      },
      Ok(None) => {
          tracing::error!("Subscription token not found.");
          HttpResponse::NotFound().finish()
      }
      Err(_) => {
          tracing::error!("Invalid subscription token.");
          HttpResponse::BadRequest().finish()
      }
  }

}

#[tracing::instrument(
  name = "Get subscriber id from token.",
  skip(redis_client), 
  fields(
    subscription_token
  )
)]
pub async fn get_subscriber_id_from_token(
  redis_client: &redis::Client,
    subscription_token: &str,
) -> Result<Option<Uuid>, redis::RedisError> {
    let mut redis_conn = redis_client.get_tokio_connection().await?;

    let subscriber_id: String = redis::cmd("GET")
    .arg(format!("subscription_token:{}:subscriber_id", subscription_token))
    .query_async(&mut redis_conn).await?;

    match Uuid::parse_str(&subscriber_id) {
        Ok(subscriber_id) => Ok(Some(subscriber_id)),
        Err(err) => {
          tracing::error!("Failed to get a subscriber id from a token: {:?}", err);
          Ok(None)
        },
    }
}

#[tracing::instrument(
  name = "Change subscriber status to confirmed.",
  skip(db_pool), 
  fields(
    subscriber_id
  )
)]
pub async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<Subscriber, sqlx::Error> {
    let updated_subscriber = sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        RETURNING id, email, name, status, subscribed_at
        "#,
    )
    .bind(subscriber_id)
    .map(|row: PgRow| Subscriber {
      id: row.get("id"),
      email: SubscriberEmail::parse(row.get("email")).unwrap(),
      name: SubscriberName::parse(row.get("name")).unwrap(),
      subscribed_at: row.get("subscribed_at"),
      status: SubscriberStatus::parse(row.get("status")).unwrap(),
    })
    .fetch_one(db_pool)
    .await?;

    Ok(updated_subscriber)

}