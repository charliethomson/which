//! Every pipeline this repo runs: the fleet's `rust.ci` gate from libci, and a second
//! pipeline for the two legs the template has no knob for, the `no-default-features`
//! build and the Windows cross-check.
//!
//! Regenerate the committed files after changing anything here:
//!
//! ```text
//! cargo run --manifest-path .kiln/Cargo.toml -- describe > .kiln/lock.json
//! cargo run --manifest-path .kiln/Cargo.toml -- emit woodpecker
//! ```

use kiln::prelude::*;
use kiln::recipes::git::SshKey;
use kiln::recipes::rust::{Cargo, CargoClippy, CargoTest, Rustup};
use libci::secrets::git::{KILN_SECRETS, SshDeployKey};
use libci::templates::{RustCi, TOOLCHAIN_IMAGE};

/// What a change to the Rust tree touches. `.kiln/**` is added by the template.
const WATCH: &[&str] = &["Cargo.toml", "libwhich/**", "which/**"];

/// The Windows validation in `libwhich` is only ever compiled here, so the gate
/// cross-checks it: a target without a linker is enough for clippy.
const WINDOWS_TARGET: &str = "x86_64-pc-windows-gnu";

fn main() -> kiln::Result<()> {
    kiln::repo("which")
        .secrets(KILN_SECRETS)
        // kiln and libci are private: the key that clones this repo on a native agent
        // is also what lets every agent fetch `.kiln/`'s dependencies. A repo with a
        // checkout key is push-only: nothing runs on pull requests.
        .checkout_key(SshDeployKey)
        // No coverage job: a library has no floor (https://standards.thmsn.dev/d/testing).
        // The cap is small because the workspace is: two crates over clap and tracing.
        .pipeline(
            RustCi::new(".")
                .without_coverage()
                .target_cap(4, 8)
                .watch(WATCH.iter().copied())
                .pipeline(),
        )
        .pipeline(legs())
        .main()
}

/// The gate's other legs, in one job so they share a warm target dir.
fn legs() -> Pipeline {
    let paths = WATCH.iter().copied().chain([".kiln/**"]);
    Pipeline::new("rust.ci-legs")
        .describe("no-default-features · windows cross-check")
        .on(Push::to("main").paths(paths))
        .on(Manual)
        .serialize(Always::cancel())
        .job("legs", |j| {
            Cargo::ci_env()
                .build_jobs(2)
                .apply(j.on(Linux).image(TOOLCHAIN_IMAGE))
                .env("RELEASE_VERSION", "0.0.0-ci")
                .cache(Cargo::target("legs").max_gb(4).min_free_gb(8))
                .needs_secrets(SshDeployKey)
                .timeout(mins(30))
                .run(run_legs)
        })
}

fn run_legs(ctx: &Ctx) -> Result<()> {
    Rustup::components(ctx, ["clippy"]).call()?;
    Rustup::targets(ctx, [WINDOWS_TARGET]).call()?;

    // Held to the end of the body: the key is on disk for the fetches and no longer.
    let key = SshKey::from_secret(ctx, &ctx.secret(SshDeployKey))?;
    let git = key.env();
    ctx.step("no-default-features", |ctx| {
        git.apply(
            Cargo::test(ctx).package("libwhich").no_default_features(),
            CargoTest::env,
        )
        .call()
    })?;
    ctx.step("windows", |ctx| {
        git.apply(
            Cargo::clippy(ctx)
                .workspace()
                .all_targets()
                .target(WINDOWS_TARGET)
                .deny_warnings(),
            CargoClippy::env,
        )
        .call()
    })?;
    drop(key);
    Ok(())
}
