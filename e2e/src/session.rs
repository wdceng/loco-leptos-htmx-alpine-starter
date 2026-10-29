//! A test's session: where the run's parts listen, the session opened and
//! closed whatever the test does, its time limit, what a failed test's
//! page showed, and a test that does not apply said and kept.
//!
//! A lane whose driver opens no session any more is marked broken after two
//! different tests in a row could not open one (as an islands app on this
//! stack does it): every later test of the lane fails at once, saying since
//! when and why, where each spent a whole attempt and its retry, and the
//! lane's summary says it once (e2e/run.sh). A failure once a test has its
//! session never counts: only the opening does, so a lane that runs and
//! fails tests is left alone.

use std::{future::Future, panic, time::Duration};

use thirtyfour::prelude::*;

use crate::{Browser, Page};

/// Where the run's parts listen, from `e2e/run.sh`.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    pub(crate) browser: Browser,
    pub(crate) base: String,
    pub(crate) driver: String,
    pub(crate) dir: String,
    pub(crate) headed: bool,
    /// The device a device's lane started (`E2E_DEVICE`): the Simulator's
    /// UDID or the emulator's adb serial; empty for a desktop browser.
    pub(crate) device: String,
    /// The browser a low-bar or beta lane starts instead of the installed
    /// one (`E2E_BINARY`), none for every other lane.
    pub(crate) binary: Option<String>,
}

impl Run {
    /// Read from the environment `e2e/run.sh` sets; a test run without it
    /// is refused with the way to start one.
    fn from_env() -> Self {
        let var = |name: &str| {
            std::env::var(name).unwrap_or_else(|_| {
                panic!("{name} is not set: run the browser tests with e2e/run.sh <browser>")
            })
        };
        Self {
            browser: Browser::from_name(&var("E2E_BROWSER")),
            base: var("E2E_BASE"),
            driver: var("E2E_DRIVER"),
            dir: var("E2E_DIR"),
            headed: std::env::var_os("E2E_HEADED").is_some(),
            device: std::env::var("E2E_DEVICE").unwrap_or_default(),
            binary: std::env::var("E2E_BINARY")
                .ok()
                .filter(|binary| !binary.is_empty()),
        }
    }
}

/// The running test's name: the runner names a test's thread after it,
/// the thread [`browse`] starts on (its body then runs on a worker's).
fn test_name() -> String {
    std::thread::current().name().unwrap_or("a test").to_owned()
}

/// A skipped test said on its output and kept, its name and `reason`, in
/// `dir`'s `skipped.txt` ([`Page::skip`]).
pub(crate) fn skipped(dir: &str, test: &str, browser: Browser, reason: &str) {
    use std::io::Write;
    eprintln!("{browser:?}: {reason}");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{dir}/skipped.txt"))
    {
        let _ = writeln!(file, "{test}: {reason}");
    }
}

/// Open a session in the run's browser, run `test` with its page, and close
/// the session whatever happened: a failure or a panic is passed on after
/// the session is gone. A test that passed still fails when its page logged
/// an error, a warning or a refused resource, where the browser lets the
/// harness hear them ([`Page::allow_failed_load`] names the expected), and
/// a failure of either kind keeps what the page showed.
pub async fn browse<F, Fut>(test: F)
where
    F: FnOnce(Page) -> Fut,
    Fut: Future<Output = WebDriverResult<()>> + Send + 'static,
{
    let run = Run::from_env();
    let name = test_name();
    // A lane marked broken: at once, saying since when and why.
    if let Ok(since) = std::fs::read_to_string(format!("{}/{NO_SESSION}", run.dir)) {
        panic!(
            "{:?}: no session on this lane since {}",
            run.browser,
            since.trim()
        );
    }
    let session = || {
        WebDriver::new(
            run.driver.as_str(),
            run.browser
                .capabilities(run.headed, &run.device, run.binary.as_deref()),
        )
    };
    let opened = match session().await {
        Ok(driver) => Ok(driver),
        // Safari still paired with a session that never closed (its test
        // lost the driver mid-request) refuses every new one. Safari quit,
        // in that Simulator or on the Mac (the run's own, e2e/run.sh
        // refusing to start beside an open one), frees it, and the session
        // is asked again.
        Err(e) if run.browser.is_safari() && e.to_string().contains("already paired") => {
            eprintln!(
                "{:?}: Safari still paired, quit and asked again",
                run.browser
            );
            if run.browser.is_simulator() {
                let _ = std::process::Command::new("xcrun")
                    .args(["simctl", "terminate", &run.device, "com.apple.mobilesafari"])
                    .status();
            } else {
                let _ = std::process::Command::new("osascript")
                    .args(["-e", "quit app \"Safari\""])
                    .status();
                tokio::time::sleep(Duration::from_secs(1)).await;
                let _ = std::process::Command::new("pkill")
                    .args(["-x", "Safari"])
                    .status();
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            session().await.map_err(|e| e.to_string())
        }
        // An emulator's Chrome gone after a long run of pages ("chrome not
        // reachable"). Chrome stopped on the device starts afresh with the
        // next session.
        Err(e) if run.browser.is_android() && e.to_string().contains("chrome not reachable") => {
            eprintln!(
                "{:?}: Chrome not reachable, stopped and asked again",
                run.browser
            );
            let adb = std::env::var("E2E_ADB").unwrap_or_else(|_| "adb".to_owned());
            let _ = std::process::Command::new(adb)
                .args([
                    "-s",
                    &run.device,
                    "shell",
                    "am",
                    "force-stop",
                    "com.android.chrome",
                ])
                .status();
            tokio::time::sleep(Duration::from_secs(2)).await;
            session().await.map_err(|e| e.to_string())
        }
        // Chrome or Edge not up within its driver's minute
        // ("DevToolsActivePort file doesn't exist"), too slow to start with
        // every lane starting at once. Asked once more after a pause; a
        // browser that cannot start at all fails again.
        Err(e) if run.browser.has_devtools() && e.to_string().contains("DevToolsActivePort") => {
            eprintln!(
                "{:?}: the browser did not start within its driver's wait, asked again",
                run.browser
            );
            tokio::time::sleep(Duration::from_secs(10)).await;
            session().await.map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    };
    let driver = match opened {
        Ok(driver) => {
            session_opened(&run.dir);
            driver
        }
        Err(reason) => {
            session_failed(&run.dir, &name, &reason);
            panic!("{:?} session: {reason}", run.browser)
        }
    };
    // Chrome and Edge open at their size (`--window-size`), a device's
    // screen is its own; Firefox's and Safari's windows are sized here.
    let sized = if run.browser.has_devtools() || run.browser.screen().is_some() {
        Ok(())
    } else {
        driver.set_window_rect(0, 0, 1280, 900).await
    };
    let page = match sized {
        Ok(()) => Page::start(driver.clone(), &run, &name).await,
        Err(e) => Err(e),
    };
    let page = match page {
        Ok(page) => page,
        Err(e) => {
            let _ = driver.quit().await;
            panic!("{:?}: the page's watchers: {e}", run.browser);
        }
    };
    let running = tokio::spawn(test(page.clone()));
    let stop = running.abort_handle();
    let outcome = match tokio::time::timeout(TEST_WITHIN, running).await {
        Ok(joined) => joined,
        Err(_) => {
            stop.abort();
            Ok(Err(WebDriverError::Timeout(format!(
                "the test ran past {TEST_WITHIN:?}, stopped"
            ))))
        }
    };
    // What failed, said; a panic's payload kept to be passed on.
    let mut failed = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some((e.to_string(), None)),
        Err(join) => {
            let payload = join.into_panic();
            let said = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "a panic".to_owned());
            Some((said, Some(payload)))
        }
    };
    // The page's own errors fail the test too, and join a failure's words:
    // a console error is often why the test failed.
    let problems = page.problems();
    if !problems.is_empty() {
        let logged = format!("the page logged:\n{}", problems.join("\n"));
        failed = Some(match failed {
            None => (logged, None),
            Some((said, payload)) => (format!("{said}\n\n{logged}"), payload),
        });
    }
    if let Some((said, _)) = &failed {
        evidence(&driver, &run.dir, &name, said).await;
    }
    page.stop();
    // A driver that hangs on the closing too (a hung test's) is left: the
    // next session's own recovery frees the browser.
    match tokio::time::timeout(Duration::from_secs(15), driver.quit()).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("closing the {:?} session: {e}", run.browser),
        Err(_) => eprintln!("closing the {:?} session: no answer", run.browser),
    }
    match failed {
        None => {}
        // The panic's own message is in its payload; the rest said here.
        Some((said, Some(payload))) => {
            eprintln!("{said}");
            panic::resume_unwind(payload)
        }
        Some((said, None)) => panic!("{said}"),
    }
}

/// The lane's file of the tests whose session would not open since the
/// last one that did, one name a line: a file, since nextest runs each test
/// in a process of its own.
const FAILED_SESSIONS: &str = "session-failures";

/// The lane's mark that its driver opens no session any more: the time it
/// was written and the driver's last words. `e2e/run.sh` takes it away when
/// a lane starts, so one run's mark never stops the next.
const NO_SESSION: &str = "no-session";

/// How many different tests in a row may fail to open a session before the
/// lane is marked broken: two, since nextest tries a failed test once more
/// and one test's two tries could be one hiccup of the driver's.
const BROKEN_AFTER: usize = 2;

/// A session opened: the tests that failed to open one before it no longer
/// count.
fn session_opened(dir: &str) {
    let _ = std::fs::remove_file(format!("{dir}/{FAILED_SESSIONS}"));
}

/// `test`'s session would not open, the driver saying `reason`: counted, and
/// the lane marked broken once [`BROKEN_AFTER`] different tests in a row
/// could not.
fn session_failed(dir: &str, test: &str, reason: &str) {
    let path = format!("{dir}/{FAILED_SESSIONS}");
    let kept = std::fs::read_to_string(&path).unwrap_or_default();
    let (tests, broken) = failed_in_a_row(&kept, test);
    let _ = std::fs::write(&path, tests);
    if broken {
        let _ = std::fs::write(format!("{dir}/{NO_SESSION}"), mark(&now(), reason));
        eprintln!("this lane opens no session any more: every later test fails at once");
    }
}

/// The tests that failed to open a session in a row, as the file `kept`
/// them, with `test` added once, and whether as many different ones as
/// [`BROKEN_AFTER`] have.
fn failed_in_a_row(kept: &str, test: &str) -> (String, bool) {
    let mut tests: Vec<&str> = kept.lines().filter(|line| !line.is_empty()).collect();
    if !tests.contains(&test) {
        tests.push(test);
    }
    let broken = tests.len() >= BROKEN_AFTER;
    (format!("{}\n", tests.join("\n")), broken)
}

/// What the broken lane's mark says: `at`, and the driver's words, their
/// first line, at most 300 characters.
fn mark(at: &str, reason: &str) -> String {
    let said: String = reason
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(300)
        .collect();
    format!("{at}: {said}")
}

/// The time now, as `date` writes it; nothing where it will not.
fn now() -> String {
    std::process::Command::new("date")
        .arg("+%H:%M:%S")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|out| out.trim().to_owned())
        .unwrap_or_default()
}

/// How long one test may run; the memory tests take about half a minute. A
/// test past it is stopped and fails; `e2e/.config/nextest.toml` ends the
/// process a while after, should even this not come back.
const TEST_WITHIN: Duration = Duration::from_secs(180);

/// How many of the server log's last lines a failure keeps: a test's last
/// few requests, each a few lines.
const SERVER_LINES: usize = 80;

/// What the page showed when its test failed, kept in `dir`'s `failures/`
/// for a look instead of a rerun: a screenshot, the page's HTML, its
/// address with what failed, and the server log's last lines. Each step
/// within a few seconds, the driver perhaps the reason for the failure;
/// whatever it will not give is left out.
async fn evidence(driver: &WebDriver, dir: &str, test: &str, said: &str) {
    let within = Duration::from_secs(10);
    let folder = format!("{dir}/failures");
    if std::fs::create_dir_all(&folder).is_err() {
        return;
    }
    // `moves::a_link_moves…` as `moves.a_link_moves…`.
    let base = format!("{folder}/{}", test.replace("::", "."));
    let url = match tokio::time::timeout(within, driver.current_url()).await {
        Ok(Ok(url)) => url.to_string(),
        _ => "(no answer)".to_owned(),
    };
    let _ = std::fs::write(format!("{base}.txt"), format!("{url}\n\n{said}\n"));
    if let Ok(Ok(png)) = tokio::time::timeout(within, driver.screenshot_as_png()).await {
        let _ = std::fs::write(format!("{base}.png"), png);
    }
    if let Ok(Ok(html)) = tokio::time::timeout(within, driver.source()).await {
        let _ = std::fs::write(format!("{base}.html"), html);
    }
    if let Ok(log) = std::fs::read_to_string(format!("{dir}/server.log")) {
        let lines: Vec<&str> = log.lines().collect();
        let tail = lines[lines.len().saturating_sub(SERVER_LINES)..].join("\n");
        let _ = std::fs::write(format!("{base}.server.log"), tail);
    }
    eprintln!("what the page showed: {base}.png, .html, .txt, .server.log");
}

#[cfg(test)]
mod tests {
    use super::{failed_in_a_row, mark};

    #[test]
    fn two_different_tests_in_a_row_mark_the_lane() {
        // One test, both its tries: not yet.
        let (kept, broken) = failed_in_a_row("", "a");
        assert!(!broken);
        let (kept, broken) = failed_in_a_row(&kept, "a");
        assert_eq!(kept, "a\n");
        assert!(!broken, "one test's retry is one hiccup");
        // A second test: broken.
        let (kept, broken) = failed_in_a_row(&kept, "b");
        assert_eq!(kept, "a\nb\n");
        assert!(broken);
    }

    #[test]
    fn the_mark_says_when_and_the_drivers_first_line() {
        assert_eq!(
            mark("14:32:05", "no session hosts\nmore lines"),
            "14:32:05: no session hosts"
        );
        assert_eq!(mark("14:32:05", &"x".repeat(400)).len(), 10 + 300);
    }
}
