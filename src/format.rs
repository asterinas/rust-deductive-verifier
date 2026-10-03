use cargo_metadata::MetadataCommand;
use memoize::memoize;
use std::{
    collections::HashSet,
    path::{self, PathBuf},
    process::Command,
};

use crate::{executable, helper::DynError, verus};

const VERUSFMT_MIN_EDITION_VERSION: &str = "0.7.4";
const VERUSFMT_EDITION: &str = "2024";

fn get_verusfmt_path() -> Result<PathBuf, DynError> {
    executable::locate(verus::VERUSFMT_BIN, None, &Vec::<PathBuf>::new()).ok_or(
        "Cannot find the Verusfmt binary, please install it by running `cargo dv bootstrap`".into(),
    )
}

fn print_command_output(stdout: &[u8], stderr: &[u8]) {
    if !stdout.is_empty() {
        eprint!("{}", String::from_utf8_lossy(stdout));
    }
    if !stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(stderr));
    }
}

fn verusfmt_refused_edition_arg(stdout: &[u8], stderr: &[u8]) -> bool {
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    )
    .to_lowercase();

    output.contains("--edition")
        && (output.contains("unexpected")
            || output.contains("unrecognized")
            || output.contains("unknown")
            || output.contains("not expected")
            || output.contains("invalid argument"))
}

/// Format workspace packages by delegating to `cargo verus fmt`, one
/// invocation per package so a failing one does not stop the rest.
pub fn format_packages(targets: &Vec<String>) -> Result<(), DynError> {
    let verusfmt = get_verusfmt_path()?;

    // The given targets, or all workspace members.
    let mut packages: Vec<String> = if targets.is_empty() {
        get_all_targets().into_iter().collect()
    } else {
        targets.clone()
    };
    packages.sort();

    let cargo_verus = verus::get_cargo_verus(true);
    for package in &packages {
        println!("Running `cargo verus fmt -p {}`...", package);

        // verusfmt defaults to edition 2021; this repo is formatted with 2024.
        let output = Command::new(&cargo_verus)
            .arg("fmt")
            .arg("-p")
            .arg(package)
            .env("VERUSFMT", &verusfmt)
            .args(["--", "--edition", VERUSFMT_EDITION])
            .output()?;

        if !output.status.success() {
            if verusfmt_refused_edition_arg(&output.stdout, &output.stderr) {
                eprintln!(
                    "verusfmt failed because it does not support `--edition`. Please update verusfmt to >= {}.",
                    VERUSFMT_MIN_EDITION_VERSION
                );
            } else {
                eprintln!(
                    "Warning: `cargo verus fmt -p {}` failed on some files, skipping them.",
                    package
                );
            }
            print_command_output(&output.stdout, &output.stderr);
        }
    }

    println!("Formatting complete!");
    Ok(())
}

#[memoize]
fn get_all_targets() -> HashSet<String> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get cargo metadata");

    metadata
        .workspace_members
        .into_iter()
        .map(|id| {
            let package_name = &metadata
                .packages
                .iter()
                .find(|pkg| pkg.id == id)
                .expect("Failed to find package")
                .name;
            package_name.to_string()
        })
        .collect()
}

pub fn target_parser(s: &str) -> Result<String, String> {
    let all_targets = get_all_targets();
    let s = s
        .trim_start_matches(".\\")
        .trim_end_matches(path::MAIN_SEPARATOR);
    if all_targets.contains(s) {
        Ok(s.to_string())
    } else {
        Err(format!("Unknown target: {}", s))
    }
}
