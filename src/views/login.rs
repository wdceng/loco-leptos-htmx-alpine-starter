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
        layout::{BUTTON, BusyForm, FIELD, LEAD, NextSteps, ProblemLine, SaidLine, TITLE},
        parts::QUIET_BUTTON,
    },
};

/// The `id` of the line saying why the sign-in was refused.
const SIGN_IN_PROBLEM: &str = "sign-in-problem";

/// The form: the email and the password. `problem` is what the server
/// refused last time, if anything; then the email typed comes back, the
/// password field is `aria-invalid` and `autofocus`, so the focus is in it
/// for the next try (`Navigation` leaves an `autofocus` field alone). Sent
/// as every form is (`BusyForm`).
#[component]
pub fn LoginForm(email: String, problem: Option<String>) -> impl IntoView {
    let refused = problem.is_some();
    view! {
        <h1 class=TITLE>"Sign in"</h1>
        <p class=LEAD>"With the email and password you signed up with."</p>
        <ProblemLine id=SIGN_IN_PROBLEM problem />
        <BusyForm action=PATH>
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
        </BusyForm>
        <NextSteps links=vec![
            (forgot::PATH, "Forgot your password?"),
            (register::PATH, "No account yet? Sign up"),
            (verify::PATH, "No welcome mail? Send it again"),
        ] />
    }
}

/// The `id` of the line under the code's heading: why a code was refused.
const CODE_PROBLEM: &str = "code-problem";

/// What the code's page refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeProblem {
    /// The code typed was wrong: its field is marked.
    Wrong(String),
    /// Nothing checked or mailed for now: the wait after wrong codes, or
    /// the minute between two codes. The field is not at fault.
    NotNow(String),
}

/// The second step: the six digits mailed after the password, one field
/// the phone fills from the mail (`one-time-code`), and a new code on
/// request. `problem` is what the last post refused, `said` what it did.
#[component]
pub fn CodeForm(problem: Option<CodeProblem>, said: Option<&'static str>) -> impl IntoView {
    let wrong = matches!(problem, Some(CodeProblem::Wrong(_)));
    let problem = problem.map(|problem| match problem {
        CodeProblem::Wrong(line) | CodeProblem::NotNow(line) => line,
    });
    view! {
        <h1 class=TITLE>"Your sign-in code"</h1>
        <p class=LEAD>
            {format!(
                "We mailed you six digits. They work for {CODE_MINUTES} minutes.",
            )}
        </p>
        <ProblemLine id=CODE_PROBLEM problem />
        <SaidLine said />
        <BusyForm action=CODE>
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
                aria-invalid=wrong.then_some("true")
                aria-describedby=wrong.then_some(CODE_PROBLEM)
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Sign in"
            </button>
        </BusyForm>
        <BusyForm action=CODE_AGAIN class="">
            <button type="submit" class=QUIET_BUTTON>
                "Mail me a new code"
            </button>
        </BusyForm>
    }
}

/// Signed in already: who, and the way out.
#[component]
pub fn SignedIn(name: String) -> impl IntoView {
    view! {
        <h1 class=TITLE>"You're signed in"</h1>
        <p class=LEAD>{format!("As {name}.")}</p>
        <BusyForm action=LOGOUT class="">
            <button type="submit" class=BUTTON>
                "Sign out"
            </button>
        </BusyForm>
        <NextSteps links=vec![(account::PATH, "Your account")] />
    }
}
