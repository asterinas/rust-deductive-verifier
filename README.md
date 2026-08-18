# Rust Deductive Verifier

`cargo dv` is a thin project-specific wrapper around `cargo-verus`.
Verification and build commands are delegated to `cargo-verus verify`,
`cargo-verus focus`, and `cargo-verus build`,
while `cargo dv` keeps a few repository conveniences on top,
such as bootstrapping the Verus toolchain, formatting Verus/Rust sources,
generating docs, and running pre-commit checks.

## Deployment

1. Put this repository as a directory `dv` in the root of your Rust project.

```bash
git clone [this-repo] dv
```

2. Add/modify the Cargo configuration file `.cargo/config.toml` in the root of your Rust project:

```toml
[alias]
dv = "run --manifest-path dv/Cargo.toml --bin dv --"
pre-commit = "run --manifest-path dv/Cargo.toml --bin pre_commit --"
```

3. Now, you can use the provided commands with `cargo dv`:

```bash
cargo dv verify --targets <target1> <target2> ...
```

### Cargo features and Verus arguments

The `verify` and `build` commands accept Cargo's standard feature options:
`-F`/`--features`, `--all-features`, and `--no-default-features`. DV forwards
these options to `cargo-verus` before the verifier argument separator. Arguments
after `--` continue to be passed directly to Verus.

For example, this enables the `irc11` Cargo feature while asking Verus to check
only one module:

```bash
cargo dv verify --targets ostd --features irc11 -- \
  --verify-only-module sync::rcu
```

### Line counting

Line counting is a separate command from verification. Count a complete target
using its latest Cargo/Verus dependency information:

```bash
cargo dv count --targets ostd
```

To count one Rust module (including file-based submodules), select exactly one
target and pass the Verus module path:

```bash
cargo dv count --targets ostd --module sync::rwlock
```

The default output is a per-file summary. Add `--print-all` (or `-p`) to print
every annotated source line. Whole-target counting requires dependency
information from a previous `cargo dv verify` or `cargo dv build`; module
counting resolves source files directly and does not require it.

## Bootstrapping Verus

By default, `cargo dv bootstrap` builds the `main` branch of
`asterinas/verus`. Use `--branch` to select another branch and repeat
`--build-arg` to pass additional arguments to `cargo-verus` when it builds
vstd.

The `irc11` branch is hosted by `verus-lang/verus`, rather than the default
`asterinas/verus` remote, and requires its weak-memory vstd modules to be
enabled explicitly:

```bash
cargo dv bootstrap --upstream-verus --branch irc11 \
  --build-arg=--vstd-weak-memory
```

For compatibility with the `irc11` branch's vargo spelling, DV translates
`--vstd-weak-memory` to the vstd feature arguments
`--features weak-memory` used by the current cargo-verus bootstrap path.

The same arguments are honored by upgrades:

```bash
cargo dv bootstrap --upgrade --upstream-verus --branch irc11 \
  --build-arg=--vstd-weak-memory
```

Optionally, if you want to use the pre-commit hook, you can add the rusty-hook to your project:

```bash
cargo install rusty-hook
```

Then, add the following to your `.rusty-hook.toml` file:

```bash
[hooks]
pre-commit = "cargo pre-commit"

[logging]
verbose = true
```
