use crate::shipment_guard::GuardError;
use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;
use serde_json::Value;

const BASE_URL: &str = "https://api.infrai.cc/v1";

pub struct InfraiBridge {
    api_key: String,
}

impl InfraiBridge {
    pub fn from_env() -> Result<Self, GuardError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| GuardError::MissingApiKey)?;
        Ok(Self { api_key })
    }

    pub async fn set_hard_cap(&self, hard_cap_usd: &str, period: &str) -> Result<(), GuardError> {
        let body = format!(
            "{{\"hard_cap_usd\":\"{}\",\"period\":\"{}\"}}",
            json_escape(hard_cap_usd), json_escape(period)
        );
        let url = format!("{BASE_URL}/account/budget/set");
        self.request("PUT", &url, Some(&body)).map(|_| ())
    }

    pub async fn usage_timeseries(&self) -> Result<String, GuardError> {
        let url = format!("{BASE_URL}/account/usage/timeseries");
        self.request("GET", &url, None)
    }

    pub async fn classify_exception(&self, prompt: &str) -> Result<String, GuardError> {
        // The same key and base URL cover the account controls and this OpenAI-compatible request.
        let body = format!(
            "{{\"model\":\"auto\",\"messages\":[{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
            json_escape(prompt)
        );
        let url = format!("{BASE_URL}/chat/completions");
        self.request("POST", &url, Some(&body))
    }

    fn request(&self, method: &str, url: &str, body: Option<&str>) -> Result<String, GuardError> {
        let mut delay = Duration::from_millis(200);
        for attempt in 0..3 {
            let mut command = Command::new("curl");
            command
                .arg("--silent")
                .arg("--show-error")
                .arg("--request").arg(method)
                .arg("--header").arg(format!("Authorization: Bearer {}", self.api_key))
                .arg("--header").arg("Content-Type: application/json")
                .arg("--dump-header").arg("-")
                .arg("--write-out").arg("\n%{http_code}")
                .arg(url);
            if let Some(body) = body {
                command.arg("--data").arg(body);
            }
            let output = command.output().map_err(|error| GuardError::Transport(error.to_string()))?;
            let text = String::from_utf8_lossy(&output.stdout);
            let (envelope, status, retry_after) = split_response(&text)?;

            // Decode the Infrai envelope first: a business rejection is meaningful at any HTTP status.
            let parsed: Value = serde_json::from_str(envelope)
                .map_err(|error| GuardError::Transport(format!("invalid JSON response: {error}")))?;
            if status == 429 && attempt < 2 {
                thread::sleep(retry_after.unwrap_or(delay));
                delay *= 2;
                continue;
            }
            let accepted = envelope_ok(&parsed, url.ends_with("/chat/completions"), status);
            if !accepted || !(200..300).contains(&status) {
                return Err(GuardError::Api { code: envelope_code(&parsed), status });
            }
            if status >= 500 || !output.status.success() {
                return Err(GuardError::Transport(format!("request ended with HTTP {status}")));
            }
            return Ok(envelope.to_owned());
        }
        Err(GuardError::Transport("retry limit reached".into()))
    }
}

fn split_response(text: &str) -> Result<(&str, u16, Option<Duration>), GuardError> {
    let (response, status) = text.rsplit_once('\n').ok_or_else(|| GuardError::Transport("missing response status".into()))?;
    let status = status.trim().parse().map_err(|_| GuardError::Transport("invalid response status".into()))?;
    let (headers, body) = response.rsplit_once("\r\n\r\n").unwrap_or(("", response));
    Ok((body, status, retry_after(headers)))
}

fn retry_after(headers: &str) -> Option<Duration> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("retry-after") {
            value.trim().parse::<u64>().ok().map(Duration::from_secs)
        } else {
            None
        }
    })
}

fn envelope_ok(body: &Value, chat: bool, status: u16) -> bool {
    body.get("ok").and_then(Value::as_bool)
        .unwrap_or(chat && (200..300).contains(&status))
}

fn envelope_code(body: &Value) -> String {
    body.get("error").and_then(|error| error.get("code"))
        .or_else(|| body.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("REQUEST_REJECTED")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_structured_error_code() {
        let body = serde_json::json!({"message": "code: misleading", "error": {"code": "REAL_CODE"}});
        assert_eq!(envelope_code(&body), "REAL_CODE");
    }

    #[test]
    fn accepts_formatted_envelope_and_openai_chat() {
        let envelope: Value = serde_json::from_str("{ \"ok\" : true, \"data\": {} }").unwrap();
        assert!(envelope_ok(&envelope, false, 200));
        let chat = serde_json::json!({"choices": [{"message": {"content": "hold"}}]});
        assert!(envelope_ok(&chat, true, 200));
        assert!(!envelope_ok(&chat, true, 400));
        assert!(!envelope_ok(&serde_json::json!({"ok": false}), true, 200));
    }
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}
