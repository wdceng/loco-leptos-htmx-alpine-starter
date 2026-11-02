//! What the browsers' own tools give a test: Chrome's and Edge's DevTools
//! over their own socket (thirtyfour's `cdp-events`; the driver passes
//! commands on but no events), WebDriver BiDi's log in Firefox, and
//! axe-core's accessibility audit. Through DevTools: what the page logs,
//! what the page holds in memory, the window's exact size, the visitor's
//! settings emulated, a file refused and a script run before the page's
//! own. Firefox's BiDi hears what the page logs and runs that script;
//! Safari's driver has neither, so there a page's errors go unheard. An
//! answer is changed in the page itself, in every browser
//! ([`Page::answer_with`]).

use std::sync::{Arc, Mutex, PoisonError};

use futures_util::StreamExt;
use serde_json::{Value, json};
use thirtyfour::bidi::events::LogEntryAdded;
use thirtyfour::bidi::modules::log::LogLevel;
use thirtyfour::cdp::CdpSession;
use thirtyfour::common::command::{Command, ExtensionCommand};
use thirtyfour::prelude::*;
use tokio::task::AbortHandle;

use crate::{Browser, Page, session::Run};

/// A DevTools command sent to msedgedriver's own address,
/// `/ms/cdp/execute`, for the driver's own session: thirtyfour's `cdp()`
/// posts to Chrome's, `/goog/cdp/execute`, which msedgedriver answers
/// "unknown command" (as an islands app on this stack found).
#[derive(Debug)]
struct EdgeDevTools {
    method: String,
    params: Value,
}

impl ExtensionCommand for EdgeDevTools {
    fn parameters_json(&self) -> Option<Value> {
        Some(json!({ "cmd": self.method, "params": self.params }))
    }

    fn method(&self) -> http::Method {
        http::Method::POST
    }

    fn endpoint(&self) -> Arc<str> {
        Arc::from("/ms/cdp/execute")
    }
}

/// axe-core, byte for byte from its npm package, with its MPL-2.0 licence
/// beside it (`e2e/vendor/`).
const AXE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/axe-core-4.14.0.min.js");

/// What the page holds after a garbage collection ([`Page::held`]).
#[derive(Debug, Clone, Copy)]
pub struct Held {
    /// Documents: a page swapped out but kept is one more.
    pub documents: u64,
    /// Event listeners, all of the page's.
    pub listeners: u64,
    /// The JavaScript heap in use, in bytes.
    pub heap: f64,
    /// Detached element trees something still keeps, and their nodes:
    /// Chrome's count of all nodes includes those Blink hasn't freed yet.
    pub trees: usize,
    pub detached: usize,
    /// The WASM's memory, in bytes (the debug build's `__wasmMemory`,
    /// src/lib.rs). It only grows, in 64 KiB pages.
    pub wasm: u64,
}

/// What the watcher heard, shared with the page.
#[derive(Debug, Default)]
pub(crate) struct Heard {
    problems: Vec<String>,
    /// Failed loads a test causes on purpose: the status, and the full
    /// address Chrome's line ends with ([`Page::allow_failed_load`]).
    allowed: Vec<(u16, String)>,
    /// Whether the test expects the page's errors ([`Page::allow_errors`]).
    errors_expected: bool,
}

/// The shared state, locked briefly and never across an await.
pub(crate) type Shared = Arc<Mutex<Heard>>;

fn lock(heard: &Shared) -> std::sync::MutexGuard<'_, Heard> {
    heard.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Page {
    /// The page on `driver`, listened to where the browser lets it be: in
    /// Chrome and Edge their DevTools session, the page's errors, warnings
    /// and refused resources noted; in Firefox BiDi's log, its errors and
    /// warnings noted; in Safari and on the devices nothing.
    pub(crate) async fn start(driver: WebDriver, run: &Run, test: &str) -> WebDriverResult<Self> {
        let heard = Shared::default();
        let (cdp, watcher) = match run.browser {
            Browser::Chrome | Browser::Edge => {
                let cdp = driver.cdp().connect().await?;
                // Subscribed before the domains are on, so nothing is missed.
                let watcher = watch(cdp.clone(), heard.clone());
                (Some(cdp), Some(watcher))
            }
            Browser::Firefox => (None, Some(listen(&driver, heard.clone()).await?)),
            // Safari's driver, on the Mac or in a Simulator, and Chrome's
            // on an emulator pass no page's errors on.
            _ => (None, None),
        };
        let page = Self {
            driver,
            browser: run.browser,
            cdp,
            base: run.base.clone(),
            dir: run.dir.clone(),
            test: test.to_owned(),
            heard,
            watcher,
        };
        if page.browser.has_devtools() {
            // Once a session: `held` collects and reads with them.
            for domain in [
                "Runtime.enable",
                "Log.enable",
                "HeapProfiler.enable",
                "Performance.enable",
            ] {
                page.devtools(domain, json!({})).await?;
            }
        }
        Ok(page)
    }

    /// The watcher stopped, with the session.
    pub(crate) fn stop(&self) {
        if let Some(watcher) = &self.watcher {
            watcher.abort();
        }
    }

    /// What the page logged that wasn't allowed.
    pub(crate) fn problems(&self) -> Vec<String> {
        let heard = lock(&self.heard);
        if heard.errors_expected {
            return Vec::new();
        }
        heard
            .problems
            .iter()
            .filter(|problem| {
                !heard.allowed.iter().any(|(status, url)| {
                    problem.contains(&format!("status of {status} "))
                        && problem.ends_with(&format!(" {url}"))
                })
            })
            .cloned()
            .collect()
    }

    /// A failed load the test causes on purpose, not counted: Chrome's
    /// "status of `status`" line for `path` on the run's site, that page
    /// alone, a 404 asked for or a 502 faked.
    pub fn allow_failed_load(&self, status: u16, path: &str) {
        let url = self.url(path);
        lock(&self.heard).allowed.push((status, url));
    }

    /// The page's errors expected for the rest of the test, so they don't
    /// fail it: a test that refuses one of the site's scripts, whose
    /// dependants then say so ([`Page::block`]).
    pub fn allow_errors(&self) {
        lock(&self.heard).errors_expected = true;
    }

    /// The files whose addresses match `patterns` (`*` any run of
    /// characters) refused from now on, as a blocker or a failed download
    /// would. Chrome's and Edge's alone, through DevTools: a test skips the
    /// others first.
    pub async fn block(&self, patterns: &[&str]) -> WebDriverResult<()> {
        self.devtools("Network.enable", json!({})).await?;
        self.devtools("Network.setBlockedURLs", json!({ "urls": patterns }))
            .await
            .map(drop)
    }

    /// Run `source` on every page this session loads from now on, before
    /// the page's own scripts: DevTools' `Page.addScriptToEvaluateOnNewDocument`
    /// in Chrome and Edge, on the driver's own session (sent on our socket,
    /// it never ran: a probe, 2026-10-29), WebDriver BiDi's
    /// `script.addPreloadScript` in Firefox. Safari's driver and the
    /// devices' have neither.
    pub async fn preload(&self, source: &str) -> WebDriverResult<()> {
        if self.browser.has_devtools() {
            return self
                .driver_devtools(
                    "Page.addScriptToEvaluateOnNewDocument",
                    json!({ "source": source }),
                )
                .await
                .map(drop);
        }
        let bidi = self.driver.bidi().await?;
        bidi.script()
            .add_preload_script(format!("() => {{ {source} }}"))
            .await
            .map(drop)
            .map_err(|e| WebDriverError::RequestFailed(format!("a preload script: {e}")))
    }

    /// The page's size in CSS pixels, as near as the browser goes: exactly
    /// in Chrome and Edge (DevTools' device metrics); in Firefox and Safari
    /// the window sized round it, whose windows have a least width of their
    /// own, so a test reads the width it got where it matters; on a device
    /// nothing, its screen being its own.
    pub async fn viewport(&self, width: u32, height: u32) -> WebDriverResult<()> {
        // A device's screen is its own: the test reads what it has.
        if self.browser.screen().is_some() {
            return Ok(());
        }
        if self.browser.has_devtools() {
            return self
                .devtools(
                    "Emulation.setDeviceMetricsOverride",
                    json!({ "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false }),
                )
                .await
                .map(drop);
        }
        let frame = self
            .eval("return [window.outerWidth - window.innerWidth, window.outerHeight - window.innerHeight]")
            .await?;
        let side =
            |i: usize| u32::try_from(frame[i].as_u64().unwrap_or_default()).unwrap_or_default();
        self.driver
            .set_window_rect(0, 0, width + side(0), height + side(1))
            .await
    }

    /// A media feature as the visitor's settings would set it, from now on
    /// (`prefers-reduced-motion`, `reduce`).
    pub async fn emulate(&self, feature: &str, value: &str) -> WebDriverResult<()> {
        self.devtools(
            "Emulation.setEmulatedMedia",
            json!({ "features": [{ "name": feature, "value": value }] }),
        )
        .await
        .map(drop)
    }

    /// The accessibility audit of the open page, axe-core's (vendored in
    /// `e2e/vendor/`, injected by the test, never served by the site): each
    /// rule broken, its impact and the first elements breaking it.
    pub async fn axe(&self) -> WebDriverResult<Vec<Value>> {
        let source = std::fs::read_to_string(AXE)
            .map_err(|e| WebDriverError::RequestFailed(format!("{AXE}: {e}")))?;
        self.driver.execute(&source, Vec::new()).await?;
        let found = self
            .driver
            .execute_async(
                "const done = arguments[arguments.length - 1];
                 axe.run(document, { resultTypes: ['violations'] })
                   .then(r => done(r.violations.map(v => ({ id: v.id, impact: v.impact,
                       help: v.help, nodes: v.nodes.slice(0, 3).map(n => n.target.join(' ') + ' | '
                           + n.html.slice(0, 120) + ' | ' + (n.any?.[0]?.message ?? '')) }))),
                         e => done([{ id: 'axe-error', help: String(e) }]))",
                Vec::new(),
            )
            .await?;
        Ok(found.json().as_array().cloned().unwrap_or_default())
    }

    /// Whether this Chrome's DevTools can list detached DOM nodes, which
    /// [`Page::held`] needs: Chrome 114's can't (the low bar). Asked of the
    /// browser, not by version.
    pub async fn can_count_detached(&self) -> WebDriverResult<bool> {
        self.dom_ready().await?;
        Ok(self
            .devtools("DOM.getDetachedDomNodes", json!({}))
            .await
            .is_ok())
    }

    /// The DOM domain on, with the current document requested: what
    /// `DOM.getDetachedDomNodes` needs before it answers.
    async fn dom_ready(&self) -> WebDriverResult<()> {
        self.devtools("DOM.enable", json!({})).await?;
        self.devtools("DOM.getDocument", json!({ "depth": 0 }))
            .await
            .map(drop)
    }

    /// What the page holds once the garbage is collected.
    pub async fn held(&self) -> WebDriverResult<Held> {
        self.devtools("HeapProfiler.collectGarbage", json!({}))
            .await?;
        let counters = self.devtools("Memory.getDOMCounters", json!({})).await?;
        let metrics = self.devtools("Performance.getMetrics", json!({})).await?;
        let heap = metrics["metrics"]
            .as_array()
            .and_then(|all| all.iter().find(|m| m["name"] == "JSHeapUsedSize"))
            .and_then(|m| m["value"].as_f64())
            .unwrap_or_default();
        self.dom_ready().await?;
        let found = self.devtools("DOM.getDetachedDomNodes", json!({})).await?;
        // Elements only (node type 1). With chromedriver attached, Chrome
        // keeps a swapped-out region's attribute nodes for a while: forty
        // after ten moves, none after twenty, and none at all without
        // chromedriver (a probe, 2026-10-18). A kept page is an element.
        let trees: Vec<Value> = found["detachedNodes"]
            .as_array()
            .map(|all| {
                all.iter()
                    .filter(|tree| tree["treeNode"]["nodeType"] == 1)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let detached = trees
            .iter()
            .map(|tree| tree["retainedNodeIds"].as_array().map_or(0, Vec::len))
            .sum();
        let wasm = self
            .eval("return window.__wasmMemory?.buffer.byteLength ?? 0")
            .await?;
        let count = |name: &str| counters[name].as_u64().unwrap_or_default();
        Ok(Held {
            documents: count("documents"),
            listeners: count("jsEventListeners"),
            heap,
            trees: trees.len(),
            detached,
            wasm: wasm.as_u64().unwrap_or_default(),
        })
    }

    /// A DevTools command on the page's session, Chrome's and Edge's alone:
    /// a test that needs one skips the other browsers first
    /// ([`Browser::has_devtools`]).
    pub async fn devtools(&self, method: &str, params: Value) -> WebDriverResult<Value> {
        let Some(cdp) = &self.cdp else {
            return Err(WebDriverError::RequestFailed(format!(
                "{method}: no DevTools in {:?}",
                self.browser
            )));
        };
        cdp.send_raw(method, params)
            .await
            .map_err(|e| WebDriverError::RequestFailed(format!("{method}: {e}")))
    }

    /// A DevTools command on the driver's own session rather than the
    /// page's socket: what the driver itself keeps (its browser log).
    /// Edge's goes to msedgedriver's own address ([`EdgeDevTools`]).
    pub async fn driver_devtools(&self, method: &str, params: Value) -> WebDriverResult<Value> {
        match self.browser {
            Browser::Chrome => self.driver.cdp().send_raw(method, params).await,
            Browser::Edge => self
                .driver
                .handle()
                .cmd(Command::ExtensionCommand(Box::new(EdgeDevTools {
                    method: method.to_owned(),
                    params,
                })))
                .await?
                .value(),
            other => Err(WebDriverError::RequestFailed(format!(
                "{method}: no DevTools in {other:?}"
            ))),
        }
    }
}

/// Listen to Firefox's log over WebDriver BiDi until stopped: the page's
/// errors and warnings, a script's and the console's, noted in `heard`.
/// BiDi has no CSP refusal or failed load of its own to report, so those go
/// unheard here; Chrome's and Edge's lanes hear them.
async fn listen(driver: &WebDriver, heard: Shared) -> WebDriverResult<AbortHandle> {
    let bidi = driver.bidi().await?;
    let mut entries = bidi
        .subscribe::<LogEntryAdded>()
        .await
        .map_err(|e| WebDriverError::RequestFailed(format!("BiDi's log: {e}")))?;
    Ok(tokio::spawn(async move {
        // The connection kept open while the log is read.
        let _bidi = bidi;
        while let Some(entry) = entries.next().await {
            let level = match entry.level {
                LogLevel::Error => "error",
                LogLevel::Warn => "warning",
                _ => continue,
            };
            note(
                &heard,
                format!(
                    "{}.{level}: {}{}",
                    entry.entry_type,
                    entry.text.as_deref().unwrap_or(""),
                    frames(&entry.stack_trace)
                ),
            );
        }
    })
    .abort_handle())
}

/// Where a BiDi log entry came from: each of its stack's frames on a line
/// of its own, `at function (url:line:column)`, counted from 1 (BiDi counts
/// from 0). Nothing when Firefox captured no stack. Without it an error
/// under load on the M1 said only its message, no script (2026-10-31).
fn frames(stack: &serde_json::Value) -> String {
    stack["callFrames"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|frame| {
            let number = |key: &str| frame[key].as_u64().map_or(0, |n| n + 1);
            format!(
                "\n    at {} ({}:{}:{})",
                frame["functionName"]
                    .as_str()
                    .filter(|name| !name.is_empty())
                    .unwrap_or("?"),
                frame["url"].as_str().unwrap_or("?"),
                number("lineNumber"),
                number("columnNumber")
            )
        })
        .collect()
}

/// Listen to the session's events until stopped: problems noted in
/// `heard`.
fn watch(cdp: CdpSession, heard: Shared) -> AbortHandle {
    let mut events = cdp.subscribe_all();
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            let params = &event.params;
            match event.method.as_str() {
                "Runtime.exceptionThrown" => {
                    let details = &params["exceptionDetails"];
                    let said = details["exception"]["description"]
                        .as_str()
                        .or_else(|| details["text"].as_str())
                        .unwrap_or("?");
                    note(&heard, format!("exception: {said}"));
                }
                "Runtime.consoleAPICalled"
                    if matches!(params["type"].as_str(), Some("error" | "warning")) =>
                {
                    let said: Vec<String> = params["args"]
                        .as_array()
                        .map(|args| {
                            args.iter()
                                .map(|a| match &a["value"] {
                                    Value::String(s) => s.clone(),
                                    Value::Null => {
                                        a["description"].as_str().unwrap_or("").to_owned()
                                    }
                                    other => other.to_string(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    note(
                        &heard,
                        format!(
                            "console.{}: {}",
                            params["type"].as_str().unwrap_or("?"),
                            said.join(" ")
                        ),
                    );
                }
                // CSP refusals and failed loads land here.
                "Log.entryAdded" => {
                    let entry = &params["entry"];
                    if matches!(entry["level"].as_str(), Some("error" | "warning")) {
                        note(
                            &heard,
                            format!(
                                "log.{}: {} {}",
                                entry["level"].as_str().unwrap_or("?"),
                                entry["text"].as_str().unwrap_or(""),
                                entry["url"].as_str().unwrap_or("")
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
    })
    .abort_handle()
}

fn note(heard: &Shared, problem: String) {
    lock(heard).problems.push(problem);
}
