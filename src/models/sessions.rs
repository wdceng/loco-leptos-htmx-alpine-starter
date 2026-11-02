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

use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

use crate::{cookies, secret};

/// The cookie's name.
pub const COOKIE: &str = "session";

/// How long a session lasts, from the sign-in: a working day.
pub const SESSION_HOURS: i64 = 12;

/// A session as its row keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    /// A session opened for the user whose row is `user_id`: its secret,
    /// for the cookie, and its row. Sessions that ran out are swept on the
    /// way, so the table never grows past what is live.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn open(pool: &SqlitePool, user_id: i64) -> sqlx::Result<(String, Self)> {
        let secret = secret::fresh();
        let digest = secret::digest(&secret);
        let now = Utc::now();
        let expires_at = now + Duration::hours(SESSION_HOURS);
        let mut tx = pool.begin().await?;
        sqlx::query!("DELETE FROM sessions WHERE expires_at <= ?", now)
            .execute(&mut *tx)
            .await?;
        let session = sqlx::query_as!(
            Session,
            r#"INSERT INTO sessions (user_id, token_hash, expires_at) VALUES (?, ?, ?)
               RETURNING id AS "id!", user_id, expires_at AS "expires_at: DateTime<Utc>""#,
            user_id,
            digest,
            expires_at,
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
            r#"SELECT id, user_id, expires_at AS "expires_at: DateTime<Utc>"
               FROM sessions WHERE token_hash = ? AND expires_at > ?"#,
            digest,
            now,
        )
        .fetch_optional(pool)
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
