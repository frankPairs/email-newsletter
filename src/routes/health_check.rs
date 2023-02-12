use actix_web::{HttpRequest, HttpResponse, Responder};

/// Endpoint used by clients to know if the server is working
#[tracing::instrument(name = "Health Check handler")]
pub async fn health_check(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
}
