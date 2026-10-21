//! The README walkthrough, kept compiling: `cargo run -p libwhich --example readme`.

use libwhich::{WhichResult, which, which_in};

fn main() -> WhichResult<()> {
    for path in which(&["ls", "cat"])? {
        println!("{}", path.display());
    }
    let first = which(&["python3"])?.next();
    println!("{first:?}");
    let hits: Vec<_> = which_in(&["sh"], ["/bin", "/usr/bin"]).collect();
    println!("{hits:?}");
    Ok(())
}
