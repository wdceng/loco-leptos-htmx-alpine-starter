use crate::{
    app::stored,
    hashing,
    mailers::auth::AuthMailer,
    middleware::rate_limit::Bucket,
    models::{
        _entities::users,
        users::{LoginParams, RegisterParams, name_or_email_problem, password_problem},
    },
    settings::Settings,
    views::auth::{CurrentResponse, LoginResponse},
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// The bucket on the `/api/auth` routes, built and validated in
/// `after_context` (app.rs) from `settings.rate_limit.auth` and parked in
/// the shared store, because `Hooks::routes` cannot fail. `None` when the
/// limiter is off in the config.
#[derive(Clone)]
pub struct AuthLimit(pub Option<Bucket>);

#[derive(Debug, Deserialize, Serialize)]
pub struct ForgotParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResetParams {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResendVerificationParams {
    pub email: String,
}

/// Register function creates a new user with the given parameters and sends a
/// welcome email to the user
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    // The rules first, so the answer is the same whether the email is taken
    // or not.
    if let Some(problem) = password_problem(&params.password)
        .or_else(|| name_or_email_problem(&params.name, &params.email))
    {
        return bad_request(problem);
    }
    let res = users::Model::create_with_password(&ctx.db, &params).await;

    let user = match res {
        Ok(user) => user,
        Err(err) => {
            // Never the address: personal data, and a taken one is exactly
            // what the answer keeps to itself.
            tracing::info!(message = err.to_string(), "could not register user");
            return format::json(());
        }
    };

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;

    format::json(())
}

/// Verify register user. if the user not verified his email, he can't login to
/// the system.
#[debug_handler]
async fn verify(State(ctx): State<AppContext>, Path(token): Path<String>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_verification_token(&ctx.db, &token).await else {
        return unauthorized("invalid token");
    };

    if user.email_verified_at.is_some() {
        tracing::info!(pid = user.pid.to_string(), "user already verified");
    } else {
        let active_model = user.into_active_model();
        let user = active_model.verified(&ctx.db).await?;
        tracing::info!(pid = user.pid.to_string(), "user verified");
    }

    format::json(())
}

/// In case the user forgot his password  this endpoints generate a forgot token
/// and send email to the user. In case the email not found in our DB, we are
/// returning a valid request for security reasons (not exposing users DB
/// list).
#[debug_handler]
async fn forgot(
    State(ctx): State<AppContext>,
    Json(params): Json<ForgotParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return format::json(());
    };

    let user = user
        .into_active_model()
        .set_forgot_password_sent(&ctx.db)
        .await?;

    AuthMailer::forgot_password(&ctx, &user).await?;

    format::json(())
}

/// reset user password by the given parameters
#[debug_handler]
async fn reset(State(ctx): State<AppContext>, Json(params): Json<ResetParams>) -> Result<Response> {
    // First, so the answer says nothing about whether the token is valid.
    if let Some(problem) = password_problem(&params.password) {
        return bad_request(problem);
    }
    let Ok(user) = users::Model::find_by_reset_token(&ctx.db, &params.token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::info!("reset token not found");

        return format::json(());
    };
    user.into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await?;

    format::json(())
}

/// Creates a user login and returns a token
///
/// Every refusal is the same 401 after the same work: an email with no
/// account checks the password against a dummy hash (`src/hashing.rs`), so
/// neither the answer nor its time tells whether the address is
/// registered. An account whose email isn't verified yet is refused too,
/// as `verify` above promises.
#[debug_handler]
async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> {
    let user = users::Model::find_by_email(&ctx.db, &params.email)
        .await
        .ok();
    let valid = match &user {
        Some(user) => user.verify_password(&params.password).await,
        None => {
            hashing::verify_dummy(&params.password).await;
            false
        }
    };
    let Some(user) = user
        .filter(|_| valid)
        .filter(|user| user.email_verified_at.is_some())
    else {
        // Which of the three it was stays out of the log: the address is
        // personal data, and the count is what matters.
        return unauthorized("login refused");
    };

    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    format::json(LoginResponse::new(&user, &token))
}

#[debug_handler]
async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}

/// Magic link authentication provides a secure and passwordless way to log in to the application.
///
/// # Flow
/// 1. **Request a Magic Link**:
///    A registered user sends a POST request to `/magic-link` with their email.
///    If the email exists, a short-lived, one-time-use token is generated and sent to the user's email.
///    For security and to avoid exposing whether an email exists, the response always returns 200, even if the email is invalid.
///    An address on a domain the config doesn't allow (`settings.magic_link`) is refused first, with a 400.
///
/// 2. **Click the Magic Link**:
///    The user clicks the link (/magic-link/{token}), which validates the token and its expiration.
///    If valid, the server generates a JWT and responds with a [`LoginResponse`].
///    If invalid or expired, an unauthorized response is returned.
///
/// This flow enhances security by avoiding traditional passwords and providing a seamless login experience.
async fn magic_link(
    State(ctx): State<AppContext>,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    let settings: Settings = stored(&ctx)?;
    if !settings.magic_link.allows(&params.email) {
        tracing::debug!("magic link: the address is not on settings.magic_link.allowed_domains");
        return bad_request("invalid request");
    }

    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::debug!("magic link: no account for the address");
        return format::empty_json();
    };

    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    AuthMailer::send_magic_link(&ctx, &user).await?;

    format::empty_json()
}

/// Verifies a magic link token and authenticates the user.
///
/// The link was mailed to the account's address, so following it proves
/// the same thing the verification link does: an unverified email becomes
/// verified here, and `login` then accepts the account too.
async fn magic_link_verify(
    Path(token): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_magic_token(&ctx.db, &token).await else {
        // An unknown or expired token is refused, as the verify link's is.
        return unauthorized("unauthorized!");
    };

    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;
    let user = if user.email_verified_at.is_none() {
        user.into_active_model()
            .verified_by_magic_link(&ctx.db)
            .await?
    } else {
        user
    };

    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    format::json(LoginResponse::new(&user, &token))
}

#[debug_handler]
async fn resend_verification_email(
    State(ctx): State<AppContext>,
    Json(params): Json<ResendVerificationParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::info!("User not found for resend verification");
        return format::json(());
    };

    if user.email_verified_at.is_some() {
        tracing::info!(
            pid = user.pid.to_string(),
            "User already verified, skipping resend"
        );
        return format::json(());
    }

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;
    tracing::info!(pid = user.pid.to_string(), "Verification email re-sent");

    format::json(())
}

/// The `/api/auth` routes under their own bucket ([`AuthLimit`]), inside
/// the site-wide one: `register` mails any address and `login` takes a
/// password, so these routes get a fraction of the site-wide rate. The
/// bucket missing from the store cannot happen after `after_context`; the
/// branch that says so leaves the routes under the site-wide limit rather
/// than refusing to build them.
pub fn routes(ctx: &AppContext) -> Routes {
    let routes = Routes::new()
        .prefix("/api/auth")
        .add("/register", post(register))
        .add("/verify/{token}", get(verify))
        .add("/login", post(login))
        .add("/forgot", post(forgot))
        .add("/reset", post(reset))
        .add("/current", get(current))
        .add("/magic-link", post(magic_link))
        .add("/magic-link/{token}", get(magic_link_verify))
        .add("/resend-verification-mail", post(resend_verification_email));
    match stored::<AuthLimit>(ctx) {
        Ok(AuthLimit(Some(limit))) => routes.layer(limit),
        Ok(AuthLimit(None)) => routes,
        Err(e) => {
            tracing::error!(error = %e, "auth: /api/auth has only the site-wide limit");
            routes
        }
    }
}
