//! The sign-in page (controllers/login.rs): the form, and who is signed in
//! with the way out. Plain HTML, no island: the form posts and the server
//! answers with a page, as the reset page's does.

use leptos::prelude::*;

use crate::{
    controllers::{
        account, forgot,
        login::{CODE, CODE_AGAIN, LOGOUT, PATH},
        register, verify,
    },
    models::sign_ins::CODE_MINUTES,
    views::{
        layout::{BUTTON, FIELD, LEAD, NextSteps, PROBLEM, TITLE},
        parts::QUIET_BUTTON,
    },
};

/// The `id` of the line saying why the sign-in was refused.
const SIGN_IN_PROBLEM: &str = "sign-in-problem";

/// The form: the email and the password. `problem` is what the server
/// refused last time, if anything; then the email typed comes back, the
/// password field is `aria-invalid` and `autofocus`, so the focus is in it
/// for the next try (`Navigation` leaves an `autofocus` field alone).
///
/// While it's on its way, as `ResetForm`: a second press sends nothing
/// (`hx-sync`), the form is `aria-busy`, and the button takes the `busy:`
/// look after 200 ms.
#[component]
pub fn LoginForm(email: String, problem: Option<String>) -> impl IntoView {
    let refused = problem.is_some();
    view! {
        <h1 class=TITLE>"Sign in"</h1>
        <p class=LEAD>"With the email and password you signed up with."</p>
        {problem
            .map(|problem| {
                view! {
                    <p id=SIGN_IN_PROBLEM role="alert" class=PROBLEM>
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
        >
            <label for="email" class="block font-semibold">"Email"</label>
            <input
                id="email"
                name="email"
                type="email"
                autocomplete="username"
                required
                value=email
                class=FIELD
            />
            <label for="password" class="mt-6 block font-semibold">"Password"</label>
            <input
                id="password"
                name="password"
                type="password"
                autocomplete="current-password"
                autofocus=refused
                required
                aria-invalid=refused.then_some("true")
                aria-describedby=refused.then_some(SIGN_IN_PROBLEM)
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Sign in"
            </button>
        </form>
        <NextSteps links=vec![
            (forgot::PATH, "Forgot your password?"),
            (register::PATH, "No account yet? Sign up"),
            (verify::PATH, "No welcome mail? Send it again"),
        ] />
    }
}

/// The `id` of the line under the code's heading: why a code was refused,
/// or that a new one went.
const CODE_PROBLEM: &str = "code-problem";

/// The second step: the six digits mailed after the password, one field
/// the phone fills from the mail (`one-time-code`), and a new code on
/// request. `problem` is what the last post said.
#[component]
pub fn CodeForm(problem: Option<String>) -> impl IntoView {
    let refused = problem.is_some();
    view! {
        <h1 class=TITLE>"Your sign-in code"</h1>
        <p class=LEAD>
            {format!(
                "We mailed you six digits. They work for {CODE_MINUTES} minutes.",
            )}
        </p>
        {problem
            .map(|problem| {
                view! {
                    <p id=CODE_PROBLEM role="alert" class=PROBLEM>
                        {problem}
                    </p>
                }
            })}
        // Alpine's CSP build takes one expression per attribute.
        <form
            method="post"
            action=CODE
            class="mt-8 max-w-sm"
            hx-sync="this:drop"
            x-data="{ sending: false }"
            x-on:htmx:before:request="sending = true"
            x-on:htmx:finally:request="sending = false"
            x-bind:aria-busy="sending"
        >
            <label for="code" class="block font-semibold">"Code"</label>
            <input
                id="code"
                name="code"
                type="text"
                inputmode="numeric"
                autocomplete="one-time-code"
                maxlength="6"
                autofocus
                required
                aria-describedby=refused.then_some(CODE_PROBLEM)
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Sign in"
            </button>
        </form>
        <form method="post" action=CODE_AGAIN hx-sync="this:drop">
            <button type="submit" class=QUIET_BUTTON>
                "Mail me a new code"
            </button>
        </form>
    }
}

/// Signed in already: who, and the way out.
#[component]
pub fn SignedIn(name: String) -> impl IntoView {
    view! {
        <h1 class=TITLE>"You're signed in"</h1>
        <p class=LEAD>{format!("As {name}.")}</p>
        // Alpine's CSP build takes one expression per attribute.
        <form
            method="post"
            action=LOGOUT
            hx-sync="this:drop"
            x-data="{ sending: false }"
            x-on:htmx:before:request="sending = true"
            x-on:htmx:finally:request="sending = false"
            x-bind:aria-busy="sending"
        >
            <button type="submit" class=BUTTON>
                "Sign out"
            </button>
        </form>
        <NextSteps links=vec![(account::PATH, "Your account")] />
    }
}
