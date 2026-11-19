---
name: ux-audit
description: Use when the maintainer asks for a UX audit, a UI audit, a UI/UX review of the whole site or the apps, a heuristic evaluation, or "how does it feel to use" (not a change's review, which is /review). An expert inspection of a project's journeys, the public side's, the staff side's and the apps', against Nielsen's heuristics, ecommerce guidelines, Apple's and Material's, the project's own design record (its list, .claude/ux.md) and WCAG 2.2 AA, findings by NN/g's severity, no edit until the maintainer's go. Not usability testing: no real customer or staff member is watched.
---

# UX audit

An expert inspection of the site and the apps from the user's side: the users the project's list names (a shopper on a phone, one on a desktop, a staff member at work, a customer with an account, someone with a screen reader). Read-only. Not usability testing: it finds what breaks the guidelines, never why a real person gets stuck ("heuristic evaluations cannot replace user research", NN/g); that is a few real users watched, named in the report when a finding needs it.

A change's own UI is `/review`'s (passes 1, 6, 7, 8, 17); this is the whole site at once. The backend has `/security-audit` and `/review`; nothing here repeats them. A native part's rung (web, a bridge, a native screen) and Apple's 4.2 are `/native-audit`'s; this checks how it feels.

With ultracode on, NN/g's way, a workflow: three to five evaluators each on their own, then merged, each evaluator a different user from the list, none seeing another's findings before the merge. With it off, inline, one pass, one subagent at most (the maintainer's rules: `CLAUDE.local.md`, `~/.claude/CLAUDE.md`). An agent runs no cargo command.

## The project's list first

Read `.claude/ux.md` in the project's root: its users, its journeys, its sizes and platforms, its design record and its dropped calls, its rules (a language's, a trade's), how its screens are captured. No list: say so, run anyway, and propose one at the end (a new project copies the starter's).

## Scope

- The whole site, a journey, or the screens the maintainer names; ask when unclear.
- Each screen in its sizes: a phone upright and on its side, a tablet, a desktop; light and dark; each app where the journey runs there.
- Under the phone's settings a person changes: Android's three buttons and its swipes, touch feedback off, large text, Reduce Motion.

## Journeys

The list's. For a shop, Baymard's six (home and navigation; finding a product; a product's page; the basket; the checkout and what follows it; mobile, desktop and mobile judged apart), then the project's own (the account, the staff side, the app around the page).

## What each is checked against

- Nielsen's ten heuristics (nngroup.com/articles/ten-usability-heuristics).
- Ecommerce guidelines: Baymard's public checklists and articles (the full set is paid), each opened before it is quoted.
- The apps: Apple's Human Interface Guidelines and Material 3, opened before quoted.
- The project's own record (the list): one job looking and moving one way everywhere, the public side and the staff side, the web and the apps.
- WCAG 2.2 AA in use: the order a screen reader reads, 200% zoom and large text, the focus after a swap, contrast measured on the screens.
- The project's own rules (the list).

Not reported: what the project records as tried and dropped, or a dated call of the maintainer's, unless the finding brings a new reason, named.

## Passes, one a journey

1. **Consistency**: the same control, word, icon and motion for the same job on every screen.
2. **Platform**: where the main action, ✕, back, the tab bar and the search stand, and the gestures (swipe back, pull to refresh, Android's predictive back, the long press and what selects), against the platform's own apps.
3. **Layout**: each size; the safe areas; nothing clipped, overlapping or off the screen; 44pt tap targets.
4. **States**: empty, busy, refused, error, offline, no results, a long name, a large count, a slow network.
5. **Motion**: durations and curves as the design record says, nothing jumping, Reduce Motion honoured.
6. **Words**: the language a visitor reads, one term a thing, a button a verb, an error saying how to fix it, the list's rules.
7. **Accessibility in use**: as above.
8. **Speed**: Core Web Vitals on a slow phone (LCP under 2.5 s, INP under 200 ms, CLS under 0.1), the bundle's budget where the project has one.
9. **Flows**: the taps and screens each journey's main task takes, and where one could go.
10. **Feedback**: a press on every control while the finger is down, the tick under it, what a tap shows before the answer, a busy state where an answer takes past about a second.

## Evidence

- The code and the design record first: what the server draws in each state, read from the views.
- Screens, three sources:
  - The Simulators and emulators first: every size, light and dark, the web and the apps, unattended (the list's lanes). On the maintainer's word, on the test machine, by hash.
  - The maintainer's iPhone last, for what a Simulator does not show (Liquid Glass, fonts, the Dynamic Island, real scrolling, push, the share sheet, haptics): the maintainer opens the screen, says "shot", and it is read over the cable: `xcrun devicectl device capture screenshot --device <name> --destination /tmp/ux/<n>.png`. Only the screens the audit flags, or where glass and motion matter.
  - The maintainer's Android phone the same way, for what an emulator does not show: `adb -s <serial> exec-out screencap -p > /tmp/ux/<n>.png`.
- What no screen shows (a press, a tick, a gesture's feel): the project's hand checks read, and the flagged ones tried by the maintainer on the phones.
- The maintainer's own screenshots, whenever sent.
- An automated accessibility audit already in the browser tests (axe-core) is read, not rerun, unless the maintainer says.
- Every finding is checked in the code or on a screen before it is reported; a doubt is reported as a doubt.

## Severity

NN/g's scale, by how often it happens, how much it hurts, and whether people get past it:

| | Meaning |
|---|---|
| 4 | catastrophe: fix before go-live |
| 3 | major: high priority |
| 2 | minor: low priority |
| 1 | cosmetic: when there is time |

One evaluator's rating is "too unreliable to be trusted" (NN/g): inline, a 3 or 4 is checked a second time by the subagent trying to refute it, and marked confirmed or plausible.

## Report

A table, most severe first: number, severity, the journey and the screen (size, scheme, web or app), a clickable file link with its line, what is wrong, the guideline it breaks (opened, linked), the fix proposed. Then the clean line: the journeys and passes checked and found clean, and what was not checked (a size, an app). Then one question: "go all", "go 1, 3" or "skip". Before a fix that changes what a person sees, the choices with the sources' best, then a plan; nothing is edited before the go, and the fixes go to the working session as sub-tasks.

## The list's shape

`.claude/ux.md`, in the project's git, short: `## Users`, `## Journeys`, `## Sizes and platforms`, `## The record` (the design docs, the job register, what was dropped), `## Rules` (a language's, a trade's), `## Screens` (the lanes, the phones, the hand checks, the automated audit). Task ids stay out of it where the project's guard keeps them in its plan alone.
