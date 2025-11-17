use cargo_metadata::MetadataCommand;
use memoize::memoize;
use rayon::prelude::*;
use std::{
    collections::HashSet,
    path::{self, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

use crate::{executable, helper::DynError, verus};

fn get_verusfmt_path() -> Result<PathBuf, DynError> {
    executable::locate(verus::VERUSFMT_BIN, None, &Vec::<PathBuf>::new()).ok_or(
        "Cannot find the Verusfmt binary, please install it by running `cargo dv bootstrap`".into(),
    )
}

fn collect_rust_files_from_dir(dir: &std::path::Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            let path = entry.ok()?.path().to_path_buf();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

fn run_formatter_on_files(files: &[PathBuf], formatter: &str, formatter_path: Option<&PathBuf>) {
    println!("Running {} on {} files...", formatter, files.len());
    files.par_iter().for_each(|file| {
        println!("Formatting file: {}", file.display());

        let mut cmd = if let Some(path) = formatter_path {
            Command::new(path)
        } else {
            Command::new(formatter)
        };

        let status = cmd.arg(file).status();

        match status {
            Ok(status) if status.success() => {}
            Ok(_) => {
                if formatter == "rustfmt" {
                    eprintln!("Warning: {} failed for file: {}", formatter, file.display());
                } else {
                    eprintln!("Failed to format file: {}, skipping", file.display());
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not run {} on {}: {}",
                    formatter,
                    file.display(),
                    e
                );
            }
        }
    });
}

fn run_rustfmt_on_files(files: &[PathBuf]) {
    run_formatter_on_files(files, "rustfmt", None);
}

fn run_verusfmt_on_files(files: &[PathBuf], verusfmt: &PathBuf) {
    run_formatter_on_files(files, "verusfmt", Some(verusfmt));
}

pub fn format_vostd(targets: &Vec<String>) {
    let verusfmt = match get_verusfmt_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Read the cargo metadata for all files
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get cargo metadata");

    // Filter packages based on the provided targets
    let fmt_packages = if targets.is_empty() {
        metadata.packages
    } else {
        metadata
            .packages
            .into_iter()
            .filter(|p| targets.contains(&p.name))
            .collect::<Vec<_>>()
    };

    for package in fmt_packages {
        for target in package.targets {
            let path = target.src_path;
            let src_dir = path.parent().unwrap().as_std_path();
            let rust_files = collect_rust_files_from_dir(src_dir);
            run_verusfmt_on_files(&rust_files, &verusfmt);
        }
    }
}

pub fn run_cargo_fmt(targets: &Vec<String>) {
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt");

    if !targets.is_empty() {
        for target in targets {
            cmd.arg("--package").arg(target);
        }
    }

    cmd.status().expect("Failed to run cargo fmt");
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

pub fn format_paths(paths: &[PathBuf]) -> Result<(), DynError> {
    let mut rust_files = Vec::new();

    for path in paths {
        if !path.exists() {
            eprintln!("Warning: Path does not exist: {}", path.display());
            continue;
        }

        if path.is_file() {
            if path.extension().map_or(false, |ext| ext == "rs") {
                rust_files.push(path.clone());
            }
        } else if path.is_dir() {
            rust_files.extend(collect_rust_files_from_dir(path));
        }
    }

    if rust_files.is_empty() {
        println!("No Rust files found in the specified paths.");
        return Ok(());
    }

    run_rustfmt_on_files(&rust_files);

    let verusfmt = get_verusfmt_path()?;
    run_verusfmt_on_files(&rust_files, &verusfmt);

    println!("Formatting complete!");
    Ok(())
}
