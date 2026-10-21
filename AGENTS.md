Standards: archetype=library product=which — rules via the `standards` MCP (https://standards.thmsn.dev). Repo-specific overrides below.

# AGENTS

- **[`README.md`](README.md) is the front door.** Change behaviour it describes (a command,
  the layout, what's generated, what's still a stub) and update it in the same commit. Ran a
  command from it that didn't work? Fix it before reporting back.
- **[`SHORTCUTS.md`](SHORTCUTS.md) before environment recon.** Anything about the *running
  system* rather than the source: check it first. After recon that cost more than a couple
  of commands, append the route you found (endpoint, command, mechanism), never the reading.

## Repo-specific overrides

- **Two crates, one archetype.** `libwhich/` is the library this repo is for. `which/` is a
  bundled CLI over it, held to the [cli-tool](https://standards.thmsn.dev/d/archetypes/cli-tool)
  rules only where they cost nothing: `build.rs` derives its version for `--version`. It is
  not published to the registry, so there is no install manifest, no `cli.release` pipeline
  and no completions. It does not declare a `product_name!`: pulling `libproduct` in for a
  tool this small is not worth the dependency.
- **No CI.** A library that ships no artifact gets none
  ([library](https://standards.thmsn.dev/d/archetypes/library)). The gate still exists; it
  is the commands under Development in [`README.md`](README.md), run locally before a push.
- **Manifest version is a placeholder.** The workspace `version` in `Cargo.toml` is left
  alone; consumers pin by git ref ([versioning](https://standards.thmsn.dev/d/versioning)).
- **No `.cargo/config.toml`.** The library never logs an error as a `valuable` field, so it
  does not need `tracing_unstable`, and it has no private deps.
- **`which.c` is the oracle.** The FreeBSD source at the repo root defines the lookup and
  validation semantics on Unix. A behaviour change is checked against it first; Windows has
  no reference and follows `cmd.exe` (`PATHEXT`).
- **Tests never touch the environment.** `which_in` and `search_paths_from` exist so the
  logic is tested with directories handed in. Do not add a test that sets `PATH`: the suite
  runs multi-threaded and such a test races every other one.
