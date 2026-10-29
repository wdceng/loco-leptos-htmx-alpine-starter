//! The browsers a run drives, as an islands app on this stack drives them.

use serde_json::Value;
use thirtyfour::prelude::Capabilities;

/// The browsers the run drives, by the name `e2e/run.sh` takes (its lane's,
/// `E2E_BROWSER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Edge,
    Firefox,
    Safari,
}

impl Browser {
    /// The browser of a lane's name. A low-bar lane (`chrome-low`,
    /// `firefox-low`) is its top lane's browser on the oldest version the
    /// site supports (docs/testing.md, "The Low Bar"): the same browser to
    /// the tests, another binary to `run.sh`.
    pub(crate) fn from_name(name: &str) -> Self {
        match name.strip_suffix("-low").unwrap_or(name) {
            "chrome" => Self::Chrome,
            "edge" => Self::Edge,
            "firefox" => Self::Firefox,
            "safari" => Self::Safari,
            other => panic!("unknown browser {other}: run the browser tests with e2e/run.sh"),
        }
    }

    /// Whether it has Chrome's DevTools: what a test that emulates the
    /// screen or the network, changes an answer or measures memory needs.
    /// The others skip those tests, saying why.
    #[must_use]
    pub fn has_devtools(self) -> bool {
        matches!(self, Self::Chrome | Self::Edge)
    }

    /// The session's capabilities: headless where the browser has it, so a
    /// run takes no window from the desktop, unless `E2E_HEADED` asks to
    /// watch; Safari has no headless mode. `binary` the browser a low-bar
    /// lane starts instead of the installed one (`E2E_BINARY`).
    pub(crate) fn capabilities(self, headed: bool, binary: Option<&str>) -> Capabilities {
        let json = match self {
            Self::Chrome | Self::Edge => {
                let key = if self == Self::Chrome {
                    "goog:chromeOptions"
                } else {
                    "ms:edgeOptions"
                };
                let mut args = vec!["--window-size=1280,900"];
                if !headed {
                    args.push("--headless=new");
                }
                let mut options = serde_json::json!({ "args": args });
                if let Some(binary) = binary {
                    options["binary"] = binary.into();
                }
                serde_json::json!({
                    "browserName": if self == Self::Chrome { "chrome" } else { "MicrosoftEdge" },
                    key: options,
                })
            }
            // WebDriver BiDi on (`webSocketUrl`): how the page's errors are
            // heard (src/devtools.rs).
            Self::Firefox => {
                let args: &[&str] = if headed { &[] } else { &["-headless"] };
                let mut options = serde_json::json!({ "args": args });
                if let Some(binary) = binary {
                    options["binary"] = binary.into();
                }
                serde_json::json!({
                    "browserName": "firefox",
                    "webSocketUrl": true,
                    "moz:firefoxOptions": options,
                })
            }
            Self::Safari => serde_json::json!({ "browserName": "safari" }),
        };
        match json {
            Value::Object(map) => Capabilities::from(map),
            _ => unreachable!("the capabilities are an object"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Browser, Value};

    #[test]
    fn a_low_lane_is_its_top_lanes_browser() {
        assert_eq!(Browser::from_name("chrome-low"), Browser::Chrome);
        assert_eq!(Browser::from_name("firefox-low"), Browser::Firefox);
        assert_eq!(Browser::from_name("edge"), Browser::Edge);
        assert_eq!(Browser::from_name("safari"), Browser::Safari);
    }

    #[test]
    fn a_low_lane_starts_its_own_binary() {
        let caps = Value::from(Browser::Chrome.capabilities(false, Some("/old/chrome")));
        assert_eq!(caps["goog:chromeOptions"]["binary"], "/old/chrome");
        let caps = Value::from(Browser::Firefox.capabilities(true, None));
        assert_eq!(caps["webSocketUrl"], true, "BiDi on, to hear the page");
        assert_eq!(caps["moz:firefoxOptions"]["args"], serde_json::json!([]));
    }
}
