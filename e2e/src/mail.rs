//! What the site mailed, read from the lane's Mailpit through its API
//! (`E2E_MAILPIT`, e2e/run.sh): the welcome mail's link, the sign-in code.
//! As an islands app on this stack reads its own.

use serde_json::Value;
use thirtyfour::prelude::*;

use crate::{AWAKE_WITHIN, POLL, Page};

impl Page {
    /// What the lane's Mailpit answers at `path` (its API), as JSON.
    async fn mailpit(&self, path: &str) -> WebDriverResult<Value> {
        let failed = |e: reqwest::Error| WebDriverError::RequestFailed(format!("Mailpit: {e}"));
        reqwest::get(format!("{}{path}", self.mailpit))
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(failed)?
            .json()
            .await
            .map_err(failed)
    }

    /// The text of the newest mail to `to` whose subject starts with
    /// `subject`, waited for: the site sends it in the background after its
    /// answer, and Mailpit indexes it a moment later.
    pub async fn mail_text(&self, to: &str, subject: &str) -> WebDriverResult<String> {
        let started = tokio::time::Instant::now();
        loop {
            let found = self
                .mailpit(&format!(
                    "/api/v1/search?query={}",
                    encoded(&format!("to:\"{to}\""))
                ))
                .await?;
            let id = found["messages"].as_array().and_then(|messages| {
                messages
                    .iter()
                    .find(|m| {
                        m["Subject"]
                            .as_str()
                            .is_some_and(|s| s.starts_with(subject))
                    })
                    .and_then(|m| m["ID"].as_str().map(str::to_owned))
            });
            if let Some(id) = id {
                let mail = self.mailpit(&format!("/api/v1/message/{id}")).await?;
                return Ok(mail["Text"].as_str().unwrap_or_default().to_owned());
            }
            if started.elapsed() > AWAKE_WITHIN {
                return Err(WebDriverError::Timeout(format!(
                    "no mail \"{subject}\" to {to} within {AWAKE_WITHIN:?}"
                )));
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// The first address in `text` on the lane's site: a mail's link.
    #[must_use]
    pub fn link_in(&self, text: &str) -> Option<String> {
        link_on(&self.base, text)
    }
}

/// The first address in `text` that starts with `base`, up to the first
/// space or line's end.
fn link_on(base: &str, text: &str) -> Option<String> {
    let at = text.find(base)?;
    let link: String = text[at..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    Some(link)
}

/// `query` for a URL's query string: Mailpit's search has spaces, quotes and
/// a colon in it.
fn encoded(query: &str) -> String {
    query
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{encoded, link_on};

    #[test]
    fn a_mails_link_is_found_on_the_lanes_site() {
        let text = "Verify your account with the link below:\n\nhttp://localhost:5160/verify?token=abc\n\nIf you didn't";
        assert_eq!(
            link_on("http://localhost:5160", text).as_deref(),
            Some("http://localhost:5160/verify?token=abc")
        );
        assert_eq!(link_on("http://localhost:5161", text), None);
    }

    #[test]
    fn mailpits_query_is_encoded() {
        assert_eq!(
            encoded("to:\"a+1@example.com\""),
            "to%3A%22a%2B1%40example.com%22"
        );
    }
}
