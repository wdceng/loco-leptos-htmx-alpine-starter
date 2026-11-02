//! The sign-up page (controllers/register.rs): the form, and what it says
//! after. Plain HTML, no island: the form posts and the server answers with
//! a page, as the sign-in page's does.

use leptos::prelude::*;

use crate::{
    controllers::{login, register::PATH},
    models::users::{EMAIL_VERIFICATION_EXPIRATION_HOURS, PASSWORD_MAX, PASSWORD_MIN},
    views::layout::{BUTTON, FIELD, LEAD, NextSteps, Notice, PROBLEM, TITLE},
};

/// The `id` of the line saying why the sign-up was refused.
const SIGN_UP_PROBLEM: &str = "sign-up-problem";

/// The form: the name, the email and the password. `problem` is what the
/// server refused last time, if anything; then the name and email typed
/// come back and the line is above the form, an alert. Which field it
/// names is in its words: the rules are checked one after another.
///
/// While it's on its way, as `ResetForm`: a second press sends nothing
/// (`hx-sync`), the form is `aria-busy`, and the button takes the `busy:`
/// look after 200 ms.
#[component]
pub fn RegisterForm(name: String, email: String, problem: Option<String>) -> impl IntoView {
    let refused = problem.is_some();
    view! {
        <h1 class=TITLE>"Sign up"</h1>
        <p class=LEAD>
            {format!("A password of {PASSWORD_MIN} to {PASSWORD_MAX} characters. Any characters work.")}
        </p>
        {problem
            .map(|problem| {
                view! {
                    <p id=SIGN_UP_PROBLEM role="alert" class=PROBLEM>
                        {problem}
                    </p>
                }
            })}
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
            aria-describedby=refused.then_some(SIGN_UP_PROBLEM)
        >
            <label for="name" class="block font-semibold">"Name"</label>
            <input
                id="name"
                name="name"
                type="text"
                autocomplete="name"
                required
                value=name
                class=FIELD
            />
            <label for="email" class="mt-6 block font-semibold">"Email"</label>
            <input
                id="email"
                name="email"
                type="email"
                autocomplete="email"
                required
                value=email
                class=FIELD
            />
            <label for="password" class="mt-6 block font-semibold">"Password"</label>
            <input
                id="password"
                name="password"
                type="password"
                autocomplete="new-password"
                autofocus=refused
                required
                minlength=PASSWORD_MIN
                maxlength=PASSWORD_MAX
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Sign up"
            </button>
        </form>
        <NextSteps links=vec![(login::PATH, "Have an account? Sign in")] />
    }
}

/// After a sign-up the rules took: the same words whether the address was
/// new or taken.
#[component]
pub fn RegisterSent() -> impl IntoView {
    let lead = format!(
        "We sent a link to the address you gave. Open it within {EMAIL_VERIFICATION_EXPIRATION_HOURS} hours to finish signing up. No mail? Look in the spam folder.",
    );
    view! { <Notice heading="Check your mail" lead /> }
}
