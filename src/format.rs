use cargo_metadata::MetadataCommand;
use memoize::memoize;
use rayon::prelude::*;
use std::{
    collections::HashSet,
    path::{self, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

use crate::{executable, verus};

pub fn format_vostd(targets: &Vec<String>) {
    let verusfmt = executable::locate(verus::VERUSFMT_BIN, None, &Vec::<PathBuf>::new())
        .unwrap_or_else(|| {
            eprintln!(
            "Cannot find the Verusfmt binary, please install it by running `cargo dv bootstrap`"
        );
            std::process::exit(1);
        });

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
            let src_dir = path.parent().unwrap();
            // Just search for all `.rs` files in the src directory instead of chasing them
            WalkDir::new(src_dir)
                .into_iter()
                .filter_map(|entry| {
                    let path = entry.ok()?.path().to_path_buf();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .par_bridge()
                .for_each(|file| {
                    println!("Formatting file: {}", &file.display());
                    let status = Command::new(&verusfmt)
                        .arg(&file)
                        .status()
                        .expect("Failed to run verusfmt");
                    if !status.success() {
                        eprintln!("Failed to format file: {}, skipping", &file.display());
                    }
                });
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
