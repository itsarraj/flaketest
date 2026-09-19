use std::path::PathBuf;

use clap::Parser;
use flaketest::{analyze, parse, runner};

#[derive(Parser)]
#[command(
    name = "flaketest",
    about = "Run a test command N times, find out which tests are actually flaky"
)]
struct Cli {
    #[arg(long, default_value_t = 10)]
    runs: usize,
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    /// The test command to run, after `--`, e.g. `flaketest --runs 20 -- cargo test`.
    #[arg(last = true, required = true)]
    command: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (program, args) = cli
        .command
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("no command given"))?;
    let args: Vec<String> = args.to_vec();

    println!(
        "running `{program} {}` {} time(s)...",
        args.join(" "),
        cli.runs
    );
    let results = runner::run_n_times(program, &args, &cli.cwd, cli.runs)?;

    let succeeded: Vec<bool> = results.iter().map(|r| r.succeeded).collect();
    let pass_count = succeeded.iter().filter(|&&s| s).count();
    println!(
        "{pass_count}/{} runs of the whole command succeeded",
        cli.runs
    );

    let per_run_tests: Vec<_> = results
        .iter()
        .map(|r| parse::parse_cargo_test_output(&r.combined_output))
        .collect();
    let flaky = analyze::find_flaky_tests(&per_run_tests);

    if !flaky.is_empty() {
        println!(
            "\n{} flaky test(s) (passed on some runs, failed on others):",
            flaky.len()
        );
        for f in &flaky {
            println!("  {}  ({} pass / {} fail)", f.name, f.passes, f.failures);
        }
        std::process::exit(1);
    }

    if analyze::command_is_flaky(&succeeded) {
        println!(
            "\nthe overall command is flaky (some runs passed, some failed), but no per-test names were recognized in its output — only cargo test's own format is parsed today"
        );
        std::process::exit(1);
    }

    if pass_count == 0 {
        println!("\nconsistently failing across all runs — that's broken, not flaky");
        std::process::exit(2);
    }

    println!("\nno flakiness detected across {} runs", cli.runs);
    Ok(())
}
