# flaketest

Run a test command N times, find out which tests are actually flaky.
Today's fixes are CI-platform-specific SaaS (BuildPulse) or nothing —
this is a plain wrapper binary, works with any test runner your CI
already calls.

## Installation

```sh
cargo install --path .
```

## Usage

```bash
flaketest --runs 20 -- cargo test
flaketest --runs 50 --cwd ./some-project -- npm test
```

Exit code `0` — no flakiness found. `1` — flaky test(s) found (named
individually if `cargo test`'s output format was recognized, or reported
as whole-command flakiness otherwise). `2` — consistently failing, which
is a different, more urgent problem than flakiness and reported as such,
not lumped in with it.

## A flaky test vs. a broken test are different problems, reported differently

A test that fails **every** run isn't flaky — it's just broken, and
conflating the two would bury real nondeterminism-hunting under routine
test failures. Flagged as flaky only if it passed on **some** runs and
failed on **others**.

## Per-test parsing, not just whole-command exit codes — and this is the part that actually matters

`cargo test`'s own `test name ... ok`/`FAILED` line format is parsed to
identify *which specific test* is flaky, not just "the suite sometimes
fails." Any other test runner's output falls back to whole-command-level
flakiness (some runs of the entire command passed, others didn't) —
still useful, just less precise. See Status below for why the per-test
signal is the one that actually matters in practice, not a nice-to-have.

## Status: built, and the payoff scenario verified against a real, deliberately flaky test suite

- **12 unit tests** (`cargo test --lib`): `cargo test` output parsing
  (extracting pass/fail per test name, correctly **not** mis-parsing the
  `test result: FAILED. 2 passed; 1 failed...` summary line as if it were
  itself a test); flakiness classification — passes-and-fails-across-runs
  is flaky, always-fails is not flaky (it's broken), always-passes is not
  flaky, and a test **absent** from some runs (a crash before it ran) is
  judged only on the runs it actually appeared in, not skewed by the gap;
  and the process runner against **real spawned shell processes**, not
  mocked output — including one that captures a genuinely nondeterministic
  real process (nanosecond-clock parity) alternating pass/fail across 20
  real runs, confirming `run_n_times` doesn't collapse or memoize results.
- **The actual payoff scenario, run for real, not simulated**: built a
  throwaway fixture crate with three tests — one that always passes, one
  that always fails, and one **genuinely flaky** (also nanosecond-parity
  based, a real nondeterministic test, not a mock of one) — then ran
  `flaketest --runs 20 -- cargo test` against it for real. Result:
  ```
  0/20 runs of the whole command succeeded

  1 flaky test(s) (passed on some runs, failed on others):
    tests::genuinely_flaky  (8 pass / 12 fail)
  ```
  **This is the exact scenario that proves per-test parsing isn't a
  nice-to-have**: the whole-command signal alone was `0/20` — completely
  useless, since the always-broken test fails the suite every single
  time — but the per-test analysis correctly drilled underneath that to
  find the one real, actionable flaky test, while correctly leaving the
  consistently-broken test unflagged as flaky at all.

**Not done / deliberately deferred**: parsers for other test frameworks'
output formats (Jest, pytest, `go test` each have their own line format —
cargo test is the only one implemented; the whole-command fallback covers
everything else today, just without naming individual tests); parallel
run execution (`--runs 50` currently runs sequentially — fine for
CI-adjacent use, slower than it could be for a large `--runs` count
locally).
