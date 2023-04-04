use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::domain::subscriber_status::SubscriberStatus;

#[derive(Debug, serde::Serialize)]
pub struct Subscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub status: SubscriberStatus,
}
