//! The password-reset page the forgot-password mail links to
//! (controllers/reset.rs): the form, and what it says after a post. Plain
//! HTML, no island: the form posts and the server answers with a page.

use leptos::prelude::*;

use crate::{
    controllers::reset::PATH,
    models::users::{PASSWORD_MAX, PASSWORD_MIN, RESET_TOKEN_EXPIRATION_MIN},
    views::layout::{BUTTON, BusyForm, FIELD, LEAD, Notice, ProblemLine, TITLE},
};

/// The `id` of the line saying why the password was refused.
const PASSWORD_PROBLEM: &str = "password-problem";

/// The form: the token from the mail's link in a hidden field, and the new
/// password. `problem` is what the server refused last time, if anything;
/// then the field is `autofocus`, so the focus stays in it for the next
/// try: htmx focuses it in the swapped-in page, and `Navigation` leaves an
/// `autofocus` field alone instead of focusing `<main>`.
///
/// A refused field is `aria-invalid` and points at its line
/// (`aria-describedby`). As the visitor leaves a changed field, Alpine
/// marks it the same way from the browser's own check (`minlength`,
/// `maxlength`), and clears the mark once it passes.
///
/// Sent as every form is (`BusyForm`): a second press sends nothing, where
/// it would find the token used up and swap "expired" over "changed".
#[component]
pub fn ResetForm(token: String, problem: Option<String>) -> impl IntoView {
    let refused = problem.is_some();
    view! {
        <h1 class=TITLE>"Choose a new password"</h1>
        <p class=LEAD>
            {format!("{PASSWORD_MIN} to {PASSWORD_MAX} characters. Any characters work.")}
        </p>
        <ProblemLine id=PASSWORD_PROBLEM problem />
        <BusyForm action=PATH>
            <input type="hidden" name="token" value=token />
            <label for="password" class="block font-semibold">"New password"</label>
            <input
                id="password"
                name="password"
                type="password"
                autocomplete="new-password"
                autofocus=refused
                required
                minlength=PASSWORD_MIN
                maxlength=PASSWORD_MAX
                aria-invalid=refused.then_some("true")
                aria-describedby=refused.then_some(PASSWORD_PROBLEM)
                x-data=if refused { "{ invalid: true }" } else { "{ invalid: false }" }
                x-on:change="invalid = !$el.checkValidity()"
                x-bind:aria-invalid="invalid"
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Save the new password"
            </button>
        </BusyForm>
    }
}

/// After a good post: the password is changed.
#[component]
pub fn ResetDone() -> impl IntoView {
    view! {
        <Notice heading="Your password is changed" lead="Sign in with the new one from now on." />
    }
}

/// The link's token is unknown, used or expired.
#[component]
pub fn ResetExpired() -> impl IntoView {
    let lead = format!(
        "A reset link works once, for {RESET_TOKEN_EXPIRATION_MIN} minutes. Ask for a new one.",
    );
    view! { <Notice heading="This link has expired" lead /> }
}
