#[derive(Debug, serde::Serialize)]
pub enum SubscriberStatus {
    Pending,
    Confirmed,
    Unsubscribed,
}

impl SubscriberStatus {
    pub fn is_pending(&self) -> bool {
        matches!(self, SubscriberStatus::Pending)
    }

    pub fn is_confirmed(&self) -> bool {
        matches!(self, SubscriberStatus::Confirmed)
    }

    pub fn is_unsubscribed(&self) -> bool {
        matches!(self, SubscriberStatus::Unsubscribed)
    }

    pub fn parse(status: String) -> Result<SubscriberStatus, String> {
        match status.as_str() {
            "pending_confirmation" => Ok(SubscriberStatus::Pending),
            "confirmed" => Ok(SubscriberStatus::Confirmed),
            "unsubscribed" => Ok(SubscriberStatus::Unsubscribed),
            _ => Err(format!("{} is not a valid subscriber status", status)),
        }
    }
}

impl AsRef<str> for SubscriberStatus {
    fn as_ref(&self) -> &str {
        match self {
            SubscriberStatus::Pending => "pending_confirmation",
            SubscriberStatus::Confirmed => "confirmed",
            SubscriberStatus::Unsubscribed => "unsubscribed",
        }
    }
}
