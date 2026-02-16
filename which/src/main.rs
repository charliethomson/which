use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

/// Locate executables on PATH
#[derive(Parser)]
struct Args {
    /// Exit with 0 if found, 1 if not; suppress all output
    #[arg(short)]
    silent: bool,
    /// Print all matches, not just the first
    #[arg(short)]
    all: bool,
    /// Print at most N matches (ignored with -a)
    #[arg(short)]
    limit: Option<usize>,
    /// Enable debug tracing on stderr
    #[arg(short, long)]
    verbose: bool,
    /// Command names to look up
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

    let results = libwhich::which(&args.names)?;
    if args.silent {
        let count = results.count();
        return Ok(if count == 0 {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        });
    }

    let limit = if args.all {
        usize::MAX
    } else {
        args.limit.unwrap_or(1)
    };

    for path in results.take(limit) {
        println!("{}", path.display());
    }

    Ok(ExitCode::SUCCESS)
}
