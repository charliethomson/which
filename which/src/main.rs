use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

/// Locate executables on PATH
#[derive(Parser)]
#[command(version = env!("APP_VERSION"))]
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
    /// Program names to look up
    #[arg(required = true)]
    names: Vec<String>,
}

fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();

    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
            )
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(std::io::stderr)
            .init();
    }

    let limit = if args.all {
        usize::MAX
    } else {
        args.limit.unwrap_or(1)
    };

    // Like which(1): each name is looked up on its own, the exit status is 1 if any name
    // had no match, and `-s` only changes whether the matches are printed.
    let dirs = libwhich::search_paths()?;
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
