# Rust Deductive Verifier

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

## Bootstrapping Verus

By default, `cargo dv bootstrap` builds the `main` branch of
`asterinas/verus`. Use `--branch` to select another branch and repeat
`--build-arg` to pass additional arguments to that branch's `vargo build`.

For example, the `irc11` branch requires its weak-memory vstd modules to be
enabled explicitly:

```bash
cargo dv bootstrap --branch irc11 --build-arg=--vstd-weak-memory
```

The same arguments are honored by upgrades:

```bash
cargo dv bootstrap --upgrade --branch irc11 \
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
