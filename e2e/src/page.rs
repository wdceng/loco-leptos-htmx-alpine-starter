//! A browser session on the run's site, and what any test does with it:
//! open a page and wait for it, read it, wait for a state, press.
//!
//! No element is ever the driver's: chromedriver keeps each element it
//! hands a test in a cache inside the page, and one kept link of a page
//! moved away from holds that page's whole detached tree, which the memory
//! tests would read as a leak (the sister app found it, 2026-10-10). A
//! press is a mouse's at a point the page's script measured: through
//! DevTools in Chrome and Edge, WebDriver's own pointer in Firefox, the
//! page's own events in Safari; on a device a finger's ([`Page::press`]).

use std::time::Duration;

use serde_json::{Value, json};
use thirtyfour::cdp::CdpSession;
use thirtyfour::common::command::{Actions, Command};
use thirtyfour::prelude::*;
use tokio::task::AbortHandle;

use crate::{
    AWAKE_WITHIN, Browser, OPEN_WITHIN, POLL, PRESS_HELD, Screen, WAIT, devtools::Shared,
    session::skipped,
};

/// How long a finger rests on the screen in a tap.
const TAP_HELD: Duration = Duration::from_millis(50);

/// A browser session on the run's site.
#[derive(Clone)]
pub struct Page {
    pub driver: WebDriver,
    pub browser: Browser,
    /// The DevTools socket, Chrome's and Edge's alone.
    pub(crate) cdp: Option<CdpSession>,
    pub(crate) base: String,
    /// The lane's folder (`target/e2e/<browser>/`).
    pub(crate) dir: String,
    /// The running test's name, for its skip.
    pub(crate) test: String,
    pub(crate) heard: Shared,
    /// What listens to the page: DevTools' events, or BiDi's log in
    /// Firefox. Safari has neither.
    pub(crate) watcher: Option<AbortHandle>,
}

impl Page {
    /// The site's full address of `path`.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// Load `path` whole and wait until the page is [`Page::ready`]: its
    /// address first, asked for again where the browser hasn't moved within
    /// a few seconds, as safaridriver now and then returns from going to a
    /// page without going, the page before still showing.
    pub async fn open(&self, path: &str) -> WebDriverResult<()> {
        let there = format!("return location.pathname + location.search === {path:?}");
        for _ in 0..3 {
            self.driver.goto(self.url(path)).await?;
            let started = tokio::time::Instant::now();
            while started.elapsed() < OPEN_WITHIN {
                if self.eval(&there).await?.as_bool() == Some(true) {
                    self.until(
                        &format!("{path} loaded"),
                        "return document.readyState === 'complete'",
                    )
                    .await?;
                    return self.ready().await;
                }
                tokio::time::sleep(POLL).await;
            }
        }
        let at = self.path().await.unwrap_or_default();
        Err(WebDriverError::Timeout(format!(
            "{path} never loaded, the page at {at}"
        )))
    }

    /// Wait until htmx and Alpine have started and every island on the
    /// page is awake: the debug build's `data-awake` on each
    /// `<leptos-island>` (src/collect.rs, `mark_awake`), since Leptos's
    /// loader says nothing when it's done. `e2e/run.sh` builds a debug
    /// site; a release one carries no mark.
    pub async fn ready(&self) -> WebDriverResult<()> {
        self.until_within(
            "htmx, Alpine and the islands",
            "return !!(window.htmx && window.Alpine)
                 && [...document.querySelectorAll('leptos-island')].every(i => i.hasAttribute('data-awake'))",
            AWAKE_WITHIN,
        )
        .await
    }

    /// Wait until no request of htmx's is on its way (`htmx-request` on its
    /// element) and the page is ready again.
    pub async fn settled(&self) -> WebDriverResult<()> {
        self.until(
            "htmx's requests answered",
            "return !document.querySelector('.htmx-request')",
        )
        .await?;
        self.ready().await
    }

    /// Wait until `script` returns true, or fail naming `what`.
    pub async fn until(&self, what: &str, script: &str) -> WebDriverResult<()> {
        self.until_within(what, script, WAIT).await
    }

    /// [`Page::until`] with a time of its own.
    pub async fn until_within(
        &self,
        what: &str,
        script: &str,
        within: Duration,
    ) -> WebDriverResult<()> {
        let started = tokio::time::Instant::now();
        loop {
            if self.eval(script).await?.as_bool() == Some(true) {
                return Ok(());
            }
            if started.elapsed() > within {
                return Err(WebDriverError::Timeout(format!(
                    "{what}: not within {within:?}"
                )));
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// What `script` returns.
    pub async fn eval(&self, script: &str) -> WebDriverResult<Value> {
        Ok(self
            .driver
            .execute(script, Vec::new())
            .await?
            .json()
            .clone())
    }

    /// What `script` returns, as text.
    pub async fn text(&self, script: &str) -> WebDriverResult<String> {
        Ok(self
            .eval(script)
            .await?
            .as_str()
            .unwrap_or_default()
            .to_owned())
    }

    /// The page's path and query.
    pub async fn path(&self) -> WebDriverResult<String> {
        self.text("return location.pathname + location.search")
            .await
    }

    /// Whether the page's text holds `words`.
    pub async fn says(&self, words: &str) -> WebDriverResult<bool> {
        Ok(self
            .eval(&format!(
                "return document.body.innerText.includes({words:?})"
            ))
            .await?
            == Value::Bool(true))
    }

    /// Wait until nothing on the page moves: no animation or transition
    /// running, so a press never lands on what still slides away.
    pub async fn still(&self) -> WebDriverResult<()> {
        self.until(
            "the page still",
            "return document.getAnimations().every(a => a.playState !== 'running')",
        )
        .await
    }

    /// Press the first element `css` finds as a person's mouse does: the
    /// pointer onto it, the button down, held long enough for the press to
    /// preload its page (`PRELOAD`, src/views/layout.rs), then up, the click
    /// with it. With the answer there before the click, htmx can swap it in
    /// before Alpine's click runs, which an instant script click never
    /// shows (tests/requests/races.rs). The page still first.
    ///
    /// Chrome and Edge through DevTools' mouse; Firefox through WebDriver's
    /// own pointer at the same point; an Android emulator's Chrome a
    /// finger's tap there, WebDriver's touch pointer. safaridriver sends no
    /// `mousedown`, so in Safari, on the Mac or in a Simulator, the page's
    /// own events: the pointer down (a finger's in a Simulator) and, unless
    /// the page cancels it, the focus off the field it was in, then the
    /// pointer up and the click; nothing held, so nothing preloaded.
    pub async fn press(&self, css: &str) -> WebDriverResult<()> {
        self.still().await?;
        let (x, y) = self.centre(css).await?;
        match self.browser {
            Browser::Chrome | Browser::Edge => {
                for kind in ["mouseMoved", "mousePressed"] {
                    self.mouse(kind, x, y).await?;
                }
                tokio::time::sleep(PRESS_HELD).await;
                self.mouse("mouseReleased", x, y).await
            }
            Browser::Firefox => self.pointer("mouse", x, y, PRESS_HELD).await,
            Browser::Android | Browser::AndroidTablet => {
                self.pointer("touch", x, y, TAP_HELD).await
            }
            Browser::Safari | Browser::Iphone | Browser::Ipad | Browser::IpadAir => {
                let pointer = if self.browser.is_simulator() {
                    "touch"
                } else {
                    "mouse"
                };
                self.eval(&format!(
                    "const e = document.querySelector({css:?});
                     const kind = {{ bubbles: true, cancelable: true, composed: true,
                                     pointerType: {pointer:?}, isPrimary: true }};
                     if (e.dispatchEvent(new PointerEvent('pointerdown', kind))
                         && e.dispatchEvent(new MouseEvent('mousedown', kind))) {{
                       document.activeElement?.blur?.();
                     }}
                     e.dispatchEvent(new PointerEvent('pointerup', kind));
                     e.click()"
                ))
                .await
                .map(drop)
            }
        }
    }

    /// The centre of the first element `css` finds, in the window's
    /// pixels, scrolled into view only where it isn't (`nearest`, so a
    /// test's own scroll stays); waited for.
    async fn centre(&self, css: &str) -> WebDriverResult<(f64, f64)> {
        let started = tokio::time::Instant::now();
        loop {
            let at = self
                .eval(&format!(
                    "const e = document.querySelector({css:?});
                     if (!e) return null;
                     e.scrollIntoView({{ block: 'nearest', inline: 'nearest' }});
                     const r = e.getBoundingClientRect();
                     return [r.left + r.width / 2, r.top + r.height / 2]"
                ))
                .await?;
            if let (Some(x), Some(y)) = (at[0].as_f64(), at[1].as_f64()) {
                return Ok((x, y));
            }
            if started.elapsed() > WAIT {
                return Err(WebDriverError::Timeout(format!(
                    "{css}: not on the page within {WAIT:?}"
                )));
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// One mouse event at (`x`, `y`), the left button's, through DevTools.
    async fn mouse(&self, kind: &str, x: f64, y: f64) -> WebDriverResult<()> {
        self.devtools(
            "Input.dispatchMouseEvent",
            json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 }),
        )
        .await
        .map(drop)
    }

    /// WebDriver's own pointer, a mouse's or a finger's (`kind`), at
    /// (`x`, `y`) in the window, pressed, held for `held`, and let go: no
    /// element handed over.
    async fn pointer(&self, kind: &str, x: f64, y: f64, held: Duration) -> WebDriverResult<()> {
        // Whole pixels in the window, as WebDriver's pointer takes them.
        let (x, y) = (x.round() as i64, y.round() as i64);
        let held = u64::try_from(held.as_millis()).unwrap_or(300);
        let press = json!([{
            "type": "pointer",
            "id": kind,
            "parameters": { "pointerType": kind },
            "actions": [
                { "type": "pointerMove", "duration": 0, "origin": "viewport", "x": x, "y": y },
                { "type": "pointerDown", "button": 0 },
                { "type": "pause", "duration": held },
                { "type": "pointerUp", "button": 0 },
            ],
        }]);
        self.driver
            .handle()
            .cmd(Command::PerformActions(Actions::from(press)))
            .await
            .map(drop)
    }

    /// A move in place by the link `css`, to `path`, waited for.
    pub async fn move_to(&self, css: &str, path: &str) -> WebDriverResult<()> {
        self.press(css).await?;
        self.until(
            &format!("{path} in place"),
            &format!("return location.pathname + location.search === {path:?}"),
        )
        .await?;
        self.settled().await
    }

    /// A link to `href` added to `<main>` as `#e2e-link`, boosted as the
    /// page's own links are: htmx processes the region again.
    pub async fn add_link(&self, href: &str) -> WebDriverResult<()> {
        self.eval(&format!(
            "const a = document.createElement('a');
             a.href = {href:?}; a.id = 'e2e-link'; a.textContent = 'e2e';
             document.querySelector('main').append(a);
             htmx.process(document.getElementById('page'))"
        ))
        .await
        .map(drop)
    }

    /// Mark the document, so a later [`Page::in_place`] tells a move in
    /// place (the mark kept) from a full load (a new document, no mark).
    pub async fn mark(&self) -> WebDriverResult<()> {
        self.eval("window.__e2eMark = true").await.map(drop)
    }

    /// Whether the document marked by [`Page::mark`] is still the one shown.
    pub async fn in_place(&self) -> WebDriverResult<bool> {
        Ok(self.eval("return window.__e2eMark === true").await? == Value::Bool(true))
    }

    /// The page at a phone's width for a test of the phone's layout: a
    /// desktop browser's page sized to an iPhone's 390 by 844, a phone's
    /// screen as it is. A tablet's layout isn't the phone's, so there the
    /// test is skipped. Whether it may go on.
    pub async fn phone(&self) -> WebDriverResult<bool> {
        match self.browser.screen() {
            None => self.viewport(390, 844).await.map(|()| true),
            Some(Screen::Phone) => Ok(true),
            Some(screen) => {
                self.skip(&format!(
                    "a phone's test, this device's screen a {screen:?}'s"
                ));
                Ok(false)
            }
        }
    }

    /// A test that does not apply to this browser, said and kept: the
    /// reason on the test's output, and the test's name with it in the
    /// lane's `skipped.txt`, which `run.sh` counts apart from the passed, a
    /// test returning early being "ok" to the runner. The test then
    /// returns.
    pub fn skip(&self, reason: &str) {
        skipped(&self.dir, &self.test, self.browser, reason);
    }
}
