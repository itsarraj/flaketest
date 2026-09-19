use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

fn cargo_test_line() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // `test module::name::case ... ok` / `... FAILED` — cargo test's own
    // per-test result line format.
    RE.get_or_init(|| Regex::new(r"^test (\S+) \.\.\. (ok|FAILED)\s*$").unwrap())
}

/// Best-effort per-test pass/fail extraction from `cargo test` output.
/// Not a general test-framework parser — this recognizes exactly cargo
/// test's own line format, since that's the one format this workspace's
/// own tools are guaranteed to produce, and it's a reasonable default for
/// anyone else building Rust. A test name absent from the map means the
/// output didn't contain a recognizable line for it (a crash before it
/// ran, a different framework entirely) — the caller (`analyze.rs`) only
/// ever reasons about tests it actually saw a result for.
pub fn parse_cargo_test_output(output: &str) -> HashMap<String, bool> {
    let mut results = HashMap::new();
    for line in output.lines() {
        if let Some(caps) = cargo_test_line().captures(line.trim_end()) {
            let name = caps[1].to_string();
            let passed = &caps[2] == "ok";
            results.insert(name, passed);
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_passing_and_failing_tests() {
        let output = "\
running 3 tests
test module::a_passing_test ... ok
test module::a_failing_test ... FAILED
test module::another_pass ... ok

failures:
    module::a_failing_test
";
        let results = parse_cargo_test_output(output);
        assert_eq!(results.get("module::a_passing_test"), Some(&true));
        assert_eq!(results.get("module::a_failing_test"), Some(&false));
        assert_eq!(results.get("module::another_pass"), Some(&true));
        assert_eq!(
            results.len(),
            3,
            "the 'failures:' summary line must not itself be parsed as a test"
        );
    }

    #[test]
    fn output_with_no_recognizable_lines_yields_an_empty_map() {
        let output = "some random compiler output\nerror: could not compile\n";
        assert!(parse_cargo_test_output(output).is_empty());
    }

    #[test]
    fn ignores_the_test_result_summary_line() {
        let output =
            "test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n";
        assert!(parse_cargo_test_output(output).is_empty());
    }
}
