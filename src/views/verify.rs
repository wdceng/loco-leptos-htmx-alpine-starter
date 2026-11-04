//! The email verification page the welcome mail links to
//! (controllers/verify.rs): a button that verifies, and what it says
//! after; without a token, the form that sends the mail again. Plain HTML, no island: the form posts and the server answers with
//! a page, as the reset page's does.

use leptos::prelude::*;

use crate::{
    controllers::{
        login,
        verify::{PATH, RESEND},
    },
    models::users::EMAIL_VERIFICATION_EXPIRATION_HOURS,
    views::layout::{APP_NAME, BUTTON, BusyForm, FIELD, LEAD, NextSteps, Notice, TITLE},
};

/// The button: the token from the mail's link in a hidden field. Opening
/// the page verifies nothing; the press does, so a mail scanner that opens
/// the link leaves the account as it was. Sent as every form is
/// (`BusyForm`).
#[component]
pub fn VerifyForm(token: String) -> impl IntoView {
    view! {
        <h1 class=TITLE>"Verify your email"</h1>
        <p class=LEAD>
            {format!(
                "Press the button to finish signing up to {APP_NAME}. If you didn't sign up, close this page: nothing happens.",
            )}
        </p>
        <BusyForm action=PATH class="">
            <input type="hidden" name="token" value=token />
            <button type="submit" class=BUTTON>
                "Verify my email"
            </button>
        </BusyForm>
    }
}

/// `/verify` without a token: the form that sends the welcome mail again,
/// the account's email its one field. Sent as every form is (`BusyForm`).
#[component]
pub fn ResendForm() -> impl IntoView {
    view! {
        <h1 class=TITLE>"Send the welcome mail again"</h1>
        <p class=LEAD>"Its link verifies your email, so you can sign in."</p>
        <BusyForm action=RESEND>
            <label for="email" class="block font-semibold">"Email"</label>
            <input
                id="email"
                name="email"
                type="email"
                autocomplete="username"
                required
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Send it again"
            </button>
        </BusyForm>
        <NextSteps links=vec![(login::PATH, "Verified already? Sign in")] />
    }
}

/// After a post for a new mail: the same words whether the address has an
/// account waiting or not.
#[component]
pub fn ResendSent() -> impl IntoView {
    let lead = format!(
        "If an account with that address still waits to be verified, a new link is on its way. It works for {EMAIL_VERIFICATION_EXPIRATION_HOURS} hours; earlier links no longer do.",
    );
    view! { <Notice heading="Check your mail" lead /> }
}

/// After a good press: the address is verified, now or before.
#[component]
pub fn VerifyDone() -> impl IntoView {
    view! { <Notice heading="Your email is verified" lead="You can sign in now." /> }
}

/// The link's token is unknown, used or expired.
#[component]
pub fn VerifyExpired() -> impl IntoView {
    let lead = format!(
        "A verification link works once, within {EMAIL_VERIFICATION_EXPIRATION_HOURS} hours. If your email is verified already, sign in; if not, ask for a new mail.",
    );
    view! { <Notice heading="This link no longer works" lead /> }
}
