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

#[cfg(desktop)]
use tauri::WebviewUrl;
#[cfg(desktop)]
use tauri::webview::NewWindowResponse;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Url, WebviewWindow, WebviewWindowBuilder, utils::config::WindowConfig};
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
            watched(app.handle(), &window, site)?.build()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = started {
        eprintln!("the app could not start: {error}");
    }
}

/// The window `config` describes, its navigation watched: an address the
/// page follows goes where [`destination`] says, and on the desktops a link
/// to a new tab too ([`new_window`]). Every window of the app is built
/// here, the first and the ones a link opens.
fn watched<'a>(
    handle: &'a AppHandle,
    config: &WindowConfig,
    site: Option<Url>,
) -> tauri::Result<WebviewWindowBuilder<'a, tauri::Wry, AppHandle>> {
    let builder = WebviewWindowBuilder::from_config(handle, config)?;
    #[cfg(desktop)]
    let builder = {
        let (handle, config, site) = (handle.clone(), config.clone(), site.clone());
        builder
            .on_new_window(move |url, _features| new_window(&handle, &config, site.as_ref(), &url))
    };
    let handle = handle.clone();
    Ok(builder
        .on_navigation(move |url| match destination(url, site.as_ref()) {
            Destination::App => true,
            Destination::Nowhere => {
                eprintln!("{url} not opened: neither the app's nor one the system opens");
                false
            }
            Destination::System => {
                to_system(&handle, url);
                false
            }
        })
        .on_page_load(|window, payload| {
            if payload.event() == PageLoadEvent::Finished {
                paint_status_bar(&window);
            }
        }))
}

/// `url` handed to the system: the browser, or the app its scheme is for.
/// Off the thread the hooks run on: Android calls them on its UI thread,
/// where the opener's own Kotlin then waits to run, and the two waited for
/// each other, the app frozen (an islands app on this stack, 2026-10-29).
fn to_system(handle: &AppHandle, url: &Url) {
    let (handle, address) = (handle.clone(), url.to_string());
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = handle.opener().open_url(&address, None::<&str>) {
            eprintln!("{address} could not be handed to the system: {error}");
        }
    });
}

/// A link to a new tab, on the desktops (Tauri's hook has none on the
/// phones, where an outside one reaches `on_navigation` first): another
/// site's to the system, as any other address; the site's own in a second
/// window of the app's, its new tab, so the page it was followed from (a
/// form half filled) stays where it was. The site has no such link of its
/// own (`no_page_opens_a_new_tab`, tests/one_job.rs, a step beside a form
/// being a dialog); the hook stays for one that comes. That window is the
/// first's, from its config, its hooks the same ([`watched`]); the webview's own new
/// window refused either way. Windows' webview did nothing with such a
/// link without this hook, and Apple's opened no window for the site's own
/// (an islands app on this stack, 2026-11-03). Untested here on Windows and
/// on a Linux desktop.
#[cfg(desktop)]
fn new_window(
    handle: &AppHandle,
    config: &WindowConfig,
    site: Option<&Url>,
    url: &Url,
) -> NewWindowResponse<tauri::Wry> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static OPENED: AtomicUsize = AtomicUsize::new(0);
    match destination(url, site) {
        Destination::System => to_system(handle, url),
        Destination::Nowhere => {
            eprintln!("{url} not opened: neither the app's nor one the system opens");
        }
        Destination::App => {
            let mut tab = config.clone();
            tab.label = format!("tab-{}", OPENED.fetch_add(1, Ordering::Relaxed) + 1);
            tab.url = WebviewUrl::External(url.clone());
            if let Err(error) = watched(handle, &tab, site.cloned()).and_then(|tab| tab.build()) {
                eprintln!("{url} could not open in a window of its own: {error}");
            }
        }
    }
    NewWindowResponse::Deny
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

    /// On the desktops a link to a new tab goes through `new_window`,
    /// which `watched` wires to Tauri's hook for every window it builds,
    /// and the webview's own new window is refused whatever the link, the
    /// site's own opened in a window of the app's with the same hooks
    /// (an islands app on this stack, 2026-11-03: Windows' webview had
    /// done nothing with such a link). Read from this file's code above its
    /// tests.
    #[test]
    fn the_desktops_links_to_a_new_tab_go_through_new_window() {
        let code = include_str!("lib.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap_or_default();
        let body = |definition: &str| {
            code.split_once(definition)
                .and_then(|(_, rest)| rest.split_once("\n}\n"))
                .map(|(body, _)| body)
                .unwrap_or_default()
        };
        let watched = body("fn watched<");
        let gate = watched.find("#[cfg(desktop)]");
        let hook = watched.find(".on_new_window(");
        assert!(
            gate.zip(hook).is_some_and(|(gate, hook)| gate < hook)
                && watched.contains("new_window(&handle"),
            "the desktops' new-tab hook not wired to new_window in watched"
        );
        assert!(
            body("fn new_window(")
                .trim_end()
                .ends_with("NewWindowResponse::Deny"),
            "new_window lets the webview's own window through"
        );
    }

    /// A long press on a link shows no preview card in the app: the
    /// iPhone's and the Mac's webview offer one by default
    /// (`allowLinkPreview`, WKWebView's `allowsLinkPreview`), a browser's
    /// way that told the app was a website (an islands app on this stack,
    /// 2026-11-07).
    #[test]
    fn a_long_press_on_a_link_shows_no_preview() {
        let config = include_str!("../tauri.conf.json");
        let window = config
            .split_once("\"windows\"")
            .and_then(|(_, rest)| rest.split_once(']'))
            .map(|(window, _)| window)
            .expect("the window's config");
        assert!(
            window.contains("\"allowLinkPreview\": false"),
            "the window's link preview off:\n{window}"
        );
    }

    /// The Android app's activity runs in a task of its own, so another
    /// app's cannot take its place there (task hijacking, OWASP MASTG); a
    /// manifest Tauri writes again would lose it.
    #[test]
    fn the_android_activity_keeps_a_task_of_its_own() {
        let manifest = include_str!("../gen/android/app/src/main/AndroidManifest.xml");
        assert!(
            manifest.contains("android:taskAffinity=\"\""),
            "MainActivity's taskAffinity empty"
        );
    }

    /// The phones' release library is stripped by Cargo's profile, before
    /// Gradle sees it: Tauri's Android template strips nothing itself, its
    /// `keepDebugSymbols` reaching every build type (tauri#15884) and
    /// Gradle given no NDK (tauri#16173), so without this line the symbols
    /// would ship in the store's bundle.
    #[test]
    fn the_phones_release_library_is_stripped_by_cargo() {
        let manifest = include_str!("../Cargo.toml");
        let release = manifest
            .split_once("[profile.release]")
            .map(|(_, rest)| rest.split("\n[").next().unwrap_or_default())
            .expect("a release profile");
        assert!(
            release.lines().any(|line| line.trim() == "strip = true"),
            "strip = true in [profile.release]:\n{release}"
        );
    }

    /// The iPhone's app declares no encryption of its own, so App Store
    /// Connect asks nothing at an upload: the key in `Info.ios.plist`, and
    /// in the `Info.plist` an iOS build merges it into, committed as the
    /// build writes it.
    #[test]
    fn the_iphones_app_declares_no_encryption_of_its_own() {
        let key = "<key>ITSAppUsesNonExemptEncryption</key>";
        for (file, plist) in [
            ("Info.ios.plist", include_str!("../Info.ios.plist")),
            (
                "gen/apple/app-native_iOS/Info.plist (an iOS build merges it)",
                include_str!("../gen/apple/app-native_iOS/Info.plist"),
            ),
        ] {
            let after = plist
                .split_once(key)
                .map(|(_, after)| after.trim_start())
                .unwrap_or_default();
            assert!(after.starts_with("<false/>"), "{file}: {key} false");
        }
    }

    /// The iPhone's app tells App Store Connect why it uses the APIs Apple
    /// asks a reason for: each category with its one reason in the app's
    /// privacy manifest, and the manifest among the Xcode target's
    /// resources, so the upload is not refused.
    #[test]
    fn the_iphones_app_declares_the_apis_apple_asks_reasons_for() {
        let manifest = include_str!("../gen/apple/app-native_iOS/PrivacyInfo.xcprivacy");
        for (category, reason) in [
            ("FileTimestamp", "C617.1"),
            ("SystemBootTime", "35F9.1"),
            ("ActiveKeyboards", "54BD.1"),
        ] {
            let category = format!("<string>NSPrivacyAccessedAPICategory{category}</string>");
            let reasons = manifest
                .split_once(&category)
                .and_then(|(_, after)| after.split_once("</array>"))
                .map(|(reasons, _)| reasons)
                .unwrap_or_default();
            assert_eq!(manifest.matches(&category).count(), 1, "{category}");
            assert_eq!(
                reasons.matches("<string>").count(),
                1,
                "{category}: one reason"
            );
            assert!(
                reasons.contains(&format!("<string>{reason}</string>")),
                "{category}: {reason}"
            );
        }
        let xcode = include_str!("../gen/apple/app-native.xcodeproj/project.pbxproj");
        assert!(
            xcode.contains("/* PrivacyInfo.xcprivacy in Resources */"),
            "the app's manifest a resource"
        );
    }

    /// Every iPhone icon is a PNG without an alpha channel: App Store
    /// Connect refuses an App Store icon that has one, and Tauri's icon tool
    /// writes them with one (`opaque.swift`, docs/development.md, Icons).
    #[test]
    fn the_iphones_icons_have_no_alpha_channel() {
        let icons = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("gen/apple/Assets.xcassets/AppIcon.appiconset");
        let mut seen = 0;
        for entry in std::fs::read_dir(&icons)
            .expect("the iPhone's icons")
            .flatten()
        {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "png") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("an icon");
            // The PNG header's colour type, IHDR's tenth byte: 2 is red,
            // green and blue alone, 6 the same with alpha.
            assert_eq!(
                bytes.get(25),
                Some(&2),
                "{} has no alpha channel",
                path.display()
            );
            seen += 1;
        }
        assert!(seen > 0, "the iPhone's icons");
    }
}
