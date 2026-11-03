//! A sign-in whose password passed and whose second step is still due
//! (controllers/login.rs): a row of `sign_ins` (migration/src), its secret
//! in the `signin` cookie and the six digits mailed to the person, both
//! kept as digests, the code's together with the cookie's secret, so a copy
//! of the database reads neither. It is no session: the pages know nobody
//! until the code is right and a session is opened. As an islands app on
//! this stack keeps them.
//!
//! A code lasts [`CODE_MINUTES`]; a sign-in has [`CODE_TRIES`] tries,
//! counted across the codes sent again, after which the password is asked
//! anew. A new code goes at most once a minute ([`RESEND_SECONDS`]),
//! decided by the database in its one write.

use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

use crate::{cookies, secret, sql};

/// The cookie that holds the sign-in's secret.
pub const COOKIE: &str = "signin";

/// How long a code lasts, and the sign-in with it.
pub const CODE_MINUTES: i64 = 10;

/// How many codes a sign-in may try before the password is asked again.
pub const CODE_TRIES: i64 = 5;

/// The least time between two codes mailed.
pub const RESEND_SECONDS: i64 = 60;

/// How many digits a code has.
const DIGITS: usize = 6;

/// The step a sign-in waits for: the mailed code.
pub const BY_MAIL: &str = "email";

/// A sign-in waiting for its second step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignIn {
    pub id: i64,
    pub user_id: i64,
    pub expires_at: DateTime<Utc>,
    /// When the last code went out: the pause before another.
    pub code_sent_at: DateTime<Utc>,
}

/// What a code typed was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checked {
    Right,
    /// Wrong, `left` tries left.
    Wrong {
        left: i64,
    },
    /// No tries left, or the sign-in ran out: the password again.
    Spent,
}

impl SignIn {
    /// A sign-in opened for the user whose row is `user_id`: its secret,
    /// for the cookie, the code, for the mail, and its row. The user's
    /// other sign-ins still waiting are closed, as are the ones that ran
    /// out, so the table holds what is live.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn open(pool: &SqlitePool, user_id: i64) -> sqlx::Result<(String, String, Self)> {
        let secret = secret::fresh();
        let token_hash = secret::digest(&secret);
        let code = fresh_code();
        let code_hash = code_digest(&secret, &code);
        let now = Utc::now();
        let expires_at = now + Duration::minutes(CODE_MINUTES);
        let mut tx = sql::write(pool).await?;
        sqlx::query!(
            "DELETE FROM sign_ins WHERE expires_at <= ? OR user_id = ?",
            now,
            user_id,
        )
        .execute(&mut *tx)
        .await?;
        let sign_in = sqlx::query_as!(
            SignIn,
            r#"INSERT INTO sign_ins (user_id, token_hash, code_hash, expires_at, code_sent_at, method)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING id AS "id!", user_id,
                         expires_at AS "expires_at: DateTime<Utc>",
                         code_sent_at AS "code_sent_at: DateTime<Utc>""#,
            user_id,
            token_hash,
            code_hash,
            expires_at,
            now,
            BY_MAIL,
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((secret, code, sign_in))
    }

    /// The live sign-in whose secret is `secret`; none for a secret without
    /// that shape, one nobody has or one that ran out.
    ///
    /// # Errors
    /// The database could not be read.
    pub async fn find(pool: &SqlitePool, secret: &str) -> sqlx::Result<Option<Self>> {
        if !secret::is_secret(secret) {
            return Ok(None);
        }
        let token_hash = secret::digest(secret);
        let now = Utc::now();
        sqlx::query_as!(
            SignIn,
            r#"SELECT id AS "id!", user_id,
                      expires_at AS "expires_at: DateTime<Utc>",
                      code_sent_at AS "code_sent_at: DateTime<Utc>"
               FROM sign_ins WHERE token_hash = ? AND expires_at > ?"#,
            token_hash,
            now,
        )
        .fetch_optional(pool)
        .await
    }

    /// The mailed `code`, typed for the sign-in whose cookie holds `secret`,
    /// checked: a try counted first, in the database's one write, so tries
    /// posted together are counted one by one, then the digests compared.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn check(
        &self,
        pool: &SqlitePool,
        secret: &str,
        code: &str,
    ) -> sqlx::Result<Checked> {
        let now = Utc::now();
        let counted = sqlx::query!(
            r#"UPDATE sign_ins SET tries = tries + 1, updated_at = ?
               WHERE id = ? AND tries < ? AND expires_at > ?
               RETURNING code_hash, tries AS "tries!: i64""#,
            now,
            self.id,
            CODE_TRIES,
            now,
        )
        .fetch_optional(pool)
        .await?;
        let Some(counted) = counted else {
            return Ok(Checked::Spent);
        };
        let typed: String = code.chars().filter(char::is_ascii_digit).collect();
        if typed.len() == DIGITS && secret::same(&code_digest(secret, &typed), &counted.code_hash) {
            return Ok(Checked::Right);
        }
        let left = CODE_TRIES - counted.tries;
        Ok(if left > 0 {
            Checked::Wrong { left }
        } else {
            Checked::Spent
        })
    }

    /// A new code for the sign-in, to be mailed, lasting [`CODE_MINUTES`]
    /// from now, the tries kept. None within [`RESEND_SECONDS`] of the
    /// last, decided by the database in the one write.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn renew_code(
        &self,
        pool: &SqlitePool,
        secret: &str,
    ) -> sqlx::Result<Option<String>> {
        let code = fresh_code();
        let code_hash = code_digest(secret, &code);
        let now = Utc::now();
        let expires_at = now + Duration::minutes(CODE_MINUTES);
        let before = now - Duration::seconds(RESEND_SECONDS);
        let renewed = sqlx::query!(
            "UPDATE sign_ins SET code_hash = ?, expires_at = ?, code_sent_at = ?, updated_at = ?
             WHERE id = ? AND code_sent_at <= ? AND expires_at > ?",
            code_hash,
            expires_at,
            now,
            now,
            self.id,
            before,
            now,
        )
        .execute(pool)
        .await?
        .rows_affected();
        Ok((renewed > 0).then_some(code))
    }

    /// The whole seconds left of the pause before another code, rounded
    /// up; none once it is over.
    #[must_use]
    pub fn code_wait(&self) -> Option<i64> {
        let left = self.code_sent_at + Duration::seconds(RESEND_SECONDS) - Utc::now();
        let millis = left.num_milliseconds();
        (millis > 0).then(|| (millis + 999) / 1000)
    }

    /// The sign-in closed: the code was right, or spent.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn close(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM sign_ins WHERE id = ?", self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Every sign-in of the user closed: a new password, the account gone.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn close_all(pool: &SqlitePool, user_id: i64) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM sign_ins WHERE user_id = ?", user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// The secret a request's `signin` cookie carries, if it has one.
    #[must_use]
    pub fn secret_from(headers: &HeaderMap) -> Option<String> {
        cookies::value(headers, COOKIE)
    }

    /// The cookie that carries the sign-in's secret, as long as its code
    /// lasts.
    #[must_use]
    pub fn cookie(secret: &str, secure: bool) -> String {
        cookies::set_cookie(COOKIE, secret, CODE_MINUTES * 60, secure)
    }

    /// The cookie taken away.
    #[must_use]
    pub fn cookie_deleted(secure: bool) -> String {
        cookies::set_cookie(COOKIE, "", 0, secure)
    }
}

/// Six random digits, each drawn evenly.
fn fresh_code() -> String {
    secret::drawn(b"0123456789", DIGITS)
}

/// What the row keeps of a code: its digest together with the sign-in's
/// secret, which only the cookie holds, since six digits alone are a
/// million guesses from their digest.
fn code_digest(secret: &str, code: &str) -> String {
    secret::digest(&format!("{secret}:{code}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_six_digits_and_two_differ() {
        let codes: Vec<String> = (0..20).map(|_| fresh_code()).collect();
        for code in &codes {
            assert_eq!(code.len(), DIGITS, "{code}");
            assert!(code.bytes().all(|byte| byte.is_ascii_digit()), "{code}");
        }
        assert!(codes.iter().any(|code| code != &codes[0]), "{codes:?}");
    }

    #[test]
    fn a_codes_digest_needs_the_cookies_secret() {
        let one = code_digest(&"a".repeat(32), "123456");
        assert_eq!(one, code_digest(&"a".repeat(32), "123456"));
        assert_ne!(one, code_digest(&"b".repeat(32), "123456"));
        assert_ne!(one, secret::digest("123456"));
    }

    #[test]
    fn the_wait_is_whole_seconds_rounded_up() {
        let mut sign_in = SignIn {
            id: 1,
            user_id: 1,
            expires_at: Utc::now() + Duration::minutes(CODE_MINUTES),
            code_sent_at: Utc::now() - Duration::seconds(10),
        };
        assert_eq!(sign_in.code_wait(), Some(50));
        sign_in.code_sent_at = Utc::now() - Duration::seconds(61);
        assert_eq!(sign_in.code_wait(), None);
    }
}
