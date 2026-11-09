//! What happened at the sign-in (migration/src, `auth_events`): a row an
//! event, the account's, and when. Nothing typed is kept, no address and no
//! IP. As an islands app on this stack keeps them.
//!
//! The wait after wrong codes reads it: a sign-in's own tries
//! (`sign_ins::CODE_TRIES`) end with the sign-in, and a right password
//! opened a new one with fresh tries, so someone with an admin's password
//! could guess on and on, one IP's bucket at a time. Now the wrong codes
//! count across sign-ins, from the account's last complete sign-in or new
//! password (NIST SP 800-63B-4: a new code, or a new sign-in, never starts
//! the count again). After [`FREE_TRIES`] the account waits before a code is
//! checked or mailed, a second, doubling with each wrong one up to
//! [`LONGEST_WAIT`] ([`wait`]): under 300 guesses a day. Never a lock: the
//! wait runs out by itself, a new password starts the count again, and only
//! someone with the password reaches the code, so a stranger holds up no
//! one. At the [`FREE_TRIES`]th the person is told by mail
//! (controllers/login.rs).
//!
//! Wrong passwords are not counted per account: an islands app on this
//! stack does, letting a browser the account signed in from skip that wait,
//! and without such browsers a stranger's guesses would hold up the
//! person's own sign-in. The rows go after [`KEEP_DAYS`] ([`purge`], at
//! every start) and with the account.

use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

/// How long a row is kept, deleted at every start ([`purge`]): half a year.
pub const KEEP_DAYS: i64 = 180;

/// Wrong codes in a row an account takes before it waits: a typo or two.
pub const FREE_TRIES: i64 = 5;

/// The longest wait, in seconds, reached at the 14th wrong code in a row: a
/// guess an account every five minutes at most.
pub const LONGEST_WAIT: i64 = 300;

/// What happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A second step's code wrong.
    CodeWrong,
    /// The second step's tries spent, the sign-in ended.
    CodeSpent,
    /// A session opened: the password, or the password and its code.
    SignedIn,
    /// A password set through the mailed link.
    PasswordSet,
    /// A password changed on the account page.
    PasswordChanged,
}

impl Event {
    /// The row's value.
    #[must_use]
    pub const fn value(self) -> &'static str {
        match self {
            Self::CodeWrong => "code_wrong",
            Self::CodeSpent => "code_spent",
            Self::SignedIn => "signed_in",
            Self::PasswordSet => "password_set",
            Self::PasswordChanged => "password_changed",
        }
    }
}

/// How long an account waits after `failures` wrong codes in a row:
/// nothing below [`FREE_TRIES`], then a second, doubling with each one
/// more, up to [`LONGEST_WAIT`].
#[must_use]
pub fn wait(failures: i64) -> Duration {
    if failures < FREE_TRIES {
        return Duration::zero();
    }
    let doublings = u32::try_from(failures - FREE_TRIES).unwrap_or(u32::MAX);
    let seconds = 1_i64
        .checked_shl(doublings)
        .filter(|&seconds| seconds > 0 && seconds < LONGEST_WAIT)
        .unwrap_or(LONGEST_WAIT);
    Duration::seconds(seconds)
}

/// The whole seconds left of the wait `failures` ask, the last at `last`,
/// at `now`, rounded up; none once it is over.
fn left(failures: i64, last: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Option<i64> {
    let millis = (last? + wait(failures) - now).num_milliseconds();
    (millis > 0).then(|| (millis + 999) / 1000)
}

/// The account's wrong codes since its last complete sign-in or new
/// password: how many, and when the last was.
async fn streak(pool: &SqlitePool, user_id: i64) -> sqlx::Result<(i64, Option<DateTime<Utc>>)> {
    let (wrong, spent) = (Event::CodeWrong.value(), Event::CodeSpent.value());
    let (signed_in, set, changed) = (
        Event::SignedIn.value(),
        Event::PasswordSet.value(),
        Event::PasswordChanged.value(),
    );
    let streak = sqlx::query!(
        r#"SELECT count(*) AS "failures!: i64", max(at) AS "last: DateTime<Utc>"
           FROM auth_events
           WHERE user_id = ? AND event IN (?, ?) AND id > COALESCE((
               SELECT max(id) FROM auth_events
               WHERE user_id = ? AND event IN (?, ?, ?)), 0)"#,
        user_id,
        wrong,
        spent,
        user_id,
        signed_in,
        set,
        changed,
    )
    .fetch_one(pool)
    .await?;
    Ok((streak.failures, streak.last))
}

/// The seconds the account whose row is `user_id` waits at `now` before a
/// code is checked or mailed; none where it does not wait.
///
/// # Errors
/// The database could not be read.
pub async fn code_waiting(
    pool: &SqlitePool,
    user_id: i64,
    now: DateTime<Utc>,
) -> sqlx::Result<Option<i64>> {
    let (failures, last) = streak(pool, user_id).await?;
    Ok(left(failures, last, now))
}

/// How many wrong codes in a row the account has: the person is told at the
/// [`FREE_TRIES`]th (controllers/login.rs).
///
/// # Errors
/// The database could not be read.
pub async fn code_failures(pool: &SqlitePool, user_id: i64) -> sqlx::Result<i64> {
    Ok(streak(pool, user_id).await?.0)
}

/// `event` written for the account whose row is `user_id`, now.
///
/// # Errors
/// The database refused the write.
pub async fn record<'e>(
    db: impl sqlx::SqliteExecutor<'e>,
    user_id: i64,
    event: Event,
) -> sqlx::Result<()> {
    let value = event.value();
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO auth_events (user_id, event, at) VALUES (?, ?, ?)",
        user_id,
        value,
        now,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The rows older than [`KEEP_DAYS`] at `now` deleted: how many.
///
/// # Errors
/// The database refused the delete.
pub async fn purge(pool: &SqlitePool, now: DateTime<Utc>) -> sqlx::Result<u64> {
    let before = now - Duration::days(KEEP_DAYS);
    let done = sqlx::query!("DELETE FROM auth_events WHERE at < ?", before)
        .execute(pool)
        .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_has_a_value_of_its_own() {
        let all = [
            Event::CodeWrong,
            Event::CodeSpent,
            Event::SignedIn,
            Event::PasswordSet,
            Event::PasswordChanged,
        ];
        let mut values: Vec<&str> = all.iter().map(|event| event.value()).collect();
        values.sort_unstable();
        values.dedup();
        assert_eq!(values.len(), all.len());
    }

    #[test]
    fn the_wait_starts_at_the_free_tries_and_doubles_to_its_longest() {
        assert_eq!(wait(FREE_TRIES - 1), Duration::zero());
        assert_eq!(wait(FREE_TRIES), Duration::seconds(1));
        assert_eq!(wait(FREE_TRIES + 1), Duration::seconds(2));
        assert_eq!(wait(FREE_TRIES + 8), Duration::seconds(256));
        assert_eq!(wait(FREE_TRIES + 9), Duration::seconds(LONGEST_WAIT));
        assert_eq!(wait(i64::MAX), Duration::seconds(LONGEST_WAIT));
    }

    #[test]
    fn the_wait_left_is_whole_seconds_rounded_up() {
        let now = Utc::now();
        let last = now - Duration::milliseconds(500);
        assert_eq!(left(FREE_TRIES + 1, Some(last), now), Some(2));
        assert_eq!(left(FREE_TRIES, Some(last), now), Some(1));
        assert_eq!(left(FREE_TRIES - 1, Some(last), now), None);
        assert_eq!(left(FREE_TRIES, None, now), None);
    }
}
