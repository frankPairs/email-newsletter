use actix_web::{
    web::{self, Query},
    HttpResponse, Responder,
};
use redis::Connection;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Parameters {
    pub token: String,
}

#[tracing::instrument(
  name = "Confirm a newsletter subscription",
  skip(redis_conn), 
  fields(
    token = %parameters.token,
  )
)]
pub async fn handle_confirm_subscription(
    redis_conn: web::Data<Connection>,
    parameters: Query<Parameters>,
) -> impl Responder {
    HttpResponse::Ok().finish()
}
