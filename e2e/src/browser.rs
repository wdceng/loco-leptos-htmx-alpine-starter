//! The browsers a run drives, as an islands app on this stack drives them:
//! the desktop's four, and the devices', Safari in an iOS Simulator and
//! Chrome in an Android emulator, each device `run.sh` started
//! (`E2E_DEVICE`, its id).

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
    /// Safari on an iPhone Simulator, through safaridriver.
    Iphone,
    /// Safari on an iPad Simulator, through safaridriver.
    Ipad,
    /// Safari on an iPad Air Simulator held upright, 820 pixels: the
    /// middle layout, which no other device shows.
    IpadAir,
    /// Chrome on an Android phone emulator, through chromedriver and adb.
    Android,
    /// Chrome on an Android tablet emulator, through chromedriver and adb.
    AndroidTablet,
}

/// A device's screen, which no window resizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// A phone held upright: the phone's layout.
    Phone,
    /// A tablet, a desktop's width or more: the desktop's layout.
    Tablet,
    /// A tablet held upright under the desktop's width: the middle layout.
    Middle,
}

impl Browser {
    /// The browser of a lane's name. A low-bar lane (`chrome-low`,
    /// `iphone-low`…) is its top lane's browser on the oldest version the
    /// site supports, a beta lane (`chrome-beta`…) on the browser's next
    /// release (docs/testing.md): the same browser to the tests, another
    /// binary or device to `run.sh`.
    pub(crate) fn from_name(name: &str) -> Self {
        let top = name
            .strip_suffix("-low")
            .or_else(|| name.strip_suffix("-beta"))
            .unwrap_or(name);
        match top {
            "chrome" => Self::Chrome,
            "edge" => Self::Edge,
            "firefox" => Self::Firefox,
            "safari" => Self::Safari,
            "iphone" => Self::Iphone,
            "ipad" => Self::Ipad,
            "ipad-air" => Self::IpadAir,
            "android" => Self::Android,
            "android-tablet" => Self::AndroidTablet,
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

    /// Whether safaridriver drives it, the Mac's Safari or a Simulator's.
    #[must_use]
    pub fn is_safari(self) -> bool {
        matches!(
            self,
            Self::Safari | Self::Iphone | Self::Ipad | Self::IpadAir
        )
    }

    /// Whether it is an iOS Simulator's Safari.
    #[must_use]
    pub fn is_simulator(self) -> bool {
        matches!(self, Self::Iphone | Self::Ipad | Self::IpadAir)
    }

    /// Whether it is Chrome on an Android emulator.
    #[must_use]
    pub fn is_android(self) -> bool {
        matches!(self, Self::Android | Self::AndroidTablet)
    }

    /// The screen a device has, which no window resizes; none for a
    /// desktop browser, whose page a test sizes ([`crate::Page::viewport`]).
    #[must_use]
    pub fn screen(self) -> Option<Screen> {
        match self {
            Self::Iphone | Self::Android => Some(Screen::Phone),
            Self::Ipad | Self::AndroidTablet => Some(Screen::Tablet),
            Self::IpadAir => Some(Screen::Middle),
            _ => None,
        }
    }

    /// The session's capabilities: headless where the browser has it, so a
    /// run takes no window from the desktop, unless `E2E_HEADED` asks to
    /// watch; Safari has no headless mode. A device's session is on the one
    /// `run.sh` started, `device` its id: the Simulator's UDID, the
    /// emulator's adb serial. `binary` the browser a low-bar or beta lane
    /// starts instead of the installed one (`E2E_BINARY`): Chrome's, Edge's
    /// or Firefox's path; for Safari its name, which safaridriver takes
    /// instead ("Safari Technology Preview", the beta lane's).
    pub(crate) fn capabilities(
        self,
        headed: bool,
        device: &str,
        binary: Option<&str>,
    ) -> Capabilities {
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
            Self::Safari => serde_json::json!({ "browserName": binary.unwrap_or("safari") }),
            Self::Iphone | Self::Ipad | Self::IpadAir => serde_json::json!({
                "browserName": "Safari",
                "platformName": "iOS",
                "safari:useSimulator": true,
                "safari:deviceUDID": device,
            }),
            Self::Android | Self::AndroidTablet => serde_json::json!({
                "browserName": "chrome",
                "goog:chromeOptions": {
                    "androidPackage": "com.android.chrome",
                    "androidDeviceSerial": device,
                },
            }),
        };
        match json {
            Value::Object(map) => Capabilities::from(map),
            _ => unreachable!("the capabilities are an object"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Browser, Screen, Value};

    #[test]
    fn a_low_or_beta_lane_is_its_top_lanes_browser() {
        assert_eq!(Browser::from_name("chrome-low"), Browser::Chrome);
        assert_eq!(Browser::from_name("firefox-beta"), Browser::Firefox);
        assert_eq!(Browser::from_name("ipad-air-low"), Browser::IpadAir);
        assert_eq!(Browser::from_name("android-tablet"), Browser::AndroidTablet);
        assert_eq!(Browser::from_name("safari-beta"), Browser::Safari);
    }

    #[test]
    fn a_low_or_beta_lane_starts_its_own_binary() {
        let caps = Value::from(Browser::Chrome.capabilities(false, "", Some("/old/chrome")));
        assert_eq!(caps["goog:chromeOptions"]["binary"], "/old/chrome");
        let caps = Value::from(Browser::Firefox.capabilities(true, "", None));
        assert_eq!(caps["webSocketUrl"], true, "BiDi on, to hear the page");
        assert_eq!(caps["moz:firefoxOptions"]["args"], serde_json::json!([]));
        let name = |binary| {
            Value::from(Browser::Safari.capabilities(false, "", binary))["browserName"].clone()
        };
        assert_eq!(
            name(Some("Safari Technology Preview")),
            "Safari Technology Preview"
        );
        assert_eq!(name(None), "safari");
    }

    #[test]
    fn a_device_lane_drives_the_device_it_started() {
        let caps = Value::from(Browser::Ipad.capabilities(false, "UDID-1", None));
        assert_eq!(caps["safari:deviceUDID"], "UDID-1");
        let caps = Value::from(Browser::Android.capabilities(false, "emulator-5560", None));
        assert_eq!(
            caps["goog:chromeOptions"]["androidDeviceSerial"],
            "emulator-5560"
        );
        assert_eq!(Browser::Android.screen(), Some(Screen::Phone));
        assert_eq!(Browser::IpadAir.screen(), Some(Screen::Middle));
        assert_eq!(Browser::Edge.screen(), None);
    }
}
