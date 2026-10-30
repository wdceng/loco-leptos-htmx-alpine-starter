---
name: e2e
description: Use when running, reading or fixing this starter's browser tests (e2e/run.sh, the twenty lanes, the M1, the Simulators and emulators). They run only when Davor says so; while developing they are written and compiled, never run.
---

# Browser tests

What each test covers, the lanes and their ports: [docs/testing.md](../../../docs/testing.md), "Browser tests" and "The Low Bar". The drivers, Simulators and emulators: [docs/prerequisites.md](../../../docs/prerequisites.md).

## When

- Only when Davor says so ("run e2e", "run the lanes"). A go on a change is not a go to run them.
- While developing: write the test and compile it, `cd e2e && cargo fmt --check && CARGO_TARGET_DIR=../target/claude-e2e cargo clippy --all-targets -- -D warnings`. A plan lists it as "written, not run".

## Where

- The M1 Max (64 GB, `ssh m1`): `all` runs the nine top lanes at once; `run.sh` reads the chip and the memory by `sysctl`. Then `low` (two batches on every Mac) and `beta`, one after another, never two families at once.
- The M5: `main`, then `tablets`; never the nine at once (a 24 GB Mac ran out of memory, 2026-10-12).
- Never beside another project's run: the apps on this stack share the Simulators and the emulators. `pgrep -fl e2e/run.sh` on that Mac first.
- `run.sh` refuses while Safari or Safari Technology Preview is open.

## Running on the M1

The fix is made here, the tree copied to `/tmp/starter-checks`, the run started in the background (`run_in_background`), never waited on. The M1's tree is a copy: never commit from it.

```bash
rsync -a --delete --exclude .git --exclude target/ --exclude '*.sqlite*' --exclude 'secrets*.env' --exclude node_modules ./ m1:/tmp/starter-checks/   # the tree to the M1, its builds kept
ssh m1 'cd /tmp/starter-checks && caffeinate -dimsu zsh -lc "e2e/run.sh all > /tmp/starter-e2e-all.log 2>&1; e2e/run.sh low > /tmp/starter-e2e-low.log 2>&1; e2e/run.sh beta > /tmp/starter-e2e-beta.log 2>&1"'   # the three bars, one after another
ssh m1 'cd /tmp/starter-checks && caffeinate -dimsu zsh -lc "E2E_DEVICE_GAPS=\"5 5 5 5 5\" E2E_STAGGER=1 e2e/run.sh android moves"'   # one lane, one test file, the start gaps shrunk
```

- **A rerun of one lane or one test shrinks the gaps** (`E2E_DEVICE_GAPS`, `E2E_STAGGER`), or minutes go to waiting.
- **Wait on the background task's notice,** never on a loop of `pgrep -f`: it matches the waiting command itself.

## Reading a run

- Each lane's line: `ssh m1 'grep -hE "^== [a-z0-9-]+: " /tmp/starter-e2e-*.log'` (passed, flaky, failed, skipped). Before it, "booted <lane> in N s" or "did not boot <lane>: why"; `BROKEN` for a lane whose driver stopped opening sessions.
- **Flaky is not passed:** report it with its lane; a second run of that lane alone says whether it's the test or the load.
- `target/e2e/<lane>/`: `run.txt`, `server.log`, `driver.log`, `skipped.txt` (each skip with its reason), `failures/` (`<test>.txt` with the address and what failed, `.png`, `.html`, `.server.log`); a Simulator that would not boot: `boot-why.txt`.
- On the M1: `ssh m1 'cat /tmp/starter-checks/target/e2e/<lane>/failures/<test>.txt'`, or `scp` the folder.
- I read the logs and report; Davor gets no `tail -f` or watch command.

## A failure

- **In one browser: probe first.** A small probe test that prints what the page saw (the events, the element on top, the address, the history state); run it, name the cause with evidence, then fix the harness or the site. A guess in the islands app on this stack (2026-10-10) broke three browsers that had passed.
- **In several browsers:** suspect the site first, then the harness.
- **A site bug is fixed,** not only reported, and named in the summary with its cause.
- **A departure from the islands app's code** (`public/js/`, the harness) is tested on its lanes before it counts: passing `parseHTMLUnsafe` options broke every swap in Chrome 145 (2026-10-29).
- After any harness change (`e2e/src/`, `run.sh`), rerun the lanes that passed before, not only the one that failed.

## Already handled, don't fix again

Chrome's slow first start (`DevToolsActivePort`, retried once, `e2e/src/session.rs`); a Simulator's lost runtime profile after a Mac restart (`code=401`, CoreSimulator restarted and the boot retried); a Simulator whose first session fails (restarted once, `boot-why.txt`); the emulators' Vulkan (off: MoltenVK crashed); Safari open on the Mac (refused, "Safari is open"); a crashed emulator (named in the summary with its report); a missing chromedriver for an emulator's Chrome (named, with the `M` to fetch).

## Known on the devices

- iOS: after an Xcode update, `sudo xcodebuild -runFirstLaunch` first (docs/prerequisites.md). Xcode 27 has no Simulator app: its window is DeviceHub.
- Android, the low bar: the emulators' Play Store is off and their Chrome the image's 124; `run.sh` stops a low lane whose Chrome is anything else ("the low bar's Chrome is …"). The fix is in docs/testing.md, The Low Bar, and the translate offer goes off again after it.
- Android: Chrome's "Translate page?" bubble eats the first tap outside it. A bubble in a failure's screenshot means that AVD has the offer on (Settings › Languages).
