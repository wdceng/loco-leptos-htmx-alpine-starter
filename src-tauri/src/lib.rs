//! The app: the window tauri.conf.json describes, on every platform. It
//! opens the start page built into the app (`offline/index.html`), which
//! hands over to the site as soon as its `/up` answers `ok`
//! (src/controllers/up.rs), and until then says why not: no internet, or
//! the site not reachable. The page then is the live site, so every deploy
//! shows at once and the app itself rarely changes. The site gets no access
//! to the device: no Tauri command is exposed to it (no capabilities file).
//! An address the page follows that is not the site's goes to the system
//! instead of the webview (`destination`): a call, a mail, a text or
//! another app's link to that app, which the webview cannot open and shows
//! its error page with no way back (Android, in an islands app on this
//! stack, 2026-10-29), and another site to the browser, from which there
//! was no way back either. Any other scheme (`file:`, `intent:`) goes
//! nowhere: the system opens only the ones named in `SYSTEM_SCHEMES`. On
//! Android the status bar takes the page's colour as each page loads
//! (`paint_status_bar`).

use tauri::webview::PageLoadEvent;
use tauri::{Url, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

/// Starts the app. The phones call it through `mobile_entry_point`, which
/// takes a function that returns nothing, so a failure to start is written
/// to the log rather than returned. The window is the config's
/// (`"create": false` there), built here so its navigation can be watched.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = tauri::Builder::default()
        // Without the plugin's own script for links to a new tab: it caught
        // their click and asked the app over IPC, which the site may not, so
        // such a link did nothing (an islands app on this stack, 2026-10-29).
        // It now navigates like any other and `on_navigation` hands it over.
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .setup(|app| {
            let Some(window) = app.config().app.windows.first().cloned() else {
                return Err("tauri.conf.json describes no window".into());
            };
            // The site the start page opens, the one address its CSP lets
            // it ask (`connect-src`): the same in a build `screens.sh`
            // points elsewhere.
            let site = app
                .config()
                .app
                .security
                .csp
                .as_ref()
                .and_then(|csp| site_of(&csp.to_string()));
            let handle = app.handle().clone();
            WebviewWindowBuilder::from_config(app.handle(), &window)?
                .on_navigation(move |url| match destination(url, site.as_ref()) {
                    Destination::App => true,
                    Destination::Nowhere => {
                        eprintln!("{url} not opened: neither the app's nor one the system opens");
                        false
                    }
                    Destination::System => {
                        // Off the thread this hook runs on: Android calls it
                        // on its UI thread, where the opener's own Kotlin
                        // then waits to run, and the two waited for each
                        // other, the app frozen (an islands app on this
                        // stack, 2026-10-29).
                        let (handle, address) = (handle.clone(), url.to_string());
                        tauri::async_runtime::spawn_blocking(move || {
                            if let Err(error) = handle.opener().open_url(&address, None::<&str>) {
                                eprintln!("{address} could not be handed to the system: {error}");
                            }
                        });
                        false
                    }
                })
                .on_page_load(|window, payload| {
                    if payload.event() == PageLoadEvent::Finished {
                        paint_status_bar(&window);
                    }
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = started {
        eprintln!("the app could not start: {error}");
    }
}

/// Android's status bar takes the page's own `theme-color`, the gradient
/// under the clock (`MainActivity.paintStatusBar`, in `gen/android`). Asked
/// of the activity through wry's queue to the webview's thread: nothing
/// here waits on Android's UI thread, which this hook runs on (the opener's
/// lesson above).
#[cfg(target_os = "android")]
fn paint_status_bar(window: &WebviewWindow) {
    let asked = window.with_webview(|webview| {
        webview.jni_handle().exec(|env, activity, _webview| {
            if env
                .call_method(activity, "paintStatusBar", "()V", &[])
                .is_err()
            {
                // A Java exception left pending would fail the next JNI call.
                let _ = env.exception_clear();
                eprintln!("the status bar's colour could not be asked for");
            }
        });
    });
    if let Err(error) = asked {
        eprintln!("the status bar's colour could not be asked for: {error}");
    }
}

/// Elsewhere the system draws behind its bars itself: iOS blurs the page
/// under the status bar, the desktops have none over the page.
#[cfg(not(target_os = "android"))]
fn paint_status_bar(_window: &WebviewWindow) {}

/// Cloudflare's challenge, which a site behind Cloudflare may show before
/// its pages: its box is a frame of this host's, and Apple's webviews ask
/// this hook about every frame, so it stays in the app or the challenge
/// could never pass. The site's own pages hold no frame (their CSP's
/// `default-src 'none'`).
const CHALLENGE_HOST: &str = "challenges.cloudflare.com";

/// The site's origin in the app's CSP: its `connect-src`'s address.
fn site_of(csp: &str) -> Option<Url> {
    csp.split(';')
        .find_map(|directive| directive.trim().strip_prefix("connect-src "))
        .and_then(|sources| sources.split_whitespace().next())
        .and_then(|source| Url::parse(source).ok())
}

/// The schemes the system opens for the app: a call, a mail, a text, and
/// the messengers whose own links the site may hold. A link to another app
/// (`whatsapp:`, `tg:`) needs its scheme here; any other goes nowhere.
const SYSTEM_SCHEMES: [&str; 5] = ["tel", "mailto", "sms", "viber", "fb-messenger"];

/// Where an address the page follows goes.
#[derive(Debug, PartialEq, Eq)]
enum Destination {
    /// The webview: the site, the start page, a frame's own.
    App,
    /// The system, which opens it in its own app, the app staying where it
    /// was: a call, a mail, a text, a messenger, another site in the
    /// browser.
    System,
    /// Neither: a scheme no link on the site has a reason to use (`file:`,
    /// `intent:`, `smb:`), which only an attacker's page would follow.
    Nowhere,
}

/// Where an address the page follows goes: a scheme in [`SYSTEM_SCHEMES`]
/// to the system, which only that app can open; a web address on another
/// host than `site`'s to the browser. The start page (`tauri:`, Android's
/// `tauri.localhost`), a frame's `about:`, `data:` and `blob:` and
/// Cloudflare's challenge stay; with no site known, every web address
/// stays. Any other scheme goes nowhere.
fn destination(url: &Url, site: Option<&Url>) -> Destination {
    match url.scheme() {
        "http" | "https" => {
            let host = url.host_str();
            let ours = host == Some("tauri.localhost")
                || host == Some(CHALLENGE_HOST)
                || site.is_none_or(|site| site.origin() == url.origin());
            if ours {
                Destination::App
            } else {
                Destination::System
            }
        }
        "tauri" | "asset" | "ipc" | "about" | "data" | "blob" => Destination::App,
        scheme if SYSTEM_SCHEMES.contains(&scheme) => Destination::System,
        _ => Destination::Nowhere,
    }
}

#[cfg(test)]
mod tests {
    use super::{Destination, destination, site_of};
    use tauri::Url;

    fn url(address: &str) -> Url {
        Url::parse(address).expect("an address")
    }

    #[test]
    fn the_site_is_the_csps_connect_src() {
        let csp = "default-src 'none'; connect-src https://example.com; script-src 'self'";
        assert_eq!(site_of(csp), Some(url("https://example.com")));
        assert_eq!(site_of("default-src 'none'"), None);
    }

    #[test]
    fn another_app_or_another_site_goes_to_the_system_and_the_site_stays() {
        let site = url("https://example.com");
        for address in [
            "tel:+15550100",
            "mailto:hello@example.com",
            "sms:?&body=Hello",
            "viber://forward?text=Hello",
            "fb-messenger://share?link=https%3A%2F%2Fexample.com",
            "https://wa.me/?text=Hello",
            "https://www.facebook.com/sharer/sharer.php?u=x",
            "https://maps.app.goo.gl/abc",
            "http://example.com/",
        ] {
            assert_eq!(
                destination(&url(address), Some(&site)),
                Destination::System,
                "{address} to the system"
            );
        }
        for address in [
            "https://example.com/reset",
            "tauri://localhost/index.html",
            "http://tauri.localhost/index.html",
            "https://challenges.cloudflare.com/cdn-cgi/challenge-platform/x",
            "about:blank",
        ] {
            assert_eq!(
                destination(&url(address), Some(&site)),
                Destination::App,
                "{address} in the app"
            );
        }
        assert_eq!(
            destination(&url("https://wa.me/"), None),
            Destination::App,
            "no site known, the web stays"
        );
    }

    /// The system opens only the schemes the app names: a local file, an
    /// Android intent or a network share from an injected link opens
    /// nowhere (docs/security.md, the review of 2026-10-31).
    #[test]
    fn a_scheme_the_app_doesnt_name_goes_nowhere() {
        let site = url("https://example.com");
        for address in [
            "file:///etc/passwd",
            "intent://scan/#Intent;scheme=zxing;end",
            "smb://host/share",
            "javascript:alert(1)",
        ] {
            assert_eq!(
                destination(&url(address), Some(&site)),
                Destination::Nowhere,
                "{address} nowhere"
            );
        }
    }

    /// The opener's own script for links to a new tab stays off (`run`):
    /// it caught their click and asked the app over IPC, which the site may
    /// not, and such a link did nothing (an islands app on this stack,
    /// 2026-10-29). The plugin's `init` turns it on. Read from this file's
    /// code above its tests.
    #[test]
    fn the_opener_leaves_links_to_a_new_tab_to_on_navigation() {
        let code = include_str!("lib.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap_or_default();
        assert!(
            !code.contains("tauri_plugin_opener::init()"),
            "the opener set up by init(), its link script on"
        );
        assert!(
            code.contains(".open_js_links_on_click(false)"),
            "the opener's link script not turned off"
        );
    }
}
