use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipmentEvent {
    pub shipment_id: String,
    pub event: String,
    pub proof_reference: Option<String>,
    pub exception_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlingDecision {
    AskModel { prompt: String },
    HoldForReview { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardError {
    InvalidEvent(String),
    MissingApiKey,
    Transport(String),
    Api { code: String, status: u16 },
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEvent(message) => write!(f, "invalid shipment event: {message}"),
            Self::MissingApiKey => write!(f, "INFRAI_API_KEY is required"),
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::Api { code, status } => write!(f, "API error {code} (HTTP {status})"),
        }
    }
}

impl std::error::Error for GuardError {}

pub fn decide(event: &ShipmentEvent, cap_usd: &str, budget_allows_inference: bool) -> Result<HandlingDecision, GuardError> {
    if event.shipment_id.trim().is_empty() || event.event.trim().is_empty() {
        return Err(GuardError::InvalidEvent("shipment_id and event are required".into()));
    }
    if !budget_allows_inference {
        return Ok(HandlingDecision::HoldForReview {
            reason: format!("the account cap of {cap_usd} blocks another model request"),
        });
    }
    let proof = event.proof_reference.as_deref().unwrap_or("no proof attached");
    let exception = event.exception_note.as_deref().unwrap_or("none");
    Ok(HandlingDecision::AskModel {
        prompt: format!(
            "Classify shipment {}. Event: {}. Proof: {}. Exception: {}. Return a concise dispatch action.",
            event.shipment_id, event.event, proof, exception
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_holds_an_exception_for_human_review() {
        let event = ShipmentEvent {
            shipment_id: "SHP-204".into(),
            event: "delivery_exception".into(),
            proof_reference: Some("pod/SHP-204.jpg".into()),
            exception_note: Some("recipient unavailable".into()),
        };

        assert_eq!(
            decide(&event, "25.00", false).unwrap(),
            HandlingDecision::HoldForReview {
                reason: "the account cap of 25.00 blocks another model request".into()
            }
        );
    }
}
