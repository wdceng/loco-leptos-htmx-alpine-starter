use async_trait::async_trait;
use chrono::{Duration, offset::Local};
use loco_rs::{auth::jwt, hash, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;
use validator::ValidationError;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};
use crate::hashing;

pub const MAGIC_LINK_LENGTH: i8 = 32;
pub const MAGIC_LINK_EXPIRATION_MIN: i8 = 5;
pub const RESET_TOKEN_EXPIRATION_MIN: i64 = 30;
/// How long the welcome mail's verification link works; resending the mail
/// is the way to a new one.
pub const EMAIL_VERIFICATION_EXPIRATION_HOURS: i64 = 24;

/// An email address as it is stored and looked up: lowercase, so one
/// mailbox is one account in whatever case it's typed. Every save
/// (`before_save`) and every lookup by address goes through it.
#[must_use]
pub fn normalized_email(email: &str) -> String {
    email.to_lowercase()
}

/// A name as a paste brings it, made one line: a tab, a line or a
/// paragraph separator (U+2028, U+2029) a space, the direction marks a
/// chat or a contacts app wraps a name in (U+202A to U+202E, U+2066 to
/// U+2069) dropped, so none splits the welcome mail's line or turns the
/// words beside it around. A line break is left for [`one_line`] to refuse.
#[must_use]
pub fn one_line_text(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'))
        .map(|c| {
            if matches!(c, '\t' | '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// A name with no control character: no browser's field sends a line
/// break, and a forged post could use one to write lines of its own into
/// the welcome mail's text.
///
/// # Errors
/// A control character in it.
pub fn one_line(text: &str) -> Result<(), ValidationError> {
    if text.chars().any(char::is_control) {
        return Err(ValidationError::new("one_line"));
    }
    Ok(())
}

/// A password's length in characters: at least 8, as NIST SP 800-63B asks,
/// with no rules about which characters; at most 128, so nobody can make
/// every sign-in hash a megabyte.
pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;

/// Why `password` is refused, or `None` when it's long enough and not too
/// long. Checked before anything else, so the answer says nothing about
/// whether an account exists.
#[must_use]
pub fn password_problem(password: &str) -> Option<String> {
    let length = password.chars().count();
    (!(PASSWORD_MIN..=PASSWORD_MAX).contains(&length))
        .then(|| format!("The password must be {PASSWORD_MIN} to {PASSWORD_MAX} characters long."))
}

/// Why a new account's name or email is refused, or `None`: the rules
/// [`Validator`] holds, which `before_save` applies too. Checked first, like
/// [`password_problem`], so the answer says nothing about whether the email
/// is taken. The name as [`one_line_text`] makes it, as it is saved.
#[must_use]
pub fn name_or_email_problem(name: &str, email: &str) -> Option<String> {
    let details = Validator {
        name: one_line_text(name),
        email: email.to_owned(),
    };
    let errors = Validate::validate(&details).err()?;
    let fields = errors.field_errors();
    let first = ["name", "email"].iter().find_map(|field| {
        fields
            .get(*field)
            .and_then(|errors| errors.first())
            .and_then(|error| error.message.as_deref().map(str::to_owned))
    });
    Some(first.unwrap_or_else(|| errors.to_string()))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    // Capped: the name is chosen by whoever registers and goes into the
    // welcome mail, which is sent to any address they give. One line, so
    // it can't add lines to that mail's text.
    #[validate(
        length(min = 2, max = 100, message = "Name must be 2 to 100 characters long."),
        custom(function = "one_line", message = "Name must be one line.")
    )]
    pub name: String,
    #[validate(email(message = "invalid email"))]
    pub email: String,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;
        let mut this = self;
        if let ActiveValue::Set(email) = &this.email {
            this.email = ActiveValue::Set(normalized_email(email));
        }
        if insert {
            this.pid = ActiveValue::Set(Uuid::new_v4());
            // Loco's starter mints an API key for every account; no route
            // takes one yet (docs/security.md, "Accepted risks").
            this.api_key = ActiveValue::Set(format!("lo-{}", Uuid::new_v4()));
        }
        Ok(this)
    }
}

#[async_trait]
impl Authenticable for Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        Self::find_by_api_key(db, api_key).await
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, claims_key).await
    }
}

impl Model {
    /// finds a user by the provided email
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, normalized_email(email))
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided verification token
    ///
    /// The link works for [`EMAIL_VERIFICATION_EXPIRATION_HOURS`] while the
    /// account is unverified. A verified account's token still answers, so
    /// its old link says "already verified" (a magic link verifies and keeps
    /// it, [`ActiveModel::verified_by_magic_link`]).
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error, or the
    /// token expired
    pub async fn find_by_verification_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::EmailVerificationToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        let fresh = user.email_verification_sent_at.is_some_and(|sent_at| {
            sent_at + Duration::hours(EMAIL_VERIFICATION_EXPIRATION_HOURS) >= Local::now()
        });
        if user.email_verified_at.is_some() || fresh {
            Ok(user)
        } else {
            tracing::debug!(
                user_pid = user.pid.to_string(),
                "verification token expired for the user."
            );
            Err(ModelError::msg("verification token expired"))
        }
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                query::condition()
                    .eq(users::Column::MagicLinkToken, token)
                    .build(),
            )
            .one(db)
            .await?;

        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        if let Some(expired_at) = user.magic_link_expiration {
            if expired_at >= Local::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    token_expiration = expired_at.to_string(),
                    "magic token expired for the user."
                );
                Err(ModelError::msg("magic token expired"))
            }
        } else {
            tracing::error!(
                user_pid = user.pid.to_string(),
                "magic link expiration time not exists"
            );
            Err(ModelError::msg("expiration token not exists"))
        }
    }

    /// finds a user by the provided reset token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_reset_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ResetToken, token)
                    .build(),
            )
            .one(db)
            .await?;

        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        if let Some(sent_at) = user.reset_sent_at {
            if sent_at + Duration::minutes(RESET_TOKEN_EXPIRATION_MIN) >= Local::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    "reset token expired for the user."
                );
                Err(ModelError::msg("reset token expired"))
            }
        } else {
            tracing::error!(
                user_pid = user.pid.to_string(),
                "reset token sent time does not exist"
            );
            Err(ModelError::msg("reset token sent time not exists"))
        }
    }

    /// finds a user by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Pid, parse_uuid)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided api key
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Verifies whether the provided plain password matches the hashed
    /// password, on a blocking thread (`src/hashing.rs`).
    pub async fn verify_password(&self, password: &str) -> bool {
        hashing::verify(password, &self.password).await
    }

    /// Asynchronously creates a user with a password and saves it to the
    /// database.
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB
    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        // Hashed before the duplicate check, so a taken email costs one
        // hash like a new one, and the time says nothing either way.
        let password_hash = hashing::hash(&params.password)
            .await
            .map_err(|e| ModelError::Any(e.into()))?;
        let email = normalized_email(&params.email);

        let txn = db.begin().await?;

        if users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, &email)
                    .build(),
            )
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }

        let user = users::ActiveModel {
            email: ActiveValue::set(email),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(one_line_text(&params.name)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    /// Creates a JWT
    ///
    /// # Errors
    ///
    /// when could not convert user claims to jwt token
    pub fn generate_jwt(&self, secret: &str, expiration: u64) -> ModelResult<String> {
        jwt::JWT::new(secret)
            .generate_token(expiration, self.pid.to_string(), Map::new())
            .map_err(ModelError::from)
    }
}

impl ActiveModel {
    /// Sets the email verification information for the user and
    /// updates it in the database.
    ///
    /// This method is used to record the timestamp when the email verification
    /// was sent and generate a unique verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        self.email_verification_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.email_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Sets the information for a reset password request,
    /// generates a unique reset password token, and updates it in the
    /// database.
    ///
    /// This method records the timestamp when the reset password token is sent
    /// and generates a unique token for the user.
    ///
    /// # Arguments
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.reset_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Records the verification time when a user verifies their
    /// email and updates it in the database.
    ///
    /// This method sets the timestamp when the user successfully verifies their
    /// email.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::set(Some(Local::now().into()));
        // Invalidate the verification token so it cannot be replayed after use.
        self.email_verification_token = ActiveValue::Set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Marks the email verified by a followed magic link, which reached the
    /// same inbox. Unlike [`Self::verified`] it keeps the verification
    /// token, so the welcome mail's link, followed later, still answers
    /// ("already verified") instead of "invalid token".
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified_by_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::set(Some(Local::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Resets the current user password with a new password and
    /// updates it in the database.
    ///
    /// This method hashes the provided password and sets it as the new password
    /// for the user. A magic link still pending ends with it: a reset closes
    /// every way in that was opened before it.
    ///
    /// # Errors
    ///
    /// when has DB query error or could not hashed the given password
    pub async fn reset_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        self.password = ActiveValue::set(
            hashing::hash(password)
                .await
                .map_err(|e| ModelError::Any(e.into()))?,
        );
        self.reset_token = ActiveValue::Set(None);
        self.reset_sent_at = ActiveValue::Set(None);
        self.magic_link_token = ActiveValue::Set(None);
        self.magic_link_expiration = ActiveValue::Set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Creates a magic link token for passwordless authentication.
    ///
    /// Generates a random token with a specified length and sets an expiration time
    /// for the magic link. This method is used to initiate the magic link authentication flow.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn create_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let random_str = hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        self.magic_link_token = ActiveValue::set(Some(random_str));
        self.magic_link_expiration = ActiveValue::set(Some(expired.into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Verifies and invalidates the magic link after successful authentication.
    ///
    /// Clears the magic link token and expiration time after the user has
    /// successfully authenticated using the magic link.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn clear_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.magic_link_token = ActiveValue::set(None);
        self.magic_link_expiration = ActiveValue::set(None);
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A paste's separators made spaces and its direction marks dropped,
    /// the name then one line; a line break left for `one_line` to refuse.
    #[test]
    fn a_pasted_name_is_made_one_line() {
        let pasted = "\u{2068}Ana\u{2069}\u{2028}Anić\tMarić\u{2029}\u{202E}Jr";
        let made = one_line_text(pasted);
        assert_eq!(made, "Ana Anić Marić Jr");
        assert!(one_line(&made).is_ok());
        assert_eq!(one_line_text("Ana\nAnić"), "Ana\nAnić");
        for refused in ["Ana\nAnić", "Ana\r\nAnić", "Ana\u{7}", "Ana\u{85}"] {
            assert!(one_line(refused).is_err(), "{refused:?}");
        }
    }

    /// The rule as the forms see it: a name of two lines refused with its
    /// reason, a pasted one taken.
    #[test]
    fn a_name_of_two_lines_is_refused_with_its_reason() {
        assert_eq!(
            name_or_email_problem("Ana\nVerify at https://evil.example", "ana@example.com")
                .as_deref(),
            Some("Name must be one line.")
        );
        assert_eq!(
            name_or_email_problem("\u{2068}Ana\u{2069}\u{2028}Anić", "ana@example.com"),
            None
        );
    }
}
