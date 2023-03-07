use actix_web::web;
use serde::Deserialize;

use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

#[derive(Deserialize)]
pub struct NewSubscriberBody {
    pub name: String,
    pub email: String,
}

impl TryFrom<web::Json<NewSubscriberBody>> for NewSubscriber {
    type Error = String;

    fn try_from(body: web::Json<NewSubscriberBody>) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(body.name.clone())?;
        let email = SubscriberEmail::parse(body.email.clone())?;

        Ok(NewSubscriber { email, name })
    }
}
