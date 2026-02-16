use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
struct Args {
    #[arg(short)]
    silent: bool,
    #[arg(short)]
    all: bool,
    #[arg(short)]
    limit: Option<usize>,
    #[arg(short, long)]
    verbose: bool,
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
