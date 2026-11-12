//! The browser tests' harness (docs/testing.md, "Browser tests").
//!
//! `e2e/run.sh` builds the site, and for each browser's lane starts its own
//! site (`LOCO_ENV=e2e`, config/e2e.yaml) and the browser's driver, then
//! runs these tests with where each listens in the environment. A test
//! opens a session with [`browse`], which closes it whatever the test does
//! (Safari allows one session at a time), keeps what a failed test's page
//! showed, and fails a test whose page logged an error or a warning where
//! the browser lets it be heard.
//!
//! One file a job: the browsers (`browser`), a test's session (`session`),
//! the page and what any test does with it (`page`), the browsers' own
//! tools (`devtools`), the Android app's side through adb (`device`), the
//! lane's mails (`mail`), the accounts the tests sign in as (`accounts`)
//! and a post sent slowly by hand (`slow`).

use std::time::Duration;

mod accounts;
mod browser;
mod device;
mod devtools;
mod mail;
mod page;
mod session;
mod slow;

pub use accounts::{PASSWORD, mailbox};
pub use browser::{Browser, Screen};
pub use device::app_package;
pub use devtools::Held;
pub use page::Page;
pub use session::{browse, browse_app};
pub use slow::{Answered, slow_post};
pub use thirtyfour::prelude::*;

/// How long a wait waits: the debug build's WASM is several megabytes.
pub(crate) const WAIT: Duration = Duration::from_secs(30);

/// How long a page has to start its scripts and wake its islands
/// ([`Page::ready`]): with nine lanes at once the Mac's cores are shared by
/// nine servers, the browsers, Simulators and emulators, and Firefox and
/// Safari took more than 30 seconds now and then to start a debug build's
/// WASM in an islands app on this stack. A page that wakes sooner ends the
/// wait sooner.
pub(crate) const AWAKE_WITHIN: Duration = Duration::from_secs(60);

/// How long a page has to start loading before it is asked for again.
pub(crate) const OPEN_WITHIN: Duration = Duration::from_secs(5);

/// How often a wait asks again.
pub(crate) const POLL: Duration = Duration::from_millis(100);

/// How long a person's press holds the mouse's button down: long enough
/// for the press to preload its page before the click (`Page::press`).
pub(crate) const PRESS_HELD: Duration = Duration::from_millis(300);

/// The reset page with a token: a page without the lazy island, with a
/// form, long enough to scroll in a short window.
pub const RESET: &str = "/reset?token=e2e";

/// An [`Page::answer_with`] answer: the real one, named as a newer build's
/// (`X-Build`), followed redirects and all.
pub const NEWER_BUILD: &str = "const real = await original(...args);
     const headers = new Headers(real.headers);
     headers.set('X-Build', 'e2e-newer');
     const made = new Response(await real.text(), { status: real.status, headers });
     Object.defineProperty(made, 'redirected', { value: real.redirected });
     return made;";

/// The header's link home, on every page.
pub const HOME_LINK: &str = "header a[href='/']";
