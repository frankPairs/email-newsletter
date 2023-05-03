use crate::domain::subscriber_email::SubscriberEmail;
use crate::email_client::EmailClient;
use actix_web::{web, HttpResponse, ResponseError};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::{postgres::PgRow, PgPool, Row};

#[derive(Deserialize, Debug)]
pub struct NewNewsletter {
    pub title: String,
    pub content: NewsletterContent,
}

#[derive(Deserialize, Debug)]
pub struct NewsletterContent {
    pub html: String,
}

#[tracing::instrument(
    name = "Publishing a newsletter to all subscribers",
    skip(body, db_pool, email_client),
    fields(
        title = %body.title,
        content_html = %body.content.html
    )
)]
pub async fn publish_newsletter(
    body: web::Json<NewNewsletter>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishNewsletterError> {
    let subscriber_emails = get_subscribers(&db_pool).await?;

    if !subscriber_emails.is_empty() {
        email_client
            .broadcast_email(subscriber_emails, &body.title, &body.content.html)
            .await
            .map_err(PublishNewsletterError::SendEmailError)?;
    }

    Ok(HttpResponse::Ok().finish())
}

pub async fn get_subscribers(
    db_pool: &web::Data<PgPool>,
) -> Result<Vec<SubscriberEmail>, PublishNewsletterError> {
    sqlx::query(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .map(|row: PgRow| SubscriberEmail::parse(row.get("email")).unwrap())
    .fetch_all(db_pool.as_ref())
    .await
    .map_err(PublishNewsletterError::GetSubscribersError)
}

#[derive(thiserror::Error)]
pub enum PublishNewsletterError {
    #[error("Failed to send a confirmation email to a new subscriber.")]
    SendEmailError(#[from] reqwest::Error),
    #[error("Failed to get subscribers from the database.")]
    GetSubscribersError(#[source] sqlx::Error),
}

impl std::fmt::Debug for PublishNewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caused by:\n\t({})", self)
    }
}

impl ResponseError for PublishNewsletterError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishNewsletterError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PublishNewsletterError::GetSubscribersError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
