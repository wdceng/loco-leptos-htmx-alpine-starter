//! The app lane's side outside the page: the Android app started afresh
//! for a test, what Android shows on top, its Back key, all through adb, as
//! `src-tauri/links.sh` taps. As an islands app on this stack drives its
//! own.

use std::{
    process::Command,
    time::{Duration, Instant},
};

use thirtyfour::prelude::{WebDriverError, WebDriverResult};

use crate::{Page, session::Run};

/// The app's id, Android's package (`E2E_APP`, `e2e/run.sh`, from
/// `src-tauri/tauri.conf.json`): the package [`Page::on_top`] names while
/// the app is in front.
#[must_use]
pub fn app_package() -> String {
    std::env::var("E2E_APP").unwrap_or_default()
}

/// The lane's adb (`E2E_ADB`).
fn adb_tool() -> String {
    std::env::var("E2E_ADB").unwrap_or_else(|_| "adb".to_owned())
}

/// `adb -s <device> <args>`'s output, or why it failed. Blocking: called
/// off the async threads.
fn adb(tool: &str, device: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(tool)
        .args(["-s", device])
        .args(args)
        .output()
        .map_err(|e| format!("adb {}: {e}", args.join(" ")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "adb {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// The app stopped and started from its launcher's activity, as a person
/// opens it, then waited for until its webview takes a driver: its process
/// up and its webview's debugging socket open
/// (`webview_devtools_remote_<pid>`, which the driver attaches through),
/// within 30 seconds.
pub(crate) async fn launch(run: &Run) -> Result<(), String> {
    let (tool, device, app) = (adb_tool(), run.device.clone(), app_package());
    tokio::task::spawn_blocking(move || {
        adb(&tool, &device, &["shell", "am", "force-stop", &app])?;
        let activity = format!("{app}/.MainActivity");
        adb(
            &tool,
            &device,
            &["shell", "am", "start", "-W", "-n", &activity],
        )?;
        let started = Instant::now();
        loop {
            let pid = adb(&tool, &device, &["shell", "pidof", &app]).unwrap_or_default();
            let pid = pid.split_whitespace().next().unwrap_or_default().to_owned();
            if !pid.is_empty() {
                let sockets = adb(&tool, &device, &["shell", "cat", "/proc/net/unix"])?;
                if sockets.contains(&format!("webview_devtools_remote_{pid}")) {
                    return Ok(());
                }
            }
            if started.elapsed() >= Duration::from_secs(30) {
                return Err(format!("{app}'s webview not up within 30 s"));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

impl Page {
    /// `args` to adb on the lane's device, off the async threads.
    async fn on_device(&self, args: &[&str]) -> WebDriverResult<String> {
        let (tool, device) = (adb_tool(), self.device.clone());
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
        tokio::task::spawn_blocking(move || {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            adb(&tool, &device, &args)
        })
        .await
        .map_err(|e| WebDriverError::RequestFailed(e.to_string()))?
        .map_err(WebDriverError::RequestFailed)
    }

    /// The device's airplane mode on or off: no network for the app, adb
    /// still reaching the device.
    pub async fn airplane(&self, on: bool) -> WebDriverResult<()> {
        let mode = if on { "enable" } else { "disable" };
        self.on_device(&["shell", "cmd", "connectivity", "airplane-mode", mode])
            .await
            .map(drop)
    }

    /// The device's scheme switched to the night's (`night`) or the day's,
    /// as the system's schedule does while the app runs.
    pub async fn night(&self, night: bool) -> WebDriverResult<()> {
        let mode = if night { "yes" } else { "no" };
        self.on_device(&["shell", "cmd", "uimode", "night", mode])
            .await
            .map(drop)
    }

    /// Whether the app's window asks for the system's buttons at the foot
    /// in their day look, dark on light (`dumpsys window`'s `apr`).
    pub async fn light_buttons(&self) -> WebDriverResult<bool> {
        let dump = self
            .on_device(&["shell", "dumpsys", "window", "windows"])
            .await?;
        lit_buttons(&dump, &app_package())
            .ok_or_else(|| WebDriverError::RequestFailed("the app's window not in dumpsys".into()))
    }

    /// Android's Back, as the system's own button or gesture.
    pub async fn system_back(&self) -> WebDriverResult<()> {
        self.on_device(&["shell", "input", "keyevent", "KEYCODE_BACK"])
            .await
            .map(drop)
    }

    /// The package Android shows on top, by its resumed activity; empty in
    /// the moment between two (as `src-tauri/links.sh` reads it).
    pub async fn on_top(&self) -> WebDriverResult<String> {
        let activities = self
            .on_device(&["shell", "dumpsys", "activity", "activities"])
            .await?;
        Ok(top_of(&activities))
    }

    /// Until `wanted` says yes to the package on top, within `within`; the
    /// package then.
    pub async fn until_on_top(
        &self,
        what: &str,
        wanted: impl Fn(&str) -> bool,
        within: Duration,
    ) -> WebDriverResult<String> {
        let started = Instant::now();
        loop {
            let top = self.on_top().await?;
            if wanted(&top) {
                return Ok(top);
            }
            if started.elapsed() >= within {
                return Err(WebDriverError::Timeout(format!(
                    "{what} not on top within {within:?}, {top:?} there"
                )));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

/// The package of the resumed activity in `dumpsys activity activities`'
/// answer; empty where none is.
fn top_of(activities: &str) -> String {
    activities
        .lines()
        .find(|line| line.contains("topResumedActivity"))
        .and_then(|line| line.split_whitespace().find(|word| word.contains('/')))
        .and_then(|component| component.split('/').next())
        .unwrap_or_default()
        .to_owned()
}

/// Android's `WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS`.
const LIGHT_NAVIGATION_BARS: u32 = 1 << 4;

/// Whether `app`'s main window in `dump` asks for light system buttons: its
/// `apr`, a hex number or the flags' names by Android's version, none
/// printed where it asks for nothing. None where the window isn't there.
fn lit_buttons(dump: &str, app: &str) -> Option<bool> {
    let window = dump
        .split("  Window #")
        .find(|window| window.contains(&format!("{app}/{app}.MainActivity")))?;
    let Some(apr) = window.split("apr=").nth(1) else {
        return Some(false);
    };
    let apr = apr.lines().next().unwrap_or_default();
    let number = apr.split_whitespace().next().unwrap_or_default();
    Some(match u32::from_str_radix(number, 16) {
        Ok(flags) => flags & LIGHT_NAVIGATION_BARS != 0,
        Err(_) => apr.contains("LIGHT_NAVIGATION_BARS"),
    })
}

#[cfg(test)]
mod tests {
    use super::{lit_buttons, top_of};

    /// The system's buttons' look read from the app's window alone, as a
    /// number or by the flags' names, none for nothing.
    #[test]
    fn the_buttons_look_is_the_apps_windows() {
        let app = "com.example.app";
        let other = "  Window #3 Window{1 u0 other/other.Main}:\n    mAttrs={ fl=1\n      apr=18\n";
        let numbered = format!(
            "  Window #9 Window{{2 u0 {app}/{app}.MainActivity}}:\n    mAttrs={{ fl=1\n      apr=18\n      bhv=1\n"
        );
        let named = format!(
            "  Window #9 Window{{2 u0 {app}/{app}.MainActivity}}:\n      apr=LIGHT_STATUS_BARS|LIGHT_NAVIGATION_BARS\n"
        );
        let dark = format!("  Window #9 Window{{2 u0 {app}/{app}.MainActivity}}:\n      apr=8\n");
        let none = format!("  Window #9 Window{{2 u0 {app}/{app}.MainActivity}}:\n      bhv=1\n");
        assert_eq!(lit_buttons(other, app), None);
        assert_eq!(lit_buttons(&format!("{other}{numbered}"), app), Some(true));
        assert_eq!(lit_buttons(&named, app), Some(true));
        assert_eq!(lit_buttons(&dark, app), Some(false));
        assert_eq!(lit_buttons(&none, app), Some(false));
    }

    #[test]
    fn the_top_is_the_resumed_activitys_package() {
        let said = "  mFocusedApp=…\n  topResumedActivity=ActivityRecord{1a2b u0 com.example.app/.MainActivity t12}\n";
        assert_eq!(top_of(said), "com.example.app");
        assert_eq!(top_of("nothing resumed"), "");
    }
}
