use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub struct RunResult {
    pub succeeded: bool,
    pub combined_output: String,
}

/// Runs `program args...` once in `cwd`, capturing stdout+stderr merged
/// (test frameworks split their per-test result lines and summary across
/// both streams inconsistently enough — cargo test's "failures:" section
/// on stdout, some frameworks' progress on stderr — that merging is the
/// only way `parse.rs` reliably sees everything in original order).
pub fn run_once(program: &str, args: &[String], cwd: &Path) -> Result<RunResult> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("running {program} {args:?}"))?;

    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    Ok(RunResult {
        succeeded: output.status.success(),
        combined_output: combined,
    })
}

pub fn run_n_times(program: &str, args: &[String], cwd: &Path, n: usize) -> Result<Vec<RunResult>> {
    (0..n).map(|_| run_once(program, args, cwd)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_success_and_combined_output_of_a_real_process() {
        let result = run_once(
            "sh",
            &[
                "-c".to_string(),
                "echo hello-stdout; echo hello-stderr >&2".to_string(),
            ],
            Path::new("."),
        )
        .unwrap();
        assert!(result.succeeded);
        assert!(result.combined_output.contains("hello-stdout"));
        assert!(result.combined_output.contains("hello-stderr"));
    }

    #[test]
    fn captures_failure_of_a_real_process() {
        let result = run_once(
            "sh",
            &["-c".to_string(), "exit 1".to_string()],
            Path::new("."),
        )
        .unwrap();
        assert!(!result.succeeded);
    }

    #[test]
    fn runs_the_requested_number_of_times() {
        let results = run_n_times(
            "sh",
            &["-c".to_string(), "true".to_string()],
            Path::new("."),
            4,
        )
        .unwrap();
        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|r| r.succeeded));
    }

    #[test]
    fn a_command_that_genuinely_alternates_produces_a_mix_of_outcomes() {
        // A real nondeterministic process (parity of nanosecond time) —
        // not asserting an exact pattern, just that running it several
        // times captures both outcomes at least once each across enough
        // tries, proving `run_n_times` doesn't collapse or memoize results.
        let script = "test $(( $(date +%N) % 2 )) -eq 0";
        let results = run_n_times(
            "sh",
            &["-c".to_string(), script.to_string()],
            Path::new("."),
            20,
        )
        .unwrap();
        let successes = results.iter().filter(|r| r.succeeded).count();
        assert!(
            successes > 0 && successes < 20,
            "expected a genuine mix over 20 runs, got {successes}/20 successes"
        );
    }
}
