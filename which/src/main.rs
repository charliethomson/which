use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short)]
    silent: bool,
    #[arg(short)]
    all: bool,
    #[arg(short)]
    limit: Option<usize>,
    names: Vec<String>,
}

fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();
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
