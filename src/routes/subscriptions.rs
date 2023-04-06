use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::new_subscriber::{NewSubscriber, NewSubscriberBody},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[tracing::instrument(
    name = "Creating a new subscriber handler",
    skip(body, db_pool, email_client, base_url),
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
) -> impl Responder {
    let new_subscriber: NewSubscriber = match body.try_into() {
        Ok(subscriber) => subscriber,
        Err(err) => {
            tracing::error!("Validation error: {:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    if let Err(err) = create_subscription(&new_subscriber, &db_pool).await {
        // We used the debug format {:?} in order to get as much information as possible
        tracing::error!("Failed to execute query: {:?}", err);
        return HttpResponse::InternalServerError().finish();
    }

    if let Err(err) =
        send_confirmation_email(&email_client, &new_subscriber, base_url.0.as_str()).await
    {
        tracing::error!(
            "Failed to send an email to {}: {:?}",
            new_subscriber.email.as_ref(),
            err
        );
        return HttpResponse::InternalServerError().finish();
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
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status) 
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
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

async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?token=1234", base_url);
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
