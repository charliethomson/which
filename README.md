# libwhich

A Rust library and CLI for locating executables on `$PATH`, inspired by the FreeBSD `which(1)` implementation.

## Workspace

- **`libwhich`** - Core library crate
- **`which`** - CLI binary

## Library Usage

```toml
[dependencies]
libwhich = { git = "https://github.com/charliethomson/libwhich" }
```

```rust
use libwhich::which;

// Find one or more binaries by name
let results: Vec<_> = which(&["ls", "cat"]).unwrap().collect();
for path in results {
    println!("{}", path.display());
}
```

`which()` returns an iterator over all matching `PathBuf`s found across every directory in `$PATH`. Results are canonical absolute paths and are validated to be regular, executable files (on unix).

## CLI Usage

```
which [-s] [-a] [-l LIMIT] <names...>
```

| Flag | Description |
|------|-------------|
| `-s` | Silent mode - exit 0 if found, 1 otherwise |
| `-a` | Print all matches, not just the first |
| `-l` | Limit the number of results |

```bash
$ which ls
/bin/ls

$ which -a python3
/usr/local/bin/python3
/usr/bin/python3
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `libc` | yes | Uses libc for accurate permission checks (superuser-aware) |
| `tracing` | no | Enables `tracing` instrumentation |

## Reference

The `which.c` file in the repo root is the original FreeBSD `which(1)` source (BSD-3-Clause) used as a reference for the implementation.

## License

BSD-3-Clause (reference implementation), do what you want with the Rust code.
