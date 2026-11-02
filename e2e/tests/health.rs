//! What every page must keep, whatever it shows (docs/testing.md, "Browser
//! tests"), each from an islands app on this stack: no sideways scroll at
//! five widths, a phone's tap targets 44px tall, nothing the accessibility
//! audit finds, a turned phone's camera side kept clear, the top clear of a
//! status bar over the page, Reduce Motion read and obeyed, the islands
//! waking on every load in one tab, no script error or CSP refusal, and the
//! skip link first on Tab. A new page goes into [`PAGES`]. What
//! needs DevTools' emulation (the insets, Reduce Motion asked for) runs in
//! Chrome and Edge and skips elsewhere, saying why.

use app_e2e::{Browser, Key, Page, RESET, WebDriverResult, browse};
use serde_json::{Value, json};

/// One page of each kind: the home page with its lazy island, a form, a
/// notice (the not-found page).
const PAGES: [&str; 3] = ["/", RESET, MISSING];

/// The not-found page, asked for on purpose.
const MISSING: &str = "/no-such-page";

/// The 404 [`MISSING`] asks for, which Chrome logs.
fn expect_the_miss(page: &Page) {
    page.allow_failed_load(404, MISSING);
}

/// What goes wrong on a page, recorded by the page from before its own
/// scripts run ([`Page::preload`]): a script's error, a refused load, a
/// rejected promise, a CSP refusal, a console error. The harness hears the
/// same in Chrome and Edge through DevTools; in Firefox only this hears the
/// CSP's refusals.
const RECORD_TROUBLE: &str = "window.__e2eTrouble = [];
  const note = said => window.__e2eTrouble.push(said);
  window.addEventListener('error', e => note('error: ' + (e.message || e.target?.src || e.target?.href || e.type)), true);
  window.addEventListener('unhandledrejection', e => note('rejection: ' + e.reason));
  document.addEventListener('securitypolicyviolation', e => note('csp: ' + e.violatedDirective + ' ' + e.blockedURI));
  const error = console.error.bind(console);
  console.error = (...said) => { note('console: ' + said.map(String).join(' ')); error(...said); };";

/// No page has a script error or a CSP refusal, recorded by the page
/// itself ([`RECORD_TROUBLE`]), the not-found page's own 404 aside. Where a
/// script runs before the page's: Chrome and Edge, Firefox through BiDi.
#[tokio::test]
async fn no_page_has_a_script_error_or_a_csp_refusal() {
    browse(|page| async move {
        if !(page.browser.has_devtools() || page.browser == Browser::Firefox) {
            page.skip("no script before the page's own in this driver");
            return Ok(());
        }
        expect_the_miss(&page);
        page.preload(RECORD_TROUBLE).await?;
        let mut found = Vec::new();
        for path in PAGES {
            page.open(path).await?;
            // The page's scripts a moment to run.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let said = page
                .eval("return window.__e2eTrouble ?? ['the recorder did not run']")
                .await?;
            for said in said.as_array().into_iter().flatten() {
                let said = said.as_str().unwrap_or_default();
                // The not-found page's own status, which the browser reports.
                if path == MISSING && said.contains("404") {
                    continue;
                }
                found.push(format!("{path}: {said}"));
            }
        }
        assert!(found.is_empty(), "trouble:\n{}", found.join("\n"));
        Ok(())
    })
    .await;
}

/// The skip link is the first thing Tab reaches, and Enter on it takes the
/// focus to `<main>`. A real keyboard's: Safari reaches links by Tab only
/// with a setting of its own, and the devices have none; skipped there.
#[tokio::test]
async fn the_skip_link_is_the_first_tab_and_leads_to_main() {
    browse(|page| async move {
        if page.browser.is_safari() || page.browser.screen().is_some() {
            page.skip("no keyboard that tabs to links");
            return Ok(());
        }
        page.open(RESET).await?;
        page.driver
            .action_chain()
            .send_keys(Key::Tab)
            .perform()
            .await?;
        page.until(
            "the skip link in focus",
            "return document.activeElement?.getAttribute('href') === '#content'",
        )
        .await?;
        page.driver
            .action_chain()
            .send_keys(Key::Enter)
            .perform()
            .await?;
        page.until(
            "the focus on main",
            "return document.activeElement?.tagName === 'MAIN'",
        )
        .await?;
        Ok(())
    })
    .await;
}

/// DevTools' safe-area insets set to `insets`, or the test skipped where
/// the browser has none to set: Firefox and Safari, and Chrome 114 (the low
/// bar), whose DevTools lack them. Whether it may go on.
async fn insets(page: &Page, insets: Value) -> WebDriverResult<bool> {
    if !page.browser.has_devtools() {
        page.skip("no DevTools: no emulated safe-area insets");
        return Ok(false);
    }
    if page
        .devtools("Emulation.setSafeAreaInsetsOverride", insets)
        .await
        .is_err()
    {
        page.skip("this DevTools can't set safe-area insets");
        return Ok(false);
    }
    Ok(true)
}

/// Chrome 114 to 123 stopped calling the page idle after 9 to 14 whole
/// loads in one tab, and Leptos's loader waited for that with no timeout
/// until the fork's (`Cargo.toml`): twenty loads, each one's islands awake
/// ([`Page::open`] waits for them). It proves the fork in the low lane.
#[tokio::test]
async fn the_islands_wake_on_every_load_in_one_tab() {
    browse(|page| async move {
        for _ in 0..10 {
            page.open("/").await?;
            page.open(RESET).await?;
        }
        Ok(())
    })
    .await;
}

/// Five widths in a desktop browser; a device's own screen once.
#[tokio::test]
async fn no_page_scrolls_sideways_at_five_widths() {
    browse(|page| async move {
        expect_the_miss(&page);
        let widths: &[u32] = if page.browser.screen().is_some() {
            &[0]
        } else {
            &[360, 600, 768, 1024, 1440]
        };
        let mut found = Vec::new();
        for &width in widths {
            page.viewport(width, 900).await?;
            for path in PAGES {
                page.open(path).await?;
                let seen = page
                    .eval(
                        "const width = document.documentElement.clientWidth;
                         const over = document.documentElement.scrollWidth > width;
                         const wide = over ? [...document.querySelectorAll('body *')]
                             .filter(e => e.getBoundingClientRect().right > width + 1)
                             .slice(0, 4).map(e => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '')
                                 + ' ' + String(e.className).slice(0, 50)) : [];
                         return { over, wide, width }",
                    )
                    .await?;
                if seen["over"] == Value::Bool(true) {
                    found.push(format!("{path} at {}: {}", seen["width"], seen["wide"]));
                }
            }
        }
        assert!(found.is_empty(), "sideways:\n{}", found.join("\n"));
        Ok(())
    })
    .await;
}

/// Every control on the page under 44px tall, as `height tag text`: links,
/// buttons and fields a finger taps. A link inside a sentence is left out
/// (WCAG's inline exception), and a field counts its label's box when the
/// label holds it.
async fn small_targets(page: &Page) -> WebDriverResult<Vec<String>> {
    let small = page
        .eval(
            "const small = [];
             for (const e of document.querySelectorAll('a[href], button, input:not([type=hidden]), select, textarea')) {
               if (!e.checkVisibility({ visibilityProperty: true, checkVisibilityCSS: true, opacityProperty: true, checkOpacity: true })) continue;
               if (e.tagName === 'A' && getComputedStyle(e).display === 'inline') continue;
               let r = e.getBoundingClientRect();
               if (r.width <= 1 || r.height <= 1) continue;
               if (['INPUT', 'SELECT', 'TEXTAREA'].includes(e.tagName)) r = (e.closest('label') ?? e).getBoundingClientRect();
               if (r.height < 43.5) small.push(Math.round(r.height) + 'px ' + e.tagName.toLowerCase()
                   + (e.id ? '#' + e.id : '') + ' ' + (e.textContent.trim() || e.getAttribute('aria-label') || e.name || '').slice(0, 30));
             }
             return small",
        )
        .await?;
    Ok(small
        .as_array()
        .map(|small| {
            small
                .iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default())
}

/// At a phone's width ([`Page::phone`]): skipped on a tablet.
#[tokio::test]
async fn a_phones_tap_targets_are_44px_tall() {
    browse(|page| async move {
        expect_the_miss(&page);
        if !page.phone().await? {
            return Ok(());
        }
        let mut found = Vec::new();
        for path in PAGES {
            page.open(path).await?;
            for small in small_targets(&page).await? {
                found.push(format!("{path}: {small}"));
            }
        }
        assert!(found.is_empty(), "under 44px:\n{}", found.join("\n"));
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn the_accessibility_audit_finds_nothing() {
    browse(|page| async move {
        expect_the_miss(&page);
        let mut found = Vec::new();
        for path in PAGES {
            page.open(path).await?;
            for broken in page.axe().await? {
                found.push(format!(
                    "{path}: {} ({}) {}: {}",
                    broken["id"].as_str().unwrap_or_default(),
                    broken["impact"].as_str().unwrap_or_default(),
                    broken["help"].as_str().unwrap_or_default(),
                    broken["nodes"]
                ));
            }
        }
        assert!(found.is_empty(), "the audit:\n{}", found.join("\n"));
        Ok(())
    })
    .await;
}

/// `<main>`'s side gaps and the side the helper marked
/// (public/js/safe-area.js).
const SIDES: &str = "const s = getComputedStyle(document.querySelector('main'));
     return { mark: document.documentElement.dataset.safeArea ?? '',
              left: Math.round(parseFloat(s.paddingLeft)),
              right: Math.round(parseFloat(s.paddingRight)) }";

/// A phone on its side keeps only its camera's side clear: 59px insets on
/// both sides, as iOS reports them, and the helper marks the camera's side
/// from the screen's angle, the other side getting the column's 24px back.
/// Turned from 90° to 270°, the sides swap with no reload. A page narrower
/// than the screen (Chrome on an iPhone keeps its own view clear) keeps no
/// inset at all, and upright nothing moves. Which angle is which side was
/// seen on an iPhone in four browsers (public/js/safe-area.js). DevTools'
/// own insets ([`insets`]).
#[tokio::test]
async fn a_phone_on_its_side_keeps_only_its_cameras_side_clear() {
    browse(|page| async move {
        if !insets(&page, json!({ "insets": { "left": 59, "right": 59 } })).await? {
            return Ok(());
        }
        let turned = |angle: u32, screen: u32| {
            let kind = if angle == 90 {
                "landscapePrimary"
            } else {
                "landscapeSecondary"
            };
            json!({
                "width": 874, "height": 402, "deviceScaleFactor": 0, "mobile": false,
                "screenWidth": screen, "screenHeight": 402,
                "screenOrientation": { "type": kind, "angle": angle },
            })
        };
        page.devtools("Emulation.setDeviceMetricsOverride", turned(90, 874))
            .await?;
        page.open("/").await?;
        let left = page.eval(SIDES).await?;
        assert_eq!(
            left,
            json!({ "mark": "left", "left": 59, "right": 24 }),
            "the camera on the left"
        );
        page.devtools("Emulation.setDeviceMetricsOverride", turned(270, 874))
            .await?;
        // The mark, then the insets' 500ms glide to its end.
        page.until(
            "the camera on the other side",
            "const html = document.documentElement;
             return html.dataset.safeArea === 'right'
                 && getComputedStyle(html).getPropertyValue('--safe-area-left') === '0px'",
        )
        .await?;
        assert_eq!(
            page.eval(SIDES).await?,
            json!({ "mark": "right", "left": 24, "right": 59 }),
            "the camera on the right"
        );
        page.devtools("Emulation.setDeviceMetricsOverride", turned(90, 1000))
            .await?;
        page.open("/").await?;
        assert_eq!(
            page.eval(SIDES).await?,
            json!({ "mark": "none", "left": 24, "right": 24 }),
            "a view the browser keeps clear itself"
        );
        page.devtools(
            "Emulation.setSafeAreaInsetsOverride",
            json!({ "insets": {} }),
        )
        .await?;
        page.viewport(390, 844).await?;
        page.open("/").await?;
        assert_eq!(
            page.eval(SIDES).await?,
            json!({ "mark": "", "left": 24, "right": 24 }),
            "upright, nothing moves"
        );
        Ok(())
    })
    .await;
}

/// The header's top gap and the focused skip link's top.
const TOP: &str = "const link = document.querySelector('a[href=\"#content\"]');
     link.focus();
     const top = { header: Math.round(parseFloat(getComputedStyle(document.querySelector('header')).paddingTop)),
                   skip: Math.round(parseFloat(getComputedStyle(link).top)) };
     link.blur();
     return top";

/// The page's top clear of a status bar lying over it, as in the app's
/// webview on a phone (src-tauri/): a 59px top inset adds itself to the
/// header's 24px and the skip link's 16px, and with the bar gone the usual
/// gaps glide back. DevTools' own insets ([`insets`]).
#[tokio::test]
async fn the_page_keeps_clear_of_a_status_bar_over_it() {
    browse(|page| async move {
        if !insets(&page, json!({ "insets": { "top": 59 } })).await? {
            return Ok(());
        }
        page.viewport(393, 852).await?;
        page.open("/").await?;
        assert_eq!(
            page.eval(TOP).await?,
            json!({ "header": 83, "skip": 75 }),
            "under a status bar"
        );
        page.devtools(
            "Emulation.setSafeAreaInsetsOverride",
            json!({ "insets": {} }),
        )
        .await?;
        // The inset's 500ms glide to its end.
        page.until(
            "the bar gone",
            "return getComputedStyle(document.documentElement).getPropertyValue('--safe-area-top') === '0px'",
        )
        .await?;
        assert_eq!(
            page.eval(TOP).await?,
            json!({ "header": 24, "skip": 16 }),
            "no bar"
        );
        Ok(())
    })
    .await;
}

/// The Reduce Motion rule (style/tailwind.css) is in the stylesheet as the
/// browser read it, not only in the source: the build's CSS minifier can
/// split or drop a rule, and a browser can refuse one it can't parse (found
/// in an islands app, 2026-10-21). Read in every browser.
#[tokio::test]
async fn the_reduce_motion_rule_is_read() {
    browse(|page| async move {
        page.open(RESET).await?;
        let read = page
            .eval(
                "const rules = s => { try { return [...s.cssRules]; } catch (e) { return []; } };
                 const deep = list => list.flatMap(r => [r, ...(r.cssRules ? deep([...r.cssRules]) : [])]);
                 return deep([...document.styleSheets].flatMap(rules)).some(r => r.media
                     && r.media.mediaText.includes('prefers-reduced-motion: reduce')
                     && [...r.cssRules].some(inner => (inner.selectorText || '').includes('::before')
                         && inner.style.transitionProperty === 'none'));",
            )
            .await?;
        assert_eq!(read, Value::Bool(true), "the Reduce Motion rule dropped");
        Ok(())
    })
    .await;
}

/// With Reduce Motion asked for, the reset form's button, which fades into
/// its busy look, has no transition. Asked for through DevTools' emulation:
/// Chrome and Edge.
#[tokio::test]
async fn reduce_motion_stops_every_transition() {
    browse(|page| async move {
        if !page.browser.has_devtools() {
            page.skip("no DevTools: no emulated Reduce Motion");
            return Ok(());
        }
        page.open(RESET).await?;
        let button = "return getComputedStyle(document.querySelector('main form [type=submit]')).transitionProperty";
        assert_ne!(
            page.eval(button).await?,
            Value::from("none"),
            "the button fades at first"
        );
        page.emulate("prefers-reduced-motion", "reduce").await?;
        page.open(RESET).await?;
        assert_eq!(page.eval(button).await?, Value::from("none"), "and then not");
        Ok(())
    })
    .await;
}
