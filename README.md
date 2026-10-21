# which

**Find out which file a program name would run, from Rust or from a shell.**

A Rust workspace with two crates: `libwhich`, a library that locates executables on `PATH`
the way FreeBSD's `which(1)` does, and `which`, a small CLI over it. The library is the
point; the CLI is the same lookup with flags. Both work on Unix and Windows.

Docs: [Versioning](VERSIONING.md) · [Shortcuts](SHORTCUTS.md) · [Agents](AGENTS.md) ·
[Standards](https://standards.thmsn.dev)

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
| [`which/`](which) | The CLI. `build.rs` derives its version from git. |
| [`scripts/cov.sh`](scripts/cov.sh) | Optional coverage report via `cargo tarpaulin`. |
| [`which.c`](which.c) | The FreeBSD `which(1)` source (BSD-3-Clause), kept as the reference for the Unix semantics. |

## Install

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
```

| Flag | Effect |
|---|---|
| `-a` | Print every match for each name, not just the first |
| `-l N` | Print at most N matches per name (ignored with `-a`) |
| `-s` | Print nothing; the exit status is the answer |
| `-v` | Debug tracing on stderr (`RUST_LOG` narrows it) |
| `--version` | The derived version, see [`VERSIONING.md`](VERSIONING.md) |

The exit status is 0 when every name was found and 1 otherwise, like `which(1)`.

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

There is no CI; this is a library that ships no artifact. These are the gate, run them
before pushing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p libwhich --no-default-features
```

The Windows path is cross-checked from anywhere with the target installed:

```sh
rustup target add x86_64-pc-windows-gnu
cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

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

## Status and known gaps

- The Windows validation has been cross-compiled and its pure parts are unit-tested, but it
  has not been exercised on a Windows machine yet.
- The CLI is not published to the registry.

## License

`which.c` is BSD-3-Clause, see its header. Do what you want with the Rust code.

Conventions: [`AGENTS.md`](AGENTS.md) and the [shared standards](https://standards.thmsn.dev).
