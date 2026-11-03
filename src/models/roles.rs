//! What a user may do (`users.role`, migration/src): [`Role::Member`],
//! everyone who signs up, or [`Role::Admin`], given from the command line
//! (`user:create … role:admin`, src/tasks/user_create.rs). A page for one
//! role takes its extractor (`Admin`, src/controllers/signed.rs). An app
//! with more roles adds them here, each with its word in the column. Read
//! and written with the app's own SQLx queries: Loco's users entity, as
//! generated, leaves the column to them.

use sqlx::SqlitePool;

/// A user's role, by the word the column keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Member,
    Admin,
}

impl Role {
    /// The word the column keeps.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Admin => "admin",
        }
    }

    /// The role a word names; none for a word the app doesn't know.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        [Self::Member, Self::Admin]
            .into_iter()
            .find(|role| role.as_str() == word)
    }

    /// The role of the user whose row is `user_id`. A word the app doesn't
    /// know, or no such user, is a member's: the least a role can do.
    ///
    /// # Errors
    /// The database could not be read.
    pub async fn of(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Self> {
        let word = sqlx::query_scalar!("SELECT role FROM users WHERE id = ?", user_id)
            .fetch_optional(pool)
            .await?;
        Ok(word
            .as_deref()
            .and_then(Self::named)
            .unwrap_or(Self::Member))
    }

    /// The user whose row is `user_id` given this role.
    ///
    /// # Errors
    /// The database refused the write.
    pub async fn give(self, pool: &SqlitePool, user_id: i64) -> sqlx::Result<()> {
        let word = self.as_str();
        sqlx::query!("UPDATE users SET role = ? WHERE id = ?", word, user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_role_is_its_word_and_back() {
        for role in [Role::Member, Role::Admin] {
            assert_eq!(Role::named(role.as_str()), Some(role));
        }
        assert_eq!(Role::named("owner"), None);
    }
}
