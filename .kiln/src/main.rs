//! Every pipeline this repo runs.
//!
//! | pipeline | what | where |
//! |---|---|---|
//! | `rust.ci` | libci's gate: fmt, build, clippy, test | Linux |
//! | `rust.ci-matrix` | clippy and the tests natively on macOS and Windows; the `no-default-features` build on Linux | all three agents |
//! | `cli.release` | the `which` binary for macOS and Windows, attached to one registry version, then flipped live | the Mac and the Windows box, then Linux |
//!
//! Regenerate the committed files after changing anything here:
//!
//! ```text
//! cargo run --manifest-path .kiln/Cargo.toml -- check
//! cargo run --manifest-path .kiln/Cargo.toml -- describe > .kiln/lock.json
//! cargo run --manifest-path .kiln/Cargo.toml -- emit woodpecker
//! ```

use kiln::job::JobBuilder;
use kiln::prelude::*;
use kiln::recipes::rust::{Cargo, Rustup};
use kiln::recipes::windows::Toolchain;
use libci::registry::{AppRegistry, release_notes};
use libci::secrets::{SshDeployKey, git, registry};
use libci::templates::{RustCi, TOOLCHAIN_IMAGE};
use libci::version::Version;

/// What a change to the Rust tree touches. The template adds `.kiln/**` itself; the
/// pipelines written here name it.
const WATCH: &[&str] = &["Cargo.toml", "libwhich/**", "which/**", ".kiln/**"];

/// The registry identity (https://standards.thmsn.dev/d/registry-publishing): the product
/// is the repo's reverse-DNS root, the app is the surface, the binary is what lands on
/// `PATH`.
const PRODUCT: &str = "dev.thmsn.which";
const APP: &str = "cli";
const BIN: &str = "which";
const INSTALL_MANIFEST: &str = "which/packaging/install.json";
const PRODUCT_META: &str = "which/packaging/meta.json";

/// One released target. Built natively on its own agent: the Windows one is the point of
/// the release, and cross-compiling would mean never running the binary it ships.
struct Leg {
    agent: Agent,
    /// The registry's target token, and the rust triple.
    slug: &'static str,
    triple: &'static str,
}

/// Adding a target is one line here; `finalize` derives its required set from this list,
/// so a leg cannot be forgotten there.
const LEGS: &[Leg] = &[
    Leg {
        agent: Macos,
        slug: "darwin-arm64",
        triple: "aarch64-apple-darwin",
    },
    Leg {
        agent: Windows,
        slug: "windows-amd64",
        triple: "x86_64-pc-windows-msvc",
    },
];

fn main() -> kiln::Result<()> {
    kiln::repo("which")
        .secrets(git::KILN_SECRETS)
        .secrets(registry::KILN_SECRETS)
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
        .pipeline(matrix())
        .pipeline(release())
        .main()
}

/// The gate's job shape: the CI cargo environment, a capped target cache, half an hour.
fn gate_job(j: JobBuilder<Agent>, cache: &str) -> JobBuilder<Agent> {
    Cargo::ci_env()
        .build_jobs(2)
        .apply(j)
        // Pinned: the real version changes every commit, and a build that baked it would
        // rebuild on every run for a stamp the gate never reads.
        .env("RELEASE_VERSION", "0.0.0-ci")
        .cache(Cargo::target(cache).max_gb(4).min_free_gb(8))
        .timeout(mins(30))
}

/// `rust.ci` runs the gate in a Linux container. The library's validation is per
/// platform (`access(2)` on Unix, `PATHEXT` on Windows), so the same clippy and tests
/// run natively on the Mac and the Windows box too. Linux only adds the leg the template
/// has no knob for.
fn matrix() -> Pipeline {
    Pipeline::new("rust.ci-matrix")
        .describe("clippy and tests natively on macOS and Windows; no-default-features on Linux")
        .on(Push::to("main").paths(WATCH.iter().copied()))
        .on(Manual)
        .serialize(Always::cancel())
        .job("linux", |j| {
            gate_job(j.on(Linux).image(TOOLCHAIN_IMAGE), "matrix").run(|ctx| gate(ctx, false))
        })
        .job("macos", |j| {
            gate_job(j.on(Macos), "matrix").run(|ctx| gate(ctx, true))
        })
        .job("windows", |j| {
            gate_job(j.on(Windows), "matrix").run(|ctx| gate(ctx, true))
        })
}

/// `full`: clippy and the tests over the workspace, as `rust.ci` runs them on Linux.
/// Always: the library without its default features, which is the other validation
/// path on Unix.
fn gate(ctx: &Ctx, full: bool) -> Result<()> {
    Rustup::components(ctx, ["clippy"]).call()?;
    if full {
        Cargo::clippy(ctx)
            .workspace()
            .all_targets()
            .deny_warnings()
            .call()?;
        Cargo::test(ctx).workspace().call()?;
    }
    Cargo::clippy(ctx)
        .package("libwhich")
        .all_targets()
        .no_default_features()
        .deny_warnings()
        .call()?;
    Cargo::test(ctx)
        .package("libwhich")
        .no_default_features()
        .call()
}

/// One job per target, each attaching its archive to the same draft version with
/// `--no-finalize`, and one fan-in job that flips the version live once every target is
/// there. A failed leg leaves a draft nobody can install, never a live version missing a
/// platform. Never cancelled: a release half-done is worse than one that finishes late.
fn release() -> Pipeline {
    let mut pipeline = Pipeline::new("cli.release")
        .describe("the which binary for macOS and Windows, published as one registry version")
        .on(Push::to("main").paths(WATCH.iter().copied()))
        .on(Manual)
        .serialize(Never::cancel());

    for leg in LEGS {
        pipeline = pipeline.job(leg.slug, move |j| {
            AppRegistry::declare(Cargo::ci_env().build_jobs(2).apply(j.on(leg.agent)))
                .cache(Cargo::target(leg.slug).max_gb(4).min_free_gb(8))
                .timeout(mins(45))
                .run(move |ctx| build_leg(ctx, leg))
        });
    }

    pipeline.job("finalize", |j| {
        // Needs nothing from the builds, only git to derive the same version.
        AppRegistry::declare(j.on(Linux).image(TOOLCHAIN_IMAGE))
            .needs(LEGS.iter().map(|leg| leg.slug))
            .timeout(mins(10))
            .run(finalize)
    })
}

/// `MAJOR.MINOR.<commit count>` (https://standards.thmsn.dev/d/versioning). Untagged is
/// allowed because this repo has never been tagged: it versions as `0.0.<count>` until
/// the first `v0.1` tag lands, at which point this becomes `Version::derive`, so a failed
/// tag fetch can no longer publish below the live version.
fn version(ctx: &Ctx) -> Result<Version> {
    Version::derive_untagged_ok(ctx)
}

fn build_leg(ctx: &Ctx, leg: &Leg) -> Result<()> {
    let v = version(ctx)?;
    ctx.info(format!("building {BIN} {v} for {}", leg.slug));

    // RELEASE_VERSION is what build.rs bakes into `--version`, so the binary and the
    // registry version are one value.
    let bin = Cargo::build(ctx)
        .package(BIN)
        .release()
        .target(leg.triple)
        .env("RELEASE_VERSION", v.release_version())
        .call()?
        .binary(BIN)?;

    // Prove the stamp before publishing it. A mismatch ships a binary reporting a
    // version other than the one the registry lists, and the installer's up-to-date
    // check then reinstalls it forever. Every leg builds natively, so the binary runs
    // on the agent that built it.
    let reported = ctx.run(&bin, ["--version"]).capture()?;
    let expected = format!("{BIN} {v}");
    if reported.trim() != expected {
        return Err(ctx.err(format!(
            "the binary reports `{reported}`, expected `{expected}`"
        )));
    }

    // The bare binary at the archive root. The PowerShell installer unpacks a zip and
    // finds `which.exe` anywhere inside; the sh installer unpacks a tarball the same way.
    let archive = match leg.agent {
        Windows => ctx.zip(&bin, format!("dist/{BIN}-{}.zip", leg.slug))?,
        _ => ctx.tar_gz([&bin], format!("dist/{BIN}-{}.tar.gz", leg.slug))?,
    };

    let registry = connect(ctx, leg.agent)?;
    // Idempotent, and retried once: the legs run concurrently, so on the very first
    // release both can see "not found", both create, and the loser gets a conflict.
    // By the retry the entity exists and the call is the no-op path.
    if let Err(error) = ensure(&registry) {
        ctx.warn(format!("registering {PRODUCT}/{APP} failed ({error}); retrying once, the other leg may have just created it"));
        ensure(&registry)?;
    }
    // `--install` and the notes on every leg: `publish-cli` sends them with the draft's
    // create call only, and whichever leg gets there first creates it.
    registry
        .publish_cli(PRODUCT, APP, &v)
        .artifact([leg.slug], &archive)
        .bin_name(BIN)
        .install(INSTALL_MANIFEST)
        .notes(release_notes(ctx, &format!("{BIN} {v}"))?)
        .no_finalize()
        .skip_existing()
        .call()
}

/// The registry client. On Windows it runs under Git for Windows' bash, by path: the
/// agent is a `LocalSystem` service and `bash` is not on its `PATH`.
fn connect(ctx: &Ctx, agent: Agent) -> Result<AppRegistry<'_>> {
    let registry = AppRegistry::connect(ctx)?;
    Ok(if agent == Windows {
        registry.with_bash(Toolchain::discover(ctx)?.git_bash()?)
    } else {
        registry
    })
}

fn ensure(registry: &AppRegistry<'_>) -> Result<()> {
    registry.ensure_product(PRODUCT).meta(PRODUCT_META).call()?;
    registry
        .ensure_app(PRODUCT, APP)
        .cli()
        .name(BIN)
        .bin_name(BIN)
        .install(INSTALL_MANIFEST)
        .call()
}

/// The one definition of "complete". `Finalize::call` fails unless the registry actually
/// finalized, so an incomplete set is red here rather than a green tick over a draft.
fn finalize(ctx: &Ctx) -> Result<()> {
    let v = version(ctx)?;
    let required: Vec<&str> = LEGS.iter().map(|leg| leg.slug).collect();
    ctx.info(format!(
        "finalizing {BIN} {v} (requires: {})",
        required.join(", ")
    ));
    AppRegistry::connect(ctx)?
        .finalize(PRODUCT, APP, &v)
        .require_targets(required)
        .call()
}
