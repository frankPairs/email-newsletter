use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{query, PgPool};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateSubscriptionBody {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Creating a new subscriber handler",
    skip(body, db_pool),
    fields(
        subscriber_email = %body.email,
        subscriber_name = %body.name

    )
)]
pub async fn handle_create_subscription(
    body: web::Json<CreateSubscriptionBody>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    match create_subscription(&body, &db_pool).await {
        Ok(_) => {
            tracing::info!("New subscriber was created successfully.");
            HttpResponse::Created()
        }
        Err(err) => {
            // We used the debug format {:?} in order to get as much information as possible
            tracing::error!("Failed to execute query: {:?}", err);
            HttpResponse::InternalServerError()
        }
    }
}

#[tracing::instrument(
    name = "Insert a new subscriber into the database",
    skip(body, db_pool)
)]
async fn create_subscription(
    body: &CreateSubscriptionBody,
    db_pool: &web::Data<PgPool>,
) -> Result<(), sqlx::Error> {
    query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at) 
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        body.email,
        body.name,
        Utc::now()
    )
    .execute(db_pool.get_ref())
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);

        err
    });

    Ok(())
}
