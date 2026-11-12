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

#[cfg(test)]
mod tests {
    use super::top_of;

    #[test]
    fn the_top_is_the_resumed_activitys_package() {
        let said = "  mFocusedApp=…\n  topResumedActivity=ActivityRecord{1a2b u0 com.example.app/.MainActivity t12}\n";
        assert_eq!(top_of(said), "com.example.app");
        assert_eq!(top_of("nothing resumed"), "");
    }
}
