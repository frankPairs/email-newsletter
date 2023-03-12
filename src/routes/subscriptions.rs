use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::new_subscriber::{NewSubscriber, NewSubscriberBody};

#[tracing::instrument(
    name = "Creating a new subscriber handler",
    skip(body, db_pool),
    fields(
        subscriber_email = %body.email,
        subscriber_name = %body.name

    )
)]
pub async fn handle_create_subscription(
    body: web::Json<NewSubscriberBody>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    let new_subscriber: NewSubscriber = match body.try_into() {
        Ok(subscriber) => subscriber,
        Err(err) => {
            tracing::error!("Validation error: {:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    match create_subscription(&new_subscriber, &db_pool).await {
        Ok(_) => {
            tracing::info!("New subscriber was created successfully.");
            HttpResponse::Created().finish()
        }
        Err(err) => {
            // We used the debug format {:?} in order to get as much information as possible
            tracing::error!("Failed to execute query: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[tracing::instrument(
    name = "Insert a new subscriber into the database",
    skip(new_subscriber, db_pool)
)]
async fn create_subscription(
    new_subscriber: &NewSubscriber,
    db_pool: &web::Data<PgPool>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at) 
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(new_subscriber.email.as_ref())
    .bind(new_subscriber.name.as_ref())
    .bind(Utc::now())
    .execute(db_pool.get_ref())
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    });

    Ok(())
}
