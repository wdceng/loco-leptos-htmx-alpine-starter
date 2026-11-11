//! The people the tests sign in as: each test's own, signed up through the
//! site as a visitor does (the form, the welcome mail's link, its button),
//! an admin made one by its role in the lane's database. As an islands app
//! on this stack makes its own.

use std::time::Duration;

use thirtyfour::prelude::*;

use crate::Page;

/// Every test account's password: the rule's eight characters and more.
pub const PASSWORD: &str = "a long e2e password";

/// A test's own address, `name@example.com`, and on nextest's retry
/// `name+2@example.com`: the first try's account is still the lane's, and
/// a retry signing up with it would meet "the address is taken". Lower
/// case, as the site keeps addresses.
#[must_use]
pub fn mailbox(name: &str) -> String {
    match std::env::var("NEXTEST_ATTEMPT")
        .ok()
        .filter(|attempt| attempt != "1")
    {
        Some(attempt) => format!("{name}+{attempt}@example.com"),
        None => format!("{name}@example.com"),
    }
}

impl Page {
    /// `text` typed into the field `css`, as a person types it. In an iOS
    /// Simulator no key reaches a field (a probe of the accounts tests,
    /// 2026-11-11: all three of the sign-up's empty after typing), so
    /// there the page's own script sets it, with the `input` and `change`
    /// events typing fires.
    pub async fn type_into(&self, css: &str, text: &str) -> WebDriverResult<()> {
        let field = self.driver.find(By::Css(css)).await?;
        if self.browser.is_simulator() {
            self.driver
                .execute(
                    "const f = arguments[0]; f.focus(); f.value = arguments[1];
                     f.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: arguments[1] }));
                     f.dispatchEvent(new Event('change', { bubbles: true }))",
                    vec![field.to_json()?, serde_json::Value::from(text)],
                )
                .await
                .map(drop)
        } else {
            field.clear().await?;
            field.send_keys(text).await
        }
    }

    /// An account for `email`, `name` its name and [`PASSWORD`] its
    /// password, signed up through the sign-up form and verified through
    /// the welcome mail's link and its page's button, as a visitor does.
    /// Nobody signed in after it.
    pub async fn sign_up(&self, name: &str, email: &str) -> WebDriverResult<()> {
        self.open("/register").await?;
        self.type_into("#name", name).await?;
        self.type_into("#email", email).await?;
        self.type_into("#password", PASSWORD).await?;
        self.press("form[action='/register'] button[type=submit]")
            .await?;
        self.until(
            "the sign-up's \"check your mail\"",
            "return document.body.innerText.includes('Check your mail')",
        )
        .await?;
        let mail = self.mail_text(email, "Welcome to").await?;
        let link = self
            .link_in(&mail)
            .ok_or_else(|| WebDriverError::ParseError(format!("no link in:\n{mail}")))?;
        self.open(link.trim_start_matches(&self.base)).await?;
        self.press("form[action='/verify'] button[type=submit]")
            .await?;
        self.until(
            "the address verified",
            "return document.body.innerText.includes('Your email is verified')",
        )
        .await
    }

    /// `email` and `password` typed into the sign-in form and sent, then
    /// waited for until the page has left `/login`: the home page, or the
    /// admin's code (`/login/code`).
    pub async fn sign_in(&self, email: &str, password: &str) -> WebDriverResult<()> {
        self.open("/login").await?;
        self.type_into("#email", email).await?;
        self.type_into("#password", password).await?;
        self.press("form[action='/login'] button[type=submit]")
            .await?;
        self.until_within(
            "the sign-in answered",
            "return location.pathname !== '/login'",
            Duration::from_secs(30),
        )
        .await?;
        self.ready().await
    }

    /// The account of `email` made an admin, in the lane's database
    /// (`E2E_DIR`, config/e2e.yaml): a sign-up is a member, and the first
    /// admin comes from the command line, whose task's mail would leave
    /// with its process.
    pub async fn make_admin(&self, email: &str) -> WebDriverResult<()> {
        let database = format!("{}/app_e2e.sqlite", self.dir);
        let sql = format!(
            "UPDATE users SET role = 'admin' WHERE email = '{}'",
            email.replace('\'', "''")
        );
        let out = tokio::process::Command::new("sqlite3")
            .args([&database, &sql])
            .output()
            .await
            .map_err(|e| WebDriverError::RequestFailed(format!("sqlite3: {e}")))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(WebDriverError::RequestFailed(format!(
                "sqlite3: {}",
                String::from_utf8_lossy(&out.stderr)
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mailbox;

    #[test]
    fn a_tests_address_is_its_own() {
        // The first try's, outside nextest's retries.
        if std::env::var("NEXTEST_ATTEMPT").is_err() {
            assert_eq!(mailbox("ana"), "ana@example.com");
        }
    }
}
