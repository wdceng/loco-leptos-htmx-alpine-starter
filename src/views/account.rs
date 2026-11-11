//! The account page (controllers/account.rs): who is signed in, their
//! browsers, a new password, and the account deleted. Plain HTML, no
//! island: each form posts and the server answers with the page, as the
//! sign-in page's does.

use chrono::{DateTime, Utc};
use leptos::prelude::*;

use crate::{
    controllers::account::{AVATAR, CLOSE_OTHERS, DELETE, PASSWORD, close_one},
    controllers::login::LOGOUT,
    models::{
        sessions::{Browser, PROVEN_MINUTES},
        users::{PASSWORD_MAX, PASSWORD_MIN},
    },
    uploads::{SHRINK_SIDE, SHRINK_UNDER},
    views::{
        layout::{BUTTON, BusyForm, FIELD, LEAD, Notice, ProblemLine, SaidLine, TITLE},
        parts::{DANGER_BUTTON, Dialog, DialogCancel},
    },
};

/// A section's heading on the page.
const HEADING: &str = "mt-12 text-2xl font-bold tracking-tight";

/// The `id` of the line saying why the picture was refused.
const AVATAR_PROBLEM: &str = "avatar-problem";

/// The `id` of the line saying why the new password was refused.
const PASSWORD_PROBLEM: &str = "password-problem";

/// The `id` of the line saying why the deletion was refused.
const DELETE_PROBLEM: &str = "delete-problem";

/// The `id` of the dialog that asks before the account is deleted.
const DELETE_DIALOG: &str = "delete-account";

/// A form's place on the page.
const FORM: &str = "mt-6 max-w-sm";

/// What a form on the page refused, by the field: that field is marked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Avatar(String),
    /// The current password, which the new one's form asks first.
    Current(String),
    /// The new password's rule.
    New(String),
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

/// The page: who, the browsers, a new password, the deletion. `problem` is
/// what a form refused last, `said` what was just done. Every form is sent
/// as every form is (`BusyForm`).
#[component]
pub fn AccountPage(
    name: String,
    email: String,
    browsers: Browsers,
    /// The avatar's address, if one was sent.
    avatar: Option<String>,
    proven: bool,
    problem: Option<Problem>,
    said: Option<&'static str>,
) -> impl IntoView {
    let avatar_problem = match &problem {
        Some(Problem::Avatar(line)) => Some(line.clone()),
        _ => None,
    };
    let (current_refused, new_refused) = (
        matches!(problem, Some(Problem::Current(_))),
        matches!(problem, Some(Problem::New(_))),
    );
    let password_problem = match &problem {
        Some(Problem::Current(line) | Problem::New(line)) => Some(line.clone()),
        _ => None,
    };
    let delete_problem = match &problem {
        Some(Problem::Delete(line)) => Some(line.clone()),
        _ => None,
    };
    let (avatar_refused, delete_refused) = (avatar_problem.is_some(), delete_problem.is_some());
    let others = browsers.0.iter().filter(|(_, here)| !here).count();
    view! {
        <h1 class=TITLE>"Your account"</h1>
        <p class=LEAD>{format!("{name}, {email}.")}</p>
        <SaidLine said />

        <h2 class=HEADING>"Your picture"</h2>
        {avatar
            .map(|src| {
                view! {
                    <img
                        src=src
                        alt="Your picture"
                        width="96"
                        height="96"
                        class="mt-6 size-24 rounded-full"
                    />
                }
            })}
        <ProblemLine id=AVATAR_PROBLEM problem=avatar_problem />
        // A photo shrunk in the browser before it goes (`data-shrink`, the
        // `shrink` in islands.rs), so a phone's photo doesn't go up whole
        // over mobile data; the server makes the avatar either way.
        <BusyForm action=AVATAR class=FORM enctype="multipart/form-data">
            <label for="avatar" class="block font-semibold">
                "A photo, JPEG, PNG or WebP"
            </label>
            <input
                id="avatar"
                name="avatar"
                type="file"
                accept="image/jpeg,image/png,image/webp"
                required
                data-shrink=SHRINK_SIDE.to_string()
                data-shrink-under=SHRINK_UNDER.to_string()
                aria-invalid=avatar_refused.then_some("true")
                aria-describedby=avatar_refused.then_some(AVATAR_PROBLEM)
                class=FIELD
            />
            <button type="submit" class=BUTTON>
                "Save the picture"
            </button>
        </BusyForm>

        <h2 class=HEADING>"Where you're signed in"</h2>
        // A browser's button nearer its own lines than the next browser:
        // groups further apart than their lines.
        <ul class="mt-6 space-y-10">
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
                                    <BusyForm action=LOGOUT class="max-w-sm">
                                        <button type="submit" class=BUTTON>
                                            "Sign out"
                                        </button>
                                    </BusyForm>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <BusyForm action=close_one(browser.id) class="max-w-sm">
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
                    <BusyForm action=CLOSE_OTHERS class=FORM>
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
        <ProblemLine id=PASSWORD_PROBLEM problem=password_problem />
        <BusyForm action=PASSWORD class=FORM>
            <label for="current" class="block font-semibold">
                "Current password"
            </label>
            <input
                id="current"
                name="current"
                type="password"
                autocomplete="current-password"
                required
                aria-invalid=current_refused.then_some("true")
                aria-describedby=current_refused.then_some(PASSWORD_PROBLEM)
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
                aria-invalid=new_refused.then_some("true")
                aria-describedby=new_refused.then_some(PASSWORD_PROBLEM)
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
        <ProblemLine id=DELETE_PROBLEM problem=delete_problem />
        <button type="button" popovertarget=DELETE_DIALOG class=DANGER_BUTTON>
            "Delete my account"
        </button>
        <Dialog id=DELETE_DIALOG title="Delete your account?">
        <BusyForm action=DELETE class=FORM>
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
                            autofocus
                            required
                            aria-invalid=delete_refused.then_some("true")
                            aria-describedby=delete_refused.then_some(DELETE_PROBLEM)
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
            <div class="flex flex-wrap gap-x-3">
                <button type="submit" class=DANGER_BUTTON>
                    "Delete it for good"
                </button>
                // The focus on Cancel where no password is asked, never on
                // the step that can't be undone.
                <DialogCancel target=DELETE_DIALOG autofocus=proven />
            </div>
        </BusyForm>
        </Dialog>
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
