# which

**Find out which file a program name would run, from Rust or from a shell.**

A Rust workspace with two crates: `libwhich`, a library that locates executables on `PATH`
the way FreeBSD's `which(1)` does, and `which`, a CLI over it that ships to macOS and
Windows through the fleet's registry. Both work on Unix and Windows; on Windows the CLI is
the `which` the OS never had.

Docs: [Versioning](VERSIONING.md) · [Shortcuts](SHORTCUTS.md) · [Agents](AGENTS.md) ·
[Standards](https://standards.thmsn.dev)

## Install the CLI

```sh
curl -fsSL https://apps.dev.thmsn.dev/install/dev.thmsn.which | sh
```

```powershell
irm https://apps.dev.thmsn.dev/install/dev.thmsn.which.ps1 | iex
```

The installer picks the build for the machine, verifies its checksum, puts `which` on
`PATH` and installs shell completions. Re-running it upgrades to whatever is live.

## How it works

A lookup takes a list of names and a list of directories. For each name, in the order
given, it walks the directories in order and yields every candidate that is a valid
executable, so the first result for a name is what a shell would run and `-a` lists the
rest. The search is lazy: nothing is stat'd until the iterator is pulled.

```
names ─┬─ name ─┬─ dir/name            ─┐
       │        ├─ dir/name.EXE  (win) ─┼─ canonicalize ─ is it executable? ─ yield
       │        └─ …                   ─┘
       └─ a/b   ── checked at a/b, PATH ignored
```

What counts as executable is per platform, and is the whole reason the library exists:

| Platform | Rule |
|---|---|
| Unix, `libc` feature (default) | A regular file that `access(2)` says is executable for the calling user. Root gets a yes from `access` for any file, so root also needs an execute bit set. This is `which.c`'s `is_there`. |
| Unix, without `libc` | A regular file with any execute bit set. Ignores ownership and ACLs. |
| Windows | A regular file whose extension is in `PATHEXT` (default `.COM;.EXE;.BAT;.CMD;…`). A bare name is also tried with every `PATHEXT` extension appended, as `cmd.exe` does. |

Properties a caller would otherwise discover the hard way:

- **Results are canonical absolute paths.** Symlinks are resolved, so a result names the
  file that actually runs, and on Windows the `\\?\` prefix `canonicalize` adds is stripped.
- **A name with a path separator is checked where it points**, never joined onto `PATH`.
- **An empty `PATH` entry means the current directory**, as in `which.c`.
- **`PATH` is read once per call**, in `which`; `which_in` reads no environment at all.

## Layout

| Path | Role |
|---|---|
| [`libwhich/`](libwhich) | The library. `lib.rs` is the search, `util.rs` the per-platform validation, `tests.rs` the inline tests, `examples/readme.rs` the walkthrough below. |
| [`which/`](which) | The CLI. `build.rs` derives its version from git and embeds build info; `packaging/` holds the registry's install manifest and product metadata. |
| [`.kiln/`](.kiln) | CI as kiln pipelines. [`.woodpecker/`](.woodpecker) is generated from it. |
| [`scripts/cov.sh`](scripts/cov.sh) | Optional coverage report via `cargo tarpaulin`. |
| [`which.c`](which.c) | The FreeBSD `which(1)` source (BSD-3-Clause), kept as the reference for the Unix semantics. |

## Install the library

```toml
[dependencies]
libwhich = { git = "https://github.com/charliethomson/which" }
```

## Usage

```rust
use libwhich::{WhichResult, which, which_in};

fn main() -> WhichResult<()> {
    // Every match for each name, grouped by name, in PATH order.
    for path in which(&["ls", "cat"])? {
        println!("{}", path.display());
    }

    // The first match only: the iterator is lazy, so this stats until one hit.
    let first = which(&["python3"])?.next();
    println!("{first:?}");

    // Your own directories instead of PATH; no environment is read.
    let hits: Vec<_> = which_in(&["sh"], ["/bin", "/usr/bin"]).collect();
    println!("{hits:?}");
    Ok(())
}
```

That is [`libwhich/examples/readme.rs`](libwhich/examples/readme.rs); `cargo run -p libwhich --example readme`
runs it. `which` fails only when `PATH` is unset. `is_valid_executable(path)` and
`is_valid_executable_split(dir, name)` expose the validation on their own; `search_paths()`
returns `PATH` as directories.

### The CLI

```
which [-s] [-a] [-l N] [-v] <names>...
which --completions <shell> | --build-info | --version
```

| Flag | Effect |
|---|---|
| `-a` | Print every match for each name, not just the first |
| `-l N` | Print at most N matches per name (ignored with `-a`) |
| `-s` | Print nothing; the exit status is the answer |
| `-v` | Debug tracing on stderr (`RUST_LOG` narrows it) |
| `--completions <shell>` | A completion script for bash, zsh, fish, powershell or elvish; the installer runs this |
| `--build-info` | Version, commit, dirty flag and build host, via `libbuildinfo` |
| `--version` | The derived version, see [`VERSIONING.md`](VERSIONING.md) |

The exit status is 0 when every name was found and 1 otherwise, like `which(1)`. The
binary identifies itself as `dev.thmsn.which.cli`.

```sh
$ cargo run -q -p which -- -a python3
/usr/local/bin/python3
/usr/bin/python3
```

## Features

| Feature | Default | What it turns on |
|---|---|---|
| `libc` | yes | `access(2)`-based permission checks on Unix (the superuser-aware rule above) |
| `serde` | yes | `Serialize`/`Deserialize` on `WhichError`, keyed `dev.thmsn.which.…` per the [error-handling standard](https://standards.thmsn.dev/d/error-handling) |
| `valuable` | yes | `valuable::Valuable` on `WhichError`, for structured `tracing` fields |
| `tracing` | no | Spans and debug events inside the lookup |

## What stays in the consuming repo

The library finds files and says whether they are executable. It does not run them, does
not cache, and does not decide what to do when a name is missing: the iterator is simply
empty, and the CLI turns that into an exit status. Reading `PATH` is the one piece of
environment it touches, and only in `which`; a consumer that wants to search elsewhere
passes directories to `which_in`.

## Development

These are the commands CI runs on every push to `main`; passing them locally means
passing CI:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p libwhich --no-default-features
rustup target add x86_64-pc-windows-gnu
cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

CI is [kiln](https://github.com/charliethomson/kiln) on Woodpecker and runs only on `main`,
not on pull requests. `rust.ci` runs the gate on Linux; `rust.ci-matrix` runs clippy and
the tests natively on the Mac and the Windows agent too, since the validation is per
platform; `cli.release` builds the CLI on each of those two machines, attaches both to one
registry version and flips it live. The pipelines are Rust in [`.kiln/`](.kiln); the
`.woodpecker/*.yml` files are generated from them and must not be edited by hand. After
changing `.kiln/`, regenerate both committed projections and commit them with the change:

```sh
cargo run --manifest-path .kiln/Cargo.toml -- check
cargo run --manifest-path .kiln/Cargo.toml -- describe > .kiln/lock.json
cargo run --manifest-path .kiln/Cargo.toml -- emit woodpecker
```

kiln and libci are private `ssh://` dependencies, so a local build of `.kiln/` needs an
ssh key for GitHub and `CARGO_NET_GIT_FETCH_WITH_CLI=true`.

Coverage, if you want a look: `scripts/cov.sh` (needs `cargo tarpaulin`).

## Things that will bite you

- **Canonical paths collapse merged directories.** On a system where `/bin` is a symlink to
  `/usr/bin`, `which -a sh` prints `/usr/bin/dash` twice. That is the documented behaviour,
  not a bug in the walk.
- **Tests must not set `PATH`.** The suite runs multi-threaded and a test that mutates the
  environment races every other test. Hand directories to `which_in` instead.
- **The Windows validation is by extension only.** It does not consult ACLs, and a file
  called `foo.exe` that is not a program still counts, which is also what `cmd.exe` does.
- **`--version` is `0.0.<count>` until the repo has a `vMAJOR.MINOR` tag.** See
  [`VERSIONING.md`](VERSIONING.md).
- **Installing the CLI on macOS or Linux puts a second `which` on `PATH`.** Which one a
  shell runs depends on where the installer's bin directory sits in `PATH`. The two agree
  on what they print for a bare name, up to this one resolving symlinks.
- **A release leg attaches, it never finalizes.** A red `finalize` after green legs means
  a target is missing from the draft; re-run the failed leg, then `finalize`.

## Status and known gaps

- The Windows validation has been cross-compiled and its pure parts are unit-tested, but it
  has not been exercised on a Windows machine yet.
- No release has run yet: the repo still has to be enabled in Woodpecker, marked trusted,
  and given its secrets, and the native agents have to carry their labels. Until the first
  `cli.release` is green the install one-liners above find nothing. See
  [`AGENTS.md`](AGENTS.md) for the list.
- No Linux build is released. The lookup exists there natively; adding one is a line in
  `.kiln/src/main.rs`.
- The repo has no `vMAJOR.MINOR` tag yet, so releases version as `0.0.<count>`.

## License

`which.c` is BSD-3-Clause, see its header. Do what you want with the Rust code.

Conventions: [`AGENTS.md`](AGENTS.md) and the [shared standards](https://standards.thmsn.dev).
