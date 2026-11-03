//! A signed-in user's session (migration/src, `sessions`): a row per
//! sign-in, its secret handed to the browser in the `session` cookie and
//! kept here as its digest (src/secret.rs), with when it runs out. Sessions
//! live in the database rather than in a signed token, in place of Loco's
//! JWT (docs/decisions.md): signing out deletes the row and a password
//! reset deletes all the user's rows, each at once, where a token stays
//! good until it expires. One lookup per request, as Loco's own
//! `JWTWithUser` reads the user. As an islands app on this stack keeps its
//! sessions.
//!
//! The cookie (src/cookies.rs): `HttpOnly`, so no script reads it,
//! `SameSite=Lax`, `Secure` where the site is served over https, for
//! [`SESSION_HOURS`]; then a sign-in again.
//!
//! The account page lists a user's sessions as their browsers: what each
//! was opened in (`browser`, [`label_from`]) and when it was last used
//! (`seen_at`, kept to [`SEEN_EVERY_MINUTES`]). And a session keeps when
//! its person last proved who they are, `proven_at`: the sign-in, or a
//! password typed since ([`Session::prove`]). A sensitive step asks nothing
//! within [`PROVEN_MINUTES`] of it, the password past them, so a session
//! taken from an unlocked computer an hour later deletes no account
//! (GitHub's sudo mode, OWASP's reauthentication). As an islands app on
//! this stack keeps them.

use axum::http::{HeaderMap, header::USER_AGENT};
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

use crate::{cookies, secret, sql};

/// The cookie's name.
pub const COOKIE: &str = "session";

/// How long a session lasts, from the sign-in: a working day.
pub const SESSION_HOURS: i64 = 12;

/// How long a proof of the person holds in a session (the module doc).
pub const PROVEN_MINUTES: i64 = 15;

/// How often a session's last use is written: once in a while, not on
/// every request.
pub const SEEN_EVERY_MINUTES: i64 = 5;

/// A session as its row keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub expires_at: DateTime<Utc>,
    /// When it was last used, to [`SEEN_EVERY_MINUTES`].
    pub seen_at: DateTime<Utc>,
    /// The sign-in, or the last password typed in the session.
    pub proven_at: DateTime<Utc>,
}

/// A session as the account page shows it: one of the user's browsers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Browser {
    pub id: i64,
    /// What it was opened in ([`label_from`]).
    pub browser: String,
    /// The sign-in.
    pub created_at: DateTime<Utc>,
    pub seen_at: DateTime<Utc>,
}

impl Session {
    /// A session opened for the user whose row is `user_id`, in `browser`
    /// ([`label_of`]): its secret, for the cookie, and its row, proven and
    /// seen now. Sessions that ran out are swept on the way, so the table
    /// never grows past what is live.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn open(
        pool: &SqlitePool,
        user_id: i64,
        browser: &str,
    ) -> sqlx::Result<(String, Self)> {
        let secret = secret::fresh();
        let digest = secret::digest(&secret);
        let now = Utc::now();
        let expires_at = now + Duration::hours(SESSION_HOURS);
        let mut tx = sql::write(pool).await?;
        sqlx::query!("DELETE FROM sessions WHERE expires_at <= ?", now)
            .execute(&mut *tx)
            .await?;
        let session = sqlx::query_as!(
            Session,
            r#"INSERT INTO sessions (user_id, token_hash, expires_at, browser, seen_at, proven_at)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING id AS "id!", user_id, expires_at AS "expires_at: DateTime<Utc>",
                 seen_at AS "seen_at: DateTime<Utc>", proven_at AS "proven_at: DateTime<Utc>""#,
            user_id,
            digest,
            expires_at,
            browser,
            now,
            now,
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((secret, session))
    }

    /// The live session whose secret is `secret`; none for a secret without
    /// that shape, one nobody has or one that ran out.
    ///
    /// # Errors
    /// The database could not be read.
    pub async fn find(pool: &SqlitePool, secret: &str) -> sqlx::Result<Option<Self>> {
        if !secret::is_secret(secret) {
            return Ok(None);
        }
        let digest = secret::digest(secret);
        let now = Utc::now();
        sqlx::query_as!(
            Session,
            r#"SELECT id, user_id, expires_at AS "expires_at: DateTime<Utc>",
                 seen_at AS "seen_at: DateTime<Utc>", proven_at AS "proven_at: DateTime<Utc>"
               FROM sessions WHERE token_hash = ? AND expires_at > ?"#,
            digest,
            now,
        )
        .fetch_optional(pool)
        .await
    }

    /// Whether the person proved who they are in this session within
    /// [`PROVEN_MINUTES`] (the module doc).
    #[must_use]
    pub fn proven(&self) -> bool {
        Utc::now() - self.proven_at < Duration::minutes(PROVEN_MINUTES)
    }

    /// The person proved who they are now: a password typed in the session.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn prove(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
        let now = Utc::now();
        sqlx::query!("UPDATE sessions SET proven_at = ? WHERE id = ?", now, id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// The session used now, written only once [`SEEN_EVERY_MINUTES`] have
    /// passed since it last was, so a request is no write.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn seen(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let now = Utc::now();
        if now - self.seen_at < Duration::minutes(SEEN_EVERY_MINUTES) {
            return Ok(());
        }
        sqlx::query!("UPDATE sessions SET seen_at = ? WHERE id = ?", now, self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// The user's live sessions as their browsers, the last used first.
    ///
    /// # Errors
    /// The database could not be read.
    pub async fn browsers(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<Browser>> {
        let now = Utc::now();
        sqlx::query_as!(
            Browser,
            r#"SELECT id, browser, created_at AS "created_at: DateTime<Utc>",
                 seen_at AS "seen_at: DateTime<Utc>"
               FROM sessions WHERE user_id = ? AND expires_at > ?
               ORDER BY seen_at DESC, id DESC"#,
            user_id,
            now,
        )
        .fetch_all(pool)
        .await
    }

    /// The session closed: signing out.
    ///
    /// # Errors
    /// The database refused the delete.
    pub async fn close(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE id = ?", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// One of the user's sessions closed, from the account page: whether
    /// there was such a session of theirs. Another user's id closes
    /// nothing.
    ///
    /// # Errors
    /// The database refused the delete.
    pub async fn close_of(pool: &SqlitePool, user_id: i64, id: i64) -> sqlx::Result<bool> {
        let closed = sqlx::query!(
            "DELETE FROM sessions WHERE id = ? AND user_id = ?",
            id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(closed.rows_affected() > 0)
    }

    /// Every session of the user but `kept` closed: signing out everywhere
    /// else, and a changed password.
    ///
    /// # Errors
    /// The database refused the delete.
    pub async fn close_others(pool: &SqlitePool, user_id: i64, kept: i64) -> sqlx::Result<()> {
        sqlx::query!(
            "DELETE FROM sessions WHERE user_id = ? AND id != ?",
            user_id,
            kept
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Every session of the user closed: a password reset.
    ///
    /// # Errors
    /// The database refused the delete.
    pub async fn close_all(pool: &SqlitePool, user_id: i64) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE user_id = ?", user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// How many sessions the user has: the tests' check that a change
    /// closed the right ones.
    ///
    /// # Errors
    /// The database could not be read.
    pub async fn count(pool: &SqlitePool, user_id: i64) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!: i64" FROM sessions WHERE user_id = ?"#,
            user_id,
        )
        .fetch_one(pool)
        .await
    }

    /// The secret a request's `session` cookie carries, if any.
    #[must_use]
    pub fn secret_from(headers: &HeaderMap) -> Option<String> {
        cookies::value(headers, COOKIE)
    }

    /// The `Set-Cookie` value that hands `secret` to the browser for
    /// [`SESSION_HOURS`]; `secure` where the site is served over https.
    #[must_use]
    pub fn cookie(secret: &str, secure: bool) -> String {
        cookies::set_cookie(COOKIE, secret, SESSION_HOURS * 60 * 60, secure)
    }

    /// The `Set-Cookie` value that takes the cookie away: signing out.
    #[must_use]
    pub fn cookie_deleted(secure: bool) -> String {
        cookies::set_cookie(COOKIE, "", 0, secure)
    }
}

/// What the browser that sent `headers` is called ([`label_from`]).
#[must_use]
pub fn label_of(headers: &HeaderMap) -> String {
    let agent = headers
        .get(USER_AGENT)
        .and_then(|agent| agent.to_str().ok())
        .unwrap_or_default();
    label_from(agent)
}

/// What a browser is called, from its `User-Agent`: the browser and the
/// system, "Chrome on Windows", "Safari on an iPhone", either alone where
/// only one is known, "An unknown browser" where neither is. The order
/// matters: Edge, Opera and Samsung's browser also say Chrome, and Chrome
/// also says Safari. As an islands app on this stack names them.
#[must_use]
pub fn label_from(user_agent: &str) -> String {
    const BROWSERS: [(&str, &str); 8] = [
        // `Edg/` on a desktop and Android, `EdgiOS` on an iPhone.
        ("Edg", "Edge"),
        ("OPR/", "Opera"),
        ("SamsungBrowser", "Samsung Internet"),
        ("Firefox/", "Firefox"),
        ("FxiOS", "Firefox"),
        ("CriOS", "Chrome"),
        ("Chrome/", "Chrome"),
        ("Safari/", "Safari"),
    ];
    const SYSTEMS: [(&str, &str); 6] = [
        ("iPhone", "an iPhone"),
        ("iPad", "an iPad"),
        ("Android", "Android"),
        ("Windows", "Windows"),
        ("Macintosh", "a Mac"),
        ("Linux", "Linux"),
    ];
    let browser = BROWSERS
        .iter()
        .find(|(mark, _)| user_agent.contains(mark))
        .map(|(_, name)| *name);
    let system = SYSTEMS
        .iter()
        .find(|(mark, _)| user_agent.contains(mark))
        .map(|(_, name)| *name);
    match (browser, system) {
        (Some(browser), Some(system)) => format!("{browser} on {system}"),
        (Some(browser), None) => browser.to_string(),
        (None, Some(system)) => format!("A browser on {system}"),
        (None, None) => "An unknown browser".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cookie_is_http_only_lax_and_a_working_day_long() {
        assert_eq!(
            Session::cookie("abc", true),
            "__Host-session=abc; Path=/; Max-Age=43200; HttpOnly; SameSite=Lax; Secure"
        );
        assert_eq!(
            Session::cookie_deleted(false),
            "session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"
        );
    }

    #[test]
    fn a_browser_is_named_by_its_user_agent() {
        for (agent, label) in [
            (
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:140.0) Gecko/20100101 Firefox/140.0",
                "Firefox on a Mac",
            ),
            (
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0",
                "Edge on Windows",
            ),
            (
                "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
                "Safari on an iPhone",
            ),
            (
                "Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Mobile Safari/537.36",
                "Chrome on Android",
            ),
            ("curl/8.7.1", "An unknown browser"),
        ] {
            assert_eq!(label_from(agent), label, "{agent}");
        }
    }

    #[test]
    fn the_secret_is_read_from_the_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            "other=1; session=abc123".parse().expect("a header"),
        );
        assert_eq!(Session::secret_from(&headers), Some("abc123".into()));
        assert_eq!(Session::secret_from(&HeaderMap::new()), None);
    }
}
