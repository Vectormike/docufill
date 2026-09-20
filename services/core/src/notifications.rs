use crate::{AppError, AppResult, config::Config};

#[derive(Clone)]
pub struct NotificationService {
    client: reqwest::Client,
    api_key: Option<String>,
    from: Option<String>,
}

impl NotificationService {
    pub fn new(config: &Config) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: config.email_api_key.clone(),
            from: config.email_from.clone(),
        }
    }

    /// The verification code only ever reaches a participant by email, so an
    /// invite created without delivery can never be completed.
    pub fn can_deliver(&self) -> bool {
        self.api_key.is_some() && self.from.is_some()
    }

    pub async fn send_participant_invitation(
        &self,
        recipient: &str,
        requester: &str,
        subject: &str,
        share_url: &str,
        verification_code: &str,
    ) -> AppResult<bool> {
        let (Some(api_key), Some(from)) = (&self.api_key, &self.from) else {
            return Ok(false);
        };
        let requester = escape_html(requester);
        let subject = escape_html(subject);
        let share_url = escape_html(share_url);
        let verification_code = escape_html(verification_code);
        let response = self
            .client
            .post("https://api.resend.com/emails")
            .bearer_auth(api_key)
            .json(&serde_json::json!({
                "from": from,
                "to": [recipient],
                "subject": format!("{requester} invited you to complete a document"),
                "html": format!(
                    "<p><strong>{requester}</strong> asked you to complete questions for <strong>{subject}</strong>.</p>\
                     <p><a href=\"{share_url}\">Open your secure questions</a></p>\
                     <p>Your verification code is <strong>{verification_code}</strong>. It expires in 15 minutes.</p>\
                     <p>Docufill will not show you the original PDF or anyone else’s answers.</p>"
                )
            }))
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;

        if !response.status().is_success() {
            return Err(AppError::Upstream);
        }
        Ok(true)
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(api_key: Option<&str>, from: Option<&str>) -> NotificationService {
        NotificationService {
            client: reqwest::Client::new(),
            api_key: api_key.map(str::to_owned),
            from: from.map(str::to_owned),
        }
    }

    #[test]
    fn delivery_needs_both_a_key_and_a_sender() {
        assert!(service(Some("key"), Some("Docufill <mail@example.com>")).can_deliver());
        assert!(!service(None, Some("Docufill <mail@example.com>")).can_deliver());
        assert!(!service(Some("key"), None).can_deliver());
        assert!(!service(None, None).can_deliver());
    }

    #[test]
    fn invitation_content_is_escaped() {
        assert_eq!(
            escape_html("<script>&\"'"),
            "&lt;script&gt;&amp;&quot;&#39;"
        );
    }
}
