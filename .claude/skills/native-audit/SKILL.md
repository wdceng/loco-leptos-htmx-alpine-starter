---
name: native-audit
description: Use when the maintainer asks for a native audit of a project's phone apps (a web app inside a native shell, Tauri with Swift and Kotlin), "is the app native enough", "will Apple take it (4.2)", "what should be native and what web", or before a store submission. A whole-app pass against the web-first ladder (web, a bridge, a native screen), Apple's guideline 4.2, Google Play's and Android's core app quality, and the project's own list (.claude/native.md), findings by severity, no edit until the maintainer's go. A change's own app code is /review's.
---

# Native audit

A whole-app pass over a project's phone apps from three sides: the reviewer's (Apple's 4.2, Google Play's policies), the user's (does it feel like the phone's own app), and the maintainer's (one way per job, the cost of two native twins). Read-only.

With ultracode on, a workflow, one agent a pass or a platform, each finding read a second time; with it off, inline, one pass, one subagent at most (the maintainer's rules: `CLAUDE.local.md`, `~/.claude/CLAUDE.md`). An agent runs no cargo command.

## The project's list first

Read `.claude/native.md` in the project's root: its native parts and their files, the bridge (what the page hands the app, what the app announces back), its app lanes, the dated calls about what is web and what native, and what was tried and dropped. No list: say so and stop; a new project copies the starter's.

## The ladder

Every part of the app sits on one rung, the lowest that does its job (Hotwire Native's order, native.hotwired.dev: "Content is all web. Navigation is all native."):

1. **Web**: the page, drawn by the server, the same for the app, every browser and the tablets. Content, forms, words, every state of a fact, the logic, what a tap does.
2. **A bridge**: one native control standing for the page's own, the page deciding. The page hands its state (JSON, at a change alone), the app draws it, a tap goes back by its id and the page does what its own control does. The page's control hidden under the native one, never both seen (Rule no. 1). Navigation chrome (the tab bar, the header's controls), a system control that looks different (a switch, the share sheet), a system API (push, haptics, passkeys, widgets, quick actions).
3. **A native screen**: a whole screen in Swift and Kotlin, only where neither does the job (an SDK's own screen, a fidelity the web cannot reach), and then with an address of its own, so navigation stays one. Hotwire's cost, said in the plan: written once per platform, each change through store review.

A rung up is justified in the plan by one of: a reviewer would see it (4.2); the platform's convention cannot be matched by the web; it must show before the page is ready (the first screen, the bar at launch); and its cost named.

## Passes

1. **Rungs**: each native part's rung against the ladder. Too low: web where 4.2 or the platform wants native (navigation, back, the system's gestures and sheets). Too high: logic or words in Swift or Kotlin that the page should own (a second implementation).
2. **The bridge**: the page decides and the app draws; no page logic or script in the app; one place in the page calls the app; every command's input read before the app keeps or runs it; the app's capabilities only the commands the site needs; a native part with no page fallback where the app is old or the platform has none.
3. **Twins**: each iPhone part has its Android twin, or a written reason it has none; each looks and moves as its own platform's (Apple's HIG, Material 3), not as the other's.
4. **Apple's 4.2**: "features, content, and UI that elevate it beyond a repackaged website" (App Review Guidelines, 4.2, opened before quoted); not "web clippings … or a collection of links" (4.2.2, catalogs allowed); WebKit for web content (2.5.6). The usual rejection's words, from developers' reports: "not sufficiently different from a mobile browsing experience". What a reviewer meets in the first minute: a native launch screen, native navigation, safe areas, the system's back and gestures, nothing that looks like Safari.
5. **Android's core app quality** (developer.android.com/docs/quality-guidelines/core-app-quality): the system's back, no back button of the app's own on screen, predictive back; frames within 16 ms; feedback on screen past 2 s; no crash or ANR; WebView: no `addJavaScriptInterface()` with untrusted content, `WebViewAssetLoader` for local files. Google Play: no limited functionality, no broken functionality; a webview of one's own site is allowed (the spam policy bans another's site without permission).
6. **Feel**: a press state on every control; sheets and panels slide, never snap; the system's haptics, not a vibrator's pattern; Reduce Motion honoured; text as an app's (no selection, no link previews) where the project decided so.
7. **Tests**: each native part on the app lanes; what a lane cannot drive named as a hand check in the project's testing doc.
8. **The review kit** (before a submission): a demo account that passes every sign-in step, review notes walking the reviewer through each native part, screenshots from the device, a written list of how the app differs from the website, ready for a Resolution Center reply or an appeal.

## Sources

Opened before quoted, each time: Apple's App Review Guidelines and Human Interface Guidelines (the HIG renders by script: read it in a browser or the maintainer's paste), Android's core app quality and Material 3, Google Play's policy pages, the framework's own docs (Tauri, its plugins). Developers' reports are reports, said so.

## Severity

| | Meaning |
|---|---|
| High | a likely rejection (4.2, a Play policy) or a broken platform rule (back, a crash) |
| Medium | a part that feels like a website to a phone's user, or a second implementation of a job |
| Low | polish: a press state, a motion, a twin's small difference |
| Info | worth knowing |

A High is checked a second time by trying to refute it.

## Report

A table, most severe first: number, severity, the pass, the platform, a clickable file link with its line, what is wrong, the rung or rule it breaks (with its source), the fix proposed, its rung. Then the clean line, then one question: "go all", "go 1, 3" or "skip". Nothing is edited before the go; the fixes go to the working session as sub-tasks, each with its own app-lane test.

## The list's shape

`.claude/native.md`, in the project's git, short: `## Native parts` (each with its rung, its files on both platforms, and its reason), `## The bridge` (what the page hands the app, what comes back, the one place that calls it), `## App lanes` (the lanes and what each drives), `## Calls` (dated: what stays web, what went native, and why), `## Tried and dropped`. Task ids stay out of it where the project's guard keeps them in its plan alone.
