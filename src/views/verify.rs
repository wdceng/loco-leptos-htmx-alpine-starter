//! The email verification page the welcome mail links to
//! (controllers/verify.rs): a button that verifies, and what it says
//! after. Plain HTML, no island: the form posts and the server answers with
//! a page, as the reset page's does.

use leptos::prelude::*;

use crate::{
    controllers::verify::PATH,
    models::users::EMAIL_VERIFICATION_EXPIRATION_HOURS,
    views::layout::{APP_NAME, BUTTON, LEAD, Notice, TITLE},
};

/// The button: the token from the mail's link in a hidden field. Opening
/// the page verifies nothing; the press does, so a mail scanner that opens
/// the link leaves the account as it was.
///
/// While it's on its way, as `ResetForm`: a second press sends nothing
/// (`hx-sync`), the form is `aria-busy`, and the button takes the `busy:`
/// look after 200 ms.
#[component]
pub fn VerifyForm(token: String) -> impl IntoView {
    view! {
        <h1 class=TITLE>"Verify your email"</h1>
        <p class=LEAD>
            {format!(
                "Press the button to finish signing up to {APP_NAME}. If you didn't sign up, close this page: nothing happens.",
            )}
        </p>
        // Alpine's CSP build takes one expression per attribute.
        <form
            method="post"
            action=PATH
            hx-sync="this:drop"
            x-data="{ sending: false }"
            x-on:htmx:before:request="sending = true"
            x-on:htmx:finally:request="sending = false"
            x-bind:aria-busy="sending"
        >
            <input type="hidden" name="token" value=token />
            <button type="submit" class=BUTTON>
                "Verify my email"
            </button>
        </form>
    }
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
