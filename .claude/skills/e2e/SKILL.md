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
ssh m1 'cd /tmp/starter-checks && caffeinate -dimsu zsh -lc "e2e/run.sh android moves"'   # one lane, one test file (nextest's filter)
```

## Reading a run

- Each lane's line: `ssh m1 'grep -hE "^== [a-z0-9-]+: " /tmp/starter-e2e-*.log'` (passed, flaky, failed, skipped).
- `target/e2e/<lane>/`: `run.txt`, `server.log`, `driver.log`, `skipped.txt` (each skip with its reason), `failures/` (`<test>.txt` with the address and what failed, `.png`, `.html`, `.server.log`); a Simulator that would not boot: `boot-why.txt`.
- On the M1: `ssh m1 'cat /tmp/starter-checks/target/e2e/<lane>/failures/<test>.txt'`, or `scp` the folder.
- I read the logs and report; Davor gets no `tail -f` or watch command.

## A failure

- **In one browser: probe first.** A small probe test that prints what the page saw (the events, the element on top, the address, the history state); run it, name the cause with evidence, then fix the harness or the site. A guess in the islands app on this stack (2026-10-10) broke three browsers that had passed.
- **In several browsers:** suspect the site.
- **A departure from the islands app's code** (`public/js/`, the harness) is tested on its lanes before it counts: passing `parseHTMLUnsafe` options broke every swap in Chrome 145 (2026-10-29).
- After any harness change, run the other browsers again.

## Known on the devices

- iOS: after an Xcode update, `sudo xcodebuild -runFirstLaunch` first (docs/prerequisites.md). After a Mac restart, "runtime profile not found" until `killall -9 com.apple.CoreSimulator.CoreSimulatorService`. Xcode 27 has no Simulator app: its window is DeviceHub.
- Android: `run.sh` reads the emulator's Chrome as it boots and names the chromedriver it misses. The Play Store moves the low emulators' Chrome on (124 to 156, 2026-10-29); fetch the driver it names.
- Android: Chrome's "Translate page?" bubble eats the first tap outside it. A bubble in a failure's screenshot means that AVD has the offer on (Settings › Languages).
- An emulator that crashes is named in the lane's summary with its report.
