use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct FlakyTest {
    pub name: String,
    pub passes: usize,
    pub failures: usize,
}

/// A test is flaky if it appears in the per-run results with **both**
/// outcomes at least once — a test that's always failing isn't flaky,
/// it's just broken, and that distinction matters: a broken test needs
/// fixing, a flaky one needs investigating for nondeterminism.
pub fn find_flaky_tests(runs: &[HashMap<String, bool>]) -> Vec<FlakyTest> {
    let mut tally: HashMap<&str, (usize, usize)> = HashMap::new();
    for run in runs {
        for (name, &passed) in run {
            let entry = tally.entry(name.as_str()).or_insert((0, 0));
            if passed {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }

    let mut flaky: Vec<FlakyTest> = tally
        .into_iter()
        .filter(|(_, (passes, failures))| *passes > 0 && *failures > 0)
        .map(|(name, (passes, failures))| FlakyTest {
            name: name.to_string(),
            passes,
            failures,
        })
        .collect();
    flaky.sort_by(|a, b| a.name.cmp(&b.name));
    flaky
}

/// Whole-command flakiness (exit-code level) — the fallback signal for
/// any test runner whose output `parse.rs` can't understand: were some
/// runs of the *entire command* successful and others not.
pub fn command_is_flaky(run_succeeded: &[bool]) -> bool {
    run_succeeded.iter().any(|&s| s) && run_succeeded.iter().any(|&s| !s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, bool)]) -> HashMap<String, bool> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn a_test_that_passes_and_fails_across_runs_is_flaky() {
        let runs = vec![
            map(&[("a", true), ("b", true)]),
            map(&[("a", false), ("b", true)]),
            map(&[("a", true), ("b", true)]),
        ];
        let flaky = find_flaky_tests(&runs);
        assert_eq!(flaky.len(), 1);
        assert_eq!(flaky[0].name, "a");
        assert_eq!(flaky[0].passes, 2);
        assert_eq!(flaky[0].failures, 1);
    }

    #[test]
    fn a_consistently_failing_test_is_not_flaky_its_just_broken() {
        let runs = vec![
            map(&[("always_broken", false)]),
            map(&[("always_broken", false)]),
        ];
        assert!(find_flaky_tests(&runs).is_empty());
    }

    #[test]
    fn a_consistently_passing_test_is_not_flaky() {
        let runs = vec![map(&[("stable", true)]), map(&[("stable", true)])];
        assert!(find_flaky_tests(&runs).is_empty());
    }

    #[test]
    fn a_test_missing_from_some_runs_is_judged_only_on_the_runs_it_appeared_in() {
        // e.g. a run that crashed before reaching this test — absence, not
        // a failure, so it must not skew the pass/fail tally.
        let runs = vec![map(&[("a", true)]), map(&[]), map(&[("a", false)])];
        let flaky = find_flaky_tests(&runs);
        assert_eq!(flaky.len(), 1);
        assert_eq!(flaky[0].passes, 1);
        assert_eq!(flaky[0].failures, 1);
    }

    #[test]
    fn whole_command_flakiness_requires_both_a_success_and_a_failure() {
        assert!(command_is_flaky(&[true, false, true]));
        assert!(!command_is_flaky(&[true, true, true]));
        assert!(!command_is_flaky(&[false, false]));
    }
}
