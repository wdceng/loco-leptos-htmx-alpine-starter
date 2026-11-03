//! Two boot checks for what the config's `get_env` can't judge. A
//! `get_env` without a default refuses the boot when a variable is missing,
//! but a variable that is set and still wrong goes through.
//!
//! - [`refuse_placeholders`], in production: an SMTP host, user or
//!   password that is empty or still reads `replace_me` (the placeholder
//!   `docs/production.md` writes into `secrets.production.env`) refuses the
//!   boot, and so does `settings.security.contact` still at the template's
//!   `example.com`: `security.txt` would send reports nowhere; and so do
//!   the app's name and description still the template's `TODO`. Dev-server
//!   and staging keep their mail placeholders on purpose: they boot without
//!   a mail account, and their mail fails to send. No JWT secret is
//!   checked: the configs carry none (src/app.rs, Loco's auth API
//!   unmounted).
//! - [`spawn_smtp_login`], everywhere deployed: logs in to the SMTP
//!   server once, in the background, and sends nothing. A failure is a
//!   warning in the journal, not a refusal: an outage at the mail provider
//!   must not keep the site down. Loco keeps its own transport's type
//!   private, so [`transport`] builds one from the same `mailer.smtp`
//!   block, the same way Loco's `EmailSender::smtp` does (loco-rs 1.2).

use lettre::{
    AsyncSmtpTransport, Tokio1Executor,
    transport::smtp::{authentication::Credentials, extension::ClientId},
};
use loco_rs::{
    Error, Result,
    app::AppContext,
    config::{MailerTls, SmtpMailer},
    environment::Environment,
};
use tracing::{info, warn};

use crate::{
    app::stored,
    controllers::home::DESCRIPTION,
    settings::{Settings, is_local},
    views::layout::{APP_NAME, SHORT_NAME},
};

/// What `docs/production.md` writes for a value still to be filled in.
pub const PLACEHOLDER: &str = "replace_me";

/// The marker in a value of the app's own the template leaves to each app
/// (its name, its description): "Start a new project", docs/development.md.
pub const TODO: &str = "TODO";

/// The secrets a deployed boot reads, by the variable that sets them: the
/// mail account's.
fn secrets(ctx: &AppContext) -> Vec<(&'static str, String)> {
    let mut values = Vec::new();
    let smtp = ctx
        .config
        .mailer
        .as_ref()
        .and_then(|mailer| mailer.smtp.as_ref())
        .filter(|smtp| smtp.enable);
    if let Some(smtp) = smtp {
        values.push(("MAILER_HOST", smtp.host.clone()));
        if let Some(auth) = &smtp.auth {
            values.push(("MAILER_USER", auth.user.clone()));
            values.push(("MAILER_PASSWORD", auth.password.clone()));
        }
    }
    values
}

/// The names whose value is empty or still the placeholder.
fn not_filled_in(values: &[(&'static str, String)]) -> Vec<&'static str> {
    values
        .iter()
        .filter(|(_, value)| value.trim().is_empty() || value.contains(PLACEHOLDER))
        .map(|(name, _)| *name)
        .collect()
}

/// Refuses production's boot with a mail secret that is empty or still
/// `replace_me`, with the template's security contact, or with the app's
/// name or description still `TODO`. Every other environment passes.
///
/// # Errors
/// Production with `MAILER_HOST`, `MAILER_USER` or `MAILER_PASSWORD` not
/// filled in, the template's security contact, or `APP_NAME`, `SHORT_NAME` or
/// `DESCRIPTION` still `TODO`.
pub fn refuse_placeholders(ctx: &AppContext) -> Result<()> {
    let env = &ctx.environment;
    if !matches!(env, Environment::Production) {
        return Ok(());
    }
    let missing = not_filled_in(&secrets(ctx));
    if !missing.is_empty() {
        return Err(Error::Message(format!(
            "{env} won't start: {} empty or still `{PLACEHOLDER}`. Fill them in secrets.{env}.env and deploy it (docs/{env}.md)",
            missing.join(", ")
        )));
    }
    let settings: Settings = stored(ctx)?;
    if contact_is_a_placeholder(&settings.security.contact) {
        return Err(Error::Message(format!(
            "production won't start: settings.security.contact is still `{}`. Put the real address in config/production.yaml (docs/security.md)",
            settings.security.contact
        )));
    }
    let unnamed: Vec<&str> = [
        ("APP_NAME", APP_NAME),
        ("SHORT_NAME", SHORT_NAME),
        ("DESCRIPTION", DESCRIPTION),
    ]
    .into_iter()
    .filter(|(_, value)| still_todo(value))
    .map(|(name, _)| name)
    .collect();
    if !unnamed.is_empty() {
        return Err(Error::Message(format!(
            "production won't start: {} still `{TODO}`. Fill in the app's own (\"Start a new project\", docs/development.md)",
            unnamed.join(", ")
        )));
    }
    Ok(())
}

/// Whether the security contact is still the template's.
fn contact_is_a_placeholder(contact: &str) -> bool {
    contact.contains("example.com") || contact.contains(PLACEHOLDER)
}

/// Whether a value of the app's own is still the template's: its `TODO`
/// marker ("Start a new project", docs/development.md).
fn still_todo(value: &str) -> bool {
    value.contains(TODO)
}

/// An SMTP transport for `smtp`, built the way Loco's
/// `EmailSender::smtp` builds the one it sends with: the same TLS modes,
/// port, credentials and hello name.
fn transport(smtp: &SmtpMailer) -> std::result::Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let host = &smtp.host;
    let mut builder = match smtp.tls_mode() {
        MailerTls::Starttls => {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host).map_err(|e| e.to_string())?
        }
        MailerTls::Implicit => {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(|e| e.to_string())?
        }
        MailerTls::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
    }
    .port(smtp.port);
    if let Some(auth) = &smtp.auth {
        builder = builder.credentials(Credentials::new(auth.user.clone(), auth.password.clone()));
    }
    if let Some(hello_name) = &smtp.hello_name {
        builder = builder.hello_name(ClientId::Domain(hello_name.clone()));
    }
    Ok(builder.build())
}

/// Starts the one-time SMTP login in the background, when deployed and
/// mail goes over SMTP. Development (Mailpit may not run) and test (the
/// stub mailer) are skipped.
pub fn spawn_smtp_login(ctx: &AppContext) {
    if is_local(&ctx.environment) {
        return;
    }
    let smtp = ctx
        .config
        .mailer
        .as_ref()
        .filter(|mailer| !mailer.stub)
        .and_then(|mailer| mailer.smtp.as_ref())
        .filter(|smtp| smtp.enable);
    let Some(smtp) = smtp else {
        info!("smtp login check skipped: mail isn't sent over SMTP");
        return;
    };
    let transport = match transport(smtp) {
        Ok(transport) => transport,
        Err(err) => {
            warn!(error = %err, "smtp login check: the SMTP settings don't form a transport");
            return;
        }
    };
    tokio::spawn(async move {
        // Connects, starts TLS and logs in with MAILER_USER and
        // MAILER_PASSWORD, then sends NOOP. No mail goes out.
        match transport.test_connection().await {
            Ok(true) => info!("smtp login check: the mail server accepted the login"),
            Ok(false) => warn!("smtp login check: logged in, but the server didn't answer NOOP"),
            Err(err) => warn!(
                error = %err,
                "smtp login check failed: no mail will go out until MAILER_* in secrets.env is fixed"
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
        pairs.iter().map(|(n, v)| (*n, (*v).to_string())).collect()
    }

    #[test]
    fn real_values_pass() {
        let filled = values(&[
            ("MAILER_HOST", "smtp.example.net"),
            ("MAILER_USER", "noreply@example.net"),
            ("MAILER_PASSWORD", "an app password"),
        ]);
        assert!(not_filled_in(&filled).is_empty());
    }

    #[test]
    fn empty_and_placeholder_values_are_named() {
        let mixed = values(&[
            ("MAILER_HOST", "replace_me"),
            ("MAILER_USER", "   "),
            ("MAILER_PASSWORD", ""),
        ]);
        assert_eq!(
            not_filled_in(&mixed),
            vec!["MAILER_HOST", "MAILER_USER", "MAILER_PASSWORD"]
        );
    }

    #[test]
    fn the_templates_security_contact_is_a_placeholder() {
        assert!(contact_is_a_placeholder("mailto:security@example.com"));
        assert!(contact_is_a_placeholder("mailto:replace_me"));
        assert!(!contact_is_a_placeholder("mailto:security@shop.hr"));
        assert!(!contact_is_a_placeholder("https://shop.hr/security"));
    }

    #[test]
    fn a_value_still_todo_is_the_templates() {
        assert!(still_todo("TODO App"));
        assert!(still_todo("TODO: what the site is, in one sentence."));
        assert!(!still_todo("Ljekarna Štefan"));
    }
}
