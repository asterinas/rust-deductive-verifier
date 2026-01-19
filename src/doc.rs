use crate::verus::{self, DynError, VerusTarget};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

/// Generate documentation for verification targets
pub fn exec_doc(target: &str, verus_conds: bool) -> Result<(), DynError> {
    let target_to_use = verus::find_target(target)?;
    generate_docs(&target_to_use, verus_conds)?;
    Ok(())
}

/// Generate documentation for the target including all its dependencies
fn generate_docs(target: &VerusTarget, verus_conds: bool) -> Result<(), DynError> {
    info!(
        "Generating documentation for {} with all dependencies...",
        target.name
    );

    let root_dir = verus::get_workspace_root();
    let doc_output_dir = root_dir.join("doc");

    std::fs::create_dir_all(&doc_output_dir)?;

    let deps = verus::get_local_dependency(target);

    for (_name, dep_target) in deps.iter() {
        if dep_target.name != target.name {
            generate_single_target_doc(dep_target, verus_conds, &doc_output_dir)?;
        }
    }

    generate_single_target_doc(target, verus_conds, &doc_output_dir)?;

    if verus_conds {
        run_verusdoc_postprocessor()?;
    }

    info!("{}", "Generation Complete!".bold().green(),);

    Ok(())
}

/// Generate documentation for a single target using rustdoc
fn generate_single_target_doc(
    target: &VerusTarget,
    verus_conds: bool,
    doc_output_dir: &Path,
) -> Result<(), DynError> {
    info!(
        "{} {}",
        "Generating docs".bold().blue(),
        target.name.white()
    );

    let verus_target_dir = verus::get_verus_target_dir();
    let target_dir = verus::get_target_dir();
    let mut cmd = Command::new("rustdoc");

    // Set VERUSDOC environment variable based on verus_conds flag
    let verus_doc_value = if verus_conds { "1" } else { "0" };
    cmd.env("VERUSDOC", verus_doc_value);
    cmd.env("RUSTC_BOOTSTRAP", "1");

    // Add extern dependencies for verus_builtin
    let builtin_path = verus_target_dir.join("libverus_builtin.rlib");
    cmd.arg("--extern")
        .arg(format!("verus_builtin={}", builtin_path.display()));

    // Add extern dependencies for verus_builtin_macros
    let builtin_macros_path =
        verus_target_dir.join(format!("verus_builtin_macros{}", verus::DYN_LIB));
    cmd.arg("--extern").arg(format!(
        "verus_builtin_macros={}",
        builtin_macros_path.display()
    ));

    // Add extern dependencies for verus_state_machine_macros
    let state_machine_macros_path =
        verus_target_dir.join(format!("verus_state_machine_macros{}", verus::DYN_LIB));
    cmd.arg("--extern").arg(format!(
        "verus_state_machine_macros={}",
        state_machine_macros_path.display()
    ));

    // Add extern dependencies for vstd
    let vstd_path = verus_target_dir.join("libvstd.rlib");
    cmd.arg("--extern")
        .arg(format!("vstd={}", vstd_path.display()));

    // Add dependencies that this target actually needs
    let deps = verus::get_local_dependency(target);
    for (_name, dep_target) in deps.iter() {
        if dep_target.name != target.name {
            // Check if .rlib file exists for this dependency
            let rlib_path =
                target_dir.join(format!("lib{}.rlib", dep_target.name.replace('-', "_")));
            if rlib_path.exists() {
                let extern_name = dep_target.name.replace('-', "_");
                cmd.arg("--extern")
                    .arg(format!("{}={}", extern_name, rlib_path.display()));
            } else {
                return Err(format!(
                    "Missing compiled dependency '{}' for target '{}'.\n\nPlease run:\n  cargo dv verify --targets {}",
                    dep_target.name, target.name, target.name
                ).into());
            }
        }
    }

    cmd.arg("-L").arg(format!("{}", verus_target_dir.display()));
    cmd.arg("-L").arg(format!("{}", target_dir.display()));
    cmd.arg("--edition=2021")
        .arg("--cfg")
        .arg("verus_keep_ghost")
        .arg("--cfg")
        .arg("verus_keep_ghost_body")
        .arg("--cfg")
        .arg("feature=\"std\"")
        .arg("--cfg")
        .arg("feature=\"alloc\"")
        .arg("-Zcrate-attr=feature(stmt_expr_attributes)")
        .arg("-Zcrate-attr=feature(register_tool)")
        .arg("-Zcrate-attr=register_tool(verus)")
        .arg("-Zcrate-attr=register_tool(verifier)")
        .arg("-Zcrate-attr=register_tool(verusfmt)")
        .arg("-Zcrate-attr=feature(rustc_attrs)")
        .arg("-Zcrate-attr=feature(portable_simd)")
        .arg("-Zcrate-attr=feature(negative_impls)")
        .arg("--enable-index-page")
        .arg("-Zunstable-options");

    // Set crate type and name
    cmd.arg("--crate-type=lib")
        .arg(format!("--crate-name={}", target.name.replace('-', "_")));

    // Set output directory
    cmd.arg("-o").arg(&doc_output_dir);

    // Add the source file
    let source_file = target.root_file();
    cmd.arg(&source_file);

    debug!("Running rustdoc for {}: {:?}", target.name, cmd);

    let status = cmd.status()?;
    if !status.success() {
        warn!(
            "rustdoc failed for target: {}, but continuing...",
            target.name
        );
        return Ok(()); // Continue with other targets instead of failing
    }

    info!(
        "{} {} {}",
        "Generated docs for".bold().green(),
        target.name.white(),
        "successfully".green()
    );

    Ok(())
}

fn run_verusdoc_postprocessor() -> Result<(), DynError> {
    let verusdoc = verus::get_verusdoc();

    info!("Running verusdoc post-processor...");
    let status = Command::new(&verusdoc).status()?;

    if !status.success() {
        warn!("verusdoc post-processor failed");
    } else {
        info!("verusdoc post-processor completed successfully");
    }

    Ok(())
}
