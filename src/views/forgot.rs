//! The forgotten-password page (controllers/forgot.rs): the form, and what
//! it says after. Plain HTML, no island: the form posts and the server
//! answers with a page, as the sign-in page's does.

use leptos::prelude::*;

use crate::{
    controllers::{forgot::PATH, login},
    models::users::RESET_TOKEN_EXPIRATION_MIN,
    views::layout::{BUTTON, FIELD, LEAD, NextSteps, Notice, TITLE},
};

/// The form: the account's email.
///
/// While it's on its way, as `ResetForm`: a second press sends nothing
/// (`hx-sync`), the form is `aria-busy`, and the button takes the `busy:`
/// look after 200 ms.
#[component]
pub fn ForgotForm() -> impl IntoView {
    view! {
        <h1 class=TITLE>"Forgot your password?"</h1>
        <p class=LEAD>"We'll mail you a link to choose a new one."</p>
        // Alpine's CSP build takes one expression per attribute.
        <form
            method="post"
            action=PATH
            class="mt-8 max-w-sm"
            hx-sync="this:drop"
            x-data="{ sending: false }"
            x-on:htmx:before:request="sending = true"
            x-on:htmx:finally:request="sending = false"
            x-bind:aria-busy="sending"
        >
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
                "Send the link"
            </button>
        </form>
        <NextSteps links=vec![(login::PATH, "Remembered it? Sign in")] />
    }
}

/// After a post: the same words whether the address has an account or not.
#[component]
pub fn ForgotSent() -> impl IntoView {
    let lead = format!(
        "If an account uses that address, a link to choose a new password is on its way. It works once, for {RESET_TOKEN_EXPIRATION_MIN} minutes.",
    );
    view! { <Notice heading="Check your mail" lead /> }
}
