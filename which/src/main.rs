use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use libproduct::product_name;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

// The identifier the registry, the logs and `--build-info` know this binary by
// (https://standards.thmsn.dev/d/identifiers). Declared here, published in `main`.
product_name!("dev.thmsn.which.cli");

/// Locate executables on PATH
#[derive(Parser)]
#[command(name = "which", version = env!("APP_VERSION"))]
// Each bool is one flag; clap owns the struct's shape.
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// Exit with 0 if every name is found, 1 otherwise; print nothing
    #[arg(short)]
    silent: bool,
    /// Print every match for each name, not just the first
    #[arg(short)]
    all: bool,
    /// Print at most N matches per name (ignored with -a)
    #[arg(short, value_name = "N")]
    limit: Option<usize>,
    /// Enable debug tracing on stderr
    #[arg(short, long)]
    verbose: bool,
    /// Print what this binary was built from (version, commit, host) and exit
    #[arg(long)]
    build_info: bool,
    /// Print a completion script for SHELL and exit
    #[arg(long, value_name = "SHELL", value_enum)]
    completions: Option<Shell>,
    /// Program names to look up
    #[arg(required_unless_present_any = ["build_info", "completions"])]
    names: Vec<String>,
}

impl Args {
    /// Matches to print per name.
    fn limit(&self) -> usize {
        if self.all {
            usize::MAX
        } else {
            self.limit.unwrap_or(1)
        }
    }
}

fn main() -> anyhow::Result<ExitCode> {
    // Bound to a local first: the macro expands to a const holding a LazyLock, and
    // borrowing it inline would act on a fresh copy.
    let product_name = PRODUCT_NAME;
    product_name.set_global().map_err(anyhow::Error::msg)?;

    let args = Args::parse();

    if let Some(shell) = args.completions {
        clap_complete::generate(shell, &mut Args::command(), "which", &mut std::io::stdout());
        return Ok(ExitCode::SUCCESS);
    }
    if args.build_info {
        println!("{}", product_name.descriptor(libproduct::format::FULL));
        return Ok(ExitCode::SUCCESS);
    }

    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
            )
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(std::io::stderr)
            .init();
    }

    // Like which(1): each name is looked up on its own, the exit status is 1 if any name
    // had no match, and `-s` only changes whether the matches are printed.
    let dirs = libwhich::search_paths()?;
    let limit = args.limit();
    let mut all_found = true;
    for name in &args.names {
        let mut found = false;
        for path in libwhich::which_in(&[name], &dirs).take(limit) {
            found = true;
            if !args.silent {
                println!("{}", path.display());
            }
        }
        all_found &= found;
    }

    Ok(if all_found {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use clap::ValueEnum;

    use super::*;

    fn parse(argv: &[&str]) -> Result<Args, clap::Error> {
        Args::try_parse_from(std::iter::once("which").chain(argv.iter().copied()))
    }

    #[test]
    fn the_command_line_is_well_formed() {
        Args::command().debug_assert();
    }

    #[test]
    fn names_are_required_unless_the_run_only_prints_something_about_itself() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["-a"]).is_err());
        assert!(parse(&["ls"]).is_ok());
        assert!(parse(&["--completions", "zsh"]).is_ok());
        assert!(parse(&["--build-info"]).is_ok());
    }

    #[test]
    fn flags_parse_and_the_limit_follows_them() {
        let args = parse(&["-a", "-l", "2", "ls", "cat"]).unwrap();
        assert!(args.all);
        assert_eq!(args.names, ["ls", "cat"]);
        assert_eq!(args.limit(), usize::MAX, "-a wins over -l");

        assert_eq!(parse(&["-l", "2", "ls"]).unwrap().limit(), 2);
        assert_eq!(parse(&["ls"]).unwrap().limit(), 1);
        assert!(parse(&["-s", "-v", "ls"]).unwrap().silent);
    }

    #[test]
    fn completions_generate_for_every_shell() {
        for shell in Shell::value_variants() {
            let mut out = Vec::new();
            clap_complete::generate(*shell, &mut Args::command(), "which", &mut out);
            let script = String::from_utf8(out).unwrap();
            assert!(script.contains("which"), "{shell}: {script}");
        }
    }

    #[test]
    fn the_version_is_the_derived_one() {
        let version = Args::command().get_version().unwrap().to_owned();
        assert_eq!(version, env!("APP_VERSION"));
        assert_eq!(version.split('.').count(), 3, "{version}");
    }
}
