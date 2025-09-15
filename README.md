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

