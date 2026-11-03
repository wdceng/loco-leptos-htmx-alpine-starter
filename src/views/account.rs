//! The account page (controllers/account.rs): who is signed in, their
//! browsers, a new password, and the account deleted. Plain HTML, no
//! island: each form posts and the server answers with the page, as the
//! sign-in page's does.

use chrono::{DateTime, Utc};
use leptos::prelude::*;

use crate::{
    controllers::account::{CLOSE_OTHERS, DELETE, PASSWORD, close_one},
    controllers::login::LOGOUT,
    models::{
        sessions::{Browser, PROVEN_MINUTES},
        users::{PASSWORD_MAX, PASSWORD_MIN},
    },
    views::layout::{BUTTON, FIELD, LEAD, Notice, PROBLEM, TITLE},
};

/// A section's heading on the page.
const HEADING: &str = "mt-12 text-2xl font-bold tracking-tight";

/// The `id` of the line saying why the new password was refused.
const PASSWORD_PROBLEM: &str = "password-problem";

/// The `id` of the line saying why the deletion was refused.
const DELETE_PROBLEM: &str = "delete-problem";

/// What a form on the page refused, by the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Password(String),
    Delete(String),
}

/// The user's browsers, the one this page was asked from marked.
#[derive(Clone, Debug, Default)]
pub struct Browsers(Vec<(Browser, bool)>);

impl Browsers {
    /// `listed`, the one whose session is `current` marked as this one.
    #[must_use]
    pub fn of(listed: Vec<Browser>, current: i64) -> Self {
        Self(
            listed
                .into_iter()
                .map(|browser| {
                    let here = browser.id == current;
                    (browser, here)
                })
                .collect(),
        )
    }
}

/// A moment as the page says it: the day and the time, UTC.
fn when(at: DateTime<Utc>) -> String {
    at.format("%-d %b %Y, %H:%M UTC").to_string()
}

/// While a form is on its way, as `ResetForm`: a second press sends
/// nothing (`hx-sync`), the form is `aria-busy`, and the button takes the
/// `busy:` look after 200 ms. Alpine's CSP build takes one expression per
/// attribute.
#[component]
fn BusyForm(action: String, children: Children) -> impl IntoView {
    view! {
        <form
            method="post"
            action=action
            class="mt-6 max-w-sm"
            hx-sync="this:drop"
            x-data="{ sending: false }"
            x-on:htmx:before:request="sending = true"
            x-on:htmx:finally:request="sending = false"
            x-bind:aria-busy="sending"
        >
            {children()}
        </form>
    }
}

/// The line saying why a form was refused, an alert its field points at.
#[component]
fn ProblemLine(id: &'static str, said: Option<String>) -> impl IntoView {
    said.map(|said| {
        view! {
            <p id=id role="alert" class=PROBLEM>
                {said}
            </p>
        }
    })
}

/// The page: who, the browsers, a new password, the deletion. `problem` is
/// what a form refused last, `said` what was just done.
#[component]
pub fn AccountPage(
    name: String,
    email: String,
    browsers: Browsers,
    proven: bool,
    problem: Option<Problem>,
    said: Option<&'static str>,
) -> impl IntoView {
    let password_problem = match &problem {
        Some(Problem::Password(said)) => Some(said.clone()),
        _ => None,
    };
    let delete_problem = match &problem {
        Some(Problem::Delete(said)) => Some(said.clone()),
        _ => None,
    };
    let others = browsers.0.iter().filter(|(_, here)| !here).count();
    view! {
        <h1 class=TITLE>"Your account"</h1>
        <p class=LEAD>{format!("{name}, {email}.")}</p>
        {said.map(|said| view! { <p role="status" class="mt-6 font-semibold">{said}</p> })}

        <h2 class=HEADING>"Where you're signed in"</h2>
        <ul class="mt-6 space-y-4">
            {browsers
                .0
                .into_iter()
                .map(|(browser, here)| {
                    view! {
                        <li>
                            // One string: two children would part with a
                            // marker between them.
                            <p class="font-semibold">
                                {if here {
                                    format!("{} (this browser)", browser.browser)
                                } else {
                                    browser.browser.clone()
                                }}
                            </p>
                            <p class="text-ink-muted">
                                {format!(
                                    "Signed in {}, last used {}.",
                                    when(browser.created_at),
                                    when(browser.seen_at),
                                )}
                            </p>
                            {if here {
                                view! {
                                    <BusyForm action=LOGOUT.to_string()>
                                        <button type="submit" class=BUTTON>
                                            "Sign out"
                                        </button>
                                    </BusyForm>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <BusyForm action=close_one(browser.id)>
                                        <button type="submit" class=BUTTON>
                                            "Sign it out"
                                        </button>
                                    </BusyForm>
                                }
                                    .into_any()
                            }}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
        {(others > 1)
            .then(|| {
                view! {
                    <BusyForm action=CLOSE_OTHERS.to_string()>
                        <button type="submit" class=BUTTON>
                            "Sign out every other browser"
                        </button>
                    </BusyForm>
                }
            })}

        <h2 class=HEADING>"A new password"</h2>
        <p class=LEAD>
            {format!(
                "{PASSWORD_MIN} to {PASSWORD_MAX} characters. Every other browser is signed out.",
            )}
        </p>
        <ProblemLine id=PASSWORD_PROBLEM said=password_problem.clone() />
        <BusyForm action=PASSWORD.to_string()>
            <label for="current" class="block font-semibold">
                "Current password"
            </label>
            <input
                id="current"
                name="current"
                type="password"
                autocomplete="current-password"
                required
                aria-describedby=password_problem.is_some().then_some(PASSWORD_PROBLEM)
                class=FIELD
            />
            <label for="password" class="mt-6 block font-semibold">
                "New password"
            </label>
            <input
                id="password"
                name="password"
                type="password"
                autocomplete="new-password"
                required
                minlength=PASSWORD_MIN
                maxlength=PASSWORD_MAX
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Save the new password"
            </button>
        </BusyForm>

        <h2 class=HEADING>"Delete the account"</h2>
        <p class=LEAD>
            "Your account and everything signed in with it are gone, at once and for good."
        </p>
        <ProblemLine id=DELETE_PROBLEM said=delete_problem.clone() />
        <BusyForm action=DELETE.to_string()>
            {(!proven)
                .then(|| {
                    view! {
                        <label for="delete-password" class="block font-semibold">
                            "Your password"
                        </label>
                        <input
                            id="delete-password"
                            name="password"
                            type="password"
                            autocomplete="current-password"
                            required
                            aria-describedby=delete_problem
                                .is_some()
                                .then_some(DELETE_PROBLEM)
                            class=FIELD
                        />
                    }
                })}
            {proven
                .then(|| {
                    view! {
                        <p class="text-ink-muted">
                            {format!(
                                "You signed in or typed your password in the last {PROVEN_MINUTES} minutes, so it isn't asked again.",
                            )}
                        </p>
                    }
                })}
            <button type="submit" class=BUTTON>
                "Delete my account"
            </button>
        </BusyForm>
    }
}

/// After a deletion: nobody is signed in any more.
#[component]
pub fn AccountDeleted() -> impl IntoView {
    view! {
        <Notice
            heading="Your account is deleted"
            lead="It and every browser signed in with it are gone."
        />
    }
}
