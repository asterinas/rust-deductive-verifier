use colored::Colorize;
use indexmap::IndexMap;
use memoize::memoize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use cargo_metadata;
use cargo_metadata::CrateType;

use crate::commands::CargoBuildExterns;
use crate::generator::Generative;
use crate::{
    commands, dep_tree, executable, files, fingerprint, generator, projects, serialization,
};

pub type DynError = Box<dyn std::error::Error>;

/// Verus binary location
///
/// This struct is used to locate the Verus binary and Z3 binary.
/// It uses the `Executable` struct to locate the binaries in the system PATH or in the specified hints.
/// It also provides a method to get the root directory of the project.
///

#[cfg(target_os = "windows")]
pub const VERUS_BIN: &str = "verus.exe";
#[cfg(not(target_os = "windows"))]
pub const VERUS_BIN: &str = "verus";

pub const VERUS_HINT_RELEASE: &str = "tools/verus/source/target-verus/release";
pub const VERUS_HINT: &str = "tools/verus/source/target-verus/debug";
pub const VERUS_EVN: &str = "VERUS_PATH";

#[cfg(target_os = "windows")]
pub const VERUSFMT_BIN: &str = "verusfmt.exe";
#[cfg(not(target_os = "windows"))]
pub const VERUSFMT_BIN: &str = "verusfmt";

#[cfg(target_os = "windows")]
pub const RUST_VERIFY: &str = "rust_verify.exe";
#[cfg(not(target_os = "windows"))]
pub const RUST_VERIFY: &str = "rust_verify";

#[cfg(target_os = "windows")]
pub const Z3_BIN: &str = "z3.exe";
#[cfg(not(target_os = "windows"))]
pub const Z3_BIN: &str = "z3";

#[cfg(target_os = "windows")]
pub const DYN_LIB: &str = ".dll";
#[cfg(target_os = "linux")]
pub const DYN_LIB: &str = ".so";
#[cfg(target_os = "macos")]
pub const DYN_LIB: &str = ".dylib";

pub const Z3_HINT: &str = "tools/verus/source";
pub const Z3_EVN: &str = "VERUS_Z3_PATH";

pub const RUSTDOC_BIN: &str = "rustdoc";
pub const VERUSDOC_BIN: &str = "verusdoc";
pub const VERUSDOC_HINT_RELEASE: &str = "tools/verus/source/target/release";
pub const VERUSDOC_HINT: &str = "tools/verus/source/target/debug";

#[memoize]
pub fn get_verus(release: bool) -> PathBuf {
    executable::locate(
            VERUS_BIN,
            Some(VERUS_EVN),
            if release { &[VERUS_HINT_RELEASE] } else {&[VERUS_HINT]},
        ).unwrap_or_else(|| {
            error!("Cannot find the Verus binary, please set the VERUS_PATH environment variable or add it to your PATH");
        })
}

#[memoize]
pub fn get_rust_verify(release: bool) -> PathBuf {
    executable::locate(
        RUST_VERIFY,
        None,
        if release {
            &[VERUS_HINT_RELEASE]
        } else {
            &[VERUS_HINT]
        },
    )
    .unwrap_or_else(|| {
        error!("Cannot find the Verus `rust_verify` binary.");
    })
}

#[memoize]
pub fn get_z3() -> PathBuf {
    executable::locate(
            Z3_BIN,
            Some(Z3_EVN),
            &[Z3_HINT],
        ).unwrap_or_else(|| {
            error!("Cannot find the Z3 binary, please set the VERUS_Z3_PATH environment variable or add it to your PATH");
        })
}

#[memoize]
pub fn get_rustdoc() -> PathBuf {
    executable::locate(
            RUSTDOC_BIN,
            None,
            &[] as &[&str]
        ).unwrap_or_else(|| {
            error!("Cannot find the rustdoc binary, please install it using `rustup component add rust-docs`");
        })
}

#[memoize]
pub fn get_verusdoc(release: bool) -> PathBuf {
    executable::locate(
            VERUSDOC_BIN,
            None,
            if release { &[VERUSDOC_HINT_RELEASE] } else { &[VERUSDOC_HINT] },
        ).unwrap_or_else(|| {
            error!("Cannot find the verusdoc binary, please install it using `cargo xtask bootstrap verusdoc`");
        })
}

#[memoize]
pub fn get_target_dir() -> PathBuf {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get metadata");
    metadata.target_directory.into_std_path_buf()
}

#[memoize]
pub fn get_verus_target_dir() -> PathBuf {
    let verus_dir = install::verus_dir();
    verus_dir
        .join("source")
        .join("target-verus")
        .join("release")
}

#[cfg(target_os = "windows")]
#[memoize]
pub fn system_crates() -> HashSet<&'static str> {
    HashSet::from([
        "build-script-build",
        "borsh",
        "vstd",
        "verus_builtin",
        "verus_builtin_macros",
        "verus_state_machines_macros",
    ])
}

#[cfg(target_os = "linux")]
#[memoize]
pub fn system_crates() -> HashSet<&'static str> {
    HashSet::from([
        "build-script-build",
        "borsh",
        "vstd",
        "verus_builtin",
        "verus_builtin_macros",
        "verus_state_machines_macros",
    ])
}

#[cfg(target_os = "macos")]
#[memoize]
pub fn system_crates() -> HashSet<&'static str> {
    HashSet::from([
        "build-script-build",
        "borsh",
        "vstd",
        "verus_builtin",
        "verus_builtin_macros",
        "verus_state_machines_macros",
    ])
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VerusDependency {
    // target name of the dependency
    pub name: String,
    // path to the dependency, only if the dependency is a local path
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VerusTarget {
    /// name of the package
    pub name: String,
    /// version of the package
    pub version: String,
    /// directory of the package
    pub dir: PathBuf,
    /// crate root file of the package
    pub file: PathBuf,
    /// crate type of the primary target of the package
    pub crate_type: CrateType,
    /// dependencies of the package
    pub dependencies: Vec<VerusDependency>,
    /// whether or not generate lifetime
    pub gen_lifetime: bool,
    /// runtime, has been rebuilt this session
    pub rebuilt: bool,
    /// carrying `default` features for this target
    pub features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExtraOptions {
    /// if log is enabled
    pub log: bool,
    /// if trace is enabled
    pub trace: bool,
    /// if release debug version
    pub release: bool,
    /// max number of errors before stopping
    pub max_errors: usize,
    /// needs to disassemble the output
    pub disasm: bool,
    /// pass-through options to the verifier
    pub pass_through: Vec<String>,
    /// count lines of code
    pub count_line: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocOptions {}

impl VerusTarget {
    pub fn root_file(&self) -> PathBuf {
        self.file.clone()
    }

    pub fn crate_type(&self) -> CrateType {
        self.crate_type.clone()
    }

    pub fn fingerprint(&self) -> String {
        let content = fingerprint::fingerprint_dir(&self.dir);
        fingerprint::fingerprint_as_str(&content)
    }

    pub fn fingerprint_recursive(&self, all_targets: &HashMap<String, VerusTarget>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let content = fingerprint::fingerprint_dir(&self.dir);
        fingerprint::fingerprint_as_str(&content).hash(&mut hasher);
        for dep in &self.dependencies {
            if let Some(dep_target) = all_targets.get(&dep.name) {
                dep_target
                    .fingerprint_recursive(all_targets)
                    .hash(&mut hasher);
            }
        }
        hasher.finish().to_string()
    }

    pub fn is_fresh(&self, all_targets: &HashMap<String, VerusTarget>) -> bool {
        let ts = self.library_proof_timestamp();
        if !ts.exists() {
            return false;
        }

        // Check if our own verification file exists
        if !self.library_proof().exists() {
            return false;
        }

        // Get our own timestamp
        let self_timestamp = match std::fs::metadata(&self.library_proof()) {
            Ok(metadata) => metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
            Err(_) => return false,
        };

        // Check if all recursive dependencies have their verification files and are not newer than us
        let deps = get_local_dependency_recursive(self);
        for dep in deps.values() {
            if !dep.library_proof().exists() {
                return false;
            }

            // Check if dependency is newer than us
            if let Ok(dep_metadata) = std::fs::metadata(&dep.library_proof()) {
                if let Ok(dep_timestamp) = dep_metadata.modified() {
                    if dep_timestamp > self_timestamp {
                        return false; // Dependency is newer, we need to rebuild
                    }
                }
            }
        }

        let ts_hash = self.load_library_proof_timestamp();
        let cur_hash = self.fingerprint_recursive(all_targets);
        if cur_hash == ts_hash {
            return true;
        }
        false
    }

    pub fn library_prefix(&self) -> String {
        match self.crate_type {
            CrateType::Bin => "",
            CrateType::Lib => "lib",
            _ => {
                fatal!("Unknown crate type {}", self.crate_type)
            }
        }
        .to_string()
    }

    pub fn library_suffix(&self) -> String {
        match self.crate_type {
            CrateType::Bin => "",
            CrateType::Lib => "rlib",
            _ => {
                fatal!("Unknown crate type {}", self.crate_type)
            }
        }
        .to_string()
    }

    pub fn library_proof(&self) -> PathBuf {
        get_target_dir()
            .join(format!("{}.verusdata", self.name))
            .to_path_buf()
    }

    pub fn library_proof_timestamp(&self) -> PathBuf {
        get_target_dir()
            .join(format!("{}.verusdata.timestamp", self.name))
            .to_path_buf()
    }

    pub fn load_library_proof_timestamp(&self) -> String {
        let content = File::open(self.library_proof_timestamp())
            .map(|mut f| {
                let mut content = Vec::<u8>::new();
                f.read_to_end(&mut content).unwrap_or_else(|e| {
                    warn!("Failed to read library proof timestamp: {}", e);
                    0
                });
                content
            })
            .unwrap_or_else(|e| {
                warn!("Failed to open library proof timestamp: {}", e);
                vec![]
            });
        String::from_utf8_lossy(&content).to_string()
    }

    pub fn save_library_proof_timestamp(&self, all_targets: &HashMap<String, VerusTarget>) {
        let content = self.fingerprint_recursive(all_targets);
        files::touch(self.library_proof_timestamp().to_string_lossy().as_ref());
        let mut file = File::create(self.library_proof_timestamp()).unwrap_or_else(|e| {
            error!("Failed to create library proof timestamp: {}", e);
        });
        file.write_all(content.as_bytes()).unwrap_or_else(|e| {
            error!("Failed to write library proof timestamp: {}", e);
        });
    }

    pub fn library_path(&self) -> PathBuf {
        let lib = format!(
            "{}{}.{}",
            self.library_prefix(),
            self.name,
            self.library_suffix()
        );
        get_target_dir().join(lib).to_path_buf()
    }
}

fn extract_dependencies(package: &cargo_metadata::Package) -> Vec<VerusDependency> {
    let mut deps = Vec::new();
    for dep in package.dependencies.iter() {
        let name: String = match dep.rename {
            Some(ref rename) => rename.replace('-', "_"),
            None => dep.name.replace('-', "_"),
        };
        let path = dep.path.as_ref().map(|utf| Path::new(&utf).to_path_buf());
        deps.push(VerusDependency { name, path });
    }
    deps
}

fn extract_features(
    package: &cargo_metadata::Package,
    workspace_features: &[String],
) -> Vec<String> {
    let mut features: HashSet<String> = HashSet::new();
    features.extend(workspace_features.iter().map(|s| s.to_string()));

    // level-traverse of the feature tree
    let mut q = vec!["default"];
    q.extend(workspace_features.iter().map(|s| s.as_str()));

    while let Some(feat) = q.pop() {
        if let Some(f) = package.features.get(feat) {
            for f in f.iter() {
                if !features.contains(f) {
                    features.insert(f.clone());
                    q.push(f);
                }
            }
        }
    }
    features.into_iter().collect()
}

pub fn workspace_features(name: &str, metadata: &cargo_metadata::Metadata) -> Vec<String> {
    metadata
        .workspace_metadata
        .get(name)
        .and_then(|v| v.get("features"))
        .and_then(|v| v.as_array())
        .map(|features_array| {
            features_array
                .iter()
                .filter_map(|feature| feature.as_str())
                .map(|feature_str| feature_str.to_string())
                .collect()
        })
        .unwrap_or_else(Vec::new)
}

#[memoize]
pub fn verus_targets() -> HashMap<String, VerusTarget> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .unwrap_or_else(|e| {
            error!("Failed to get metadata: {:?}", e);
        });

    let workspace: HashSet<String> = metadata
        .workspace_members
        .iter()
        .map(|id| id.to_string())
        .collect();

    let mut targets: HashMap<String, VerusTarget> = HashMap::new();
    for package in metadata.packages.iter() {
        if !workspace.contains(package.id.to_string().as_str())
            || !package.features.contains_key("verify")
        {
            // Not a valid verus target
            continue;
        }

        // check if features[verify] has "verus"
        let has_verus = package
            .features
            .get("verify")
            .map(|verifier| verifier.contains(&"verus".to_string()))
            .unwrap_or(false);
        if !has_verus {
            // Not a valid verus target
            continue;
        }

        let target_file = package
            .metadata
            .get("verus")
            .and_then(|v| v.get("path"))
            .and_then(|v| v.as_str());

        // check if the package has a target
        if let Some(target) = package.targets.first() {
            let name = package.name.as_str().to_string();
            let version = package.version.to_string();
            let dir = Path::new(&package.manifest_path)
                .parent()
                .unwrap()
                .to_path_buf();
            let crate_type = if target.crate_types.contains(&CrateType::Bin) {
                CrateType::Bin
            } else {
                CrateType::Lib
            };
            let file = dir.clone().join(target_file.unwrap_or(match crate_type {
                CrateType::Bin => "src/main.rs",
                _ => "src/lib.rs",
            }));

            let deps = extract_dependencies(package);

            let gen_lifetime = package
                .metadata
                .get("verus")
                .and_then(|v| v.get("check_lifetime"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let ws_features = workspace_features(&name, &metadata);
            let features = extract_features(package, ws_features.as_slice());

            targets.insert(
                name.clone(),
                VerusTarget {
                    name,
                    version,
                    dir,
                    file,
                    crate_type,
                    dependencies: deps,
                    gen_lifetime,
                    rebuilt: false,
                    features,
                },
            );
        } else {
            // No valid target
            continue;
        }
    }
    targets
}

pub fn find_target(t: &str) -> Result<VerusTarget, String> {
    let all = verus_targets();
    let s = files::dir_as_package(t);

    let target = all.get(&s).cloned().unwrap_or_else(|| {
        error!(
            "Cannot find target {}\n\n  Targets available:\n{}",
            t,
            all.keys()
                .fold(String::new(), |acc, k| { acc + "\n - " + k })
        );
    });
    Ok(target)
}

pub fn get_local_dependency(target: &VerusTarget) -> IndexMap<String, VerusTarget> {
    let all = verus_targets();
    let mut deps = IndexMap::new();

    for dep in target.dependencies.iter() {
        if system_crates().contains(dep.name.as_str()) {
            // Skip system crates
            continue;
        }
        if dep.path.is_none() {
            // Not a local path dependency
            continue;
        }
        if !all.contains_key(dep.name.as_str()) {
            // Not in current workspace
            continue;
        }
        let dep_target = all.get(dep.name.as_str()).unwrap();
        deps.insert(dep.name.clone(), dep_target.clone());
    }

    deps
}

pub fn get_local_dependency_recursive(target: &VerusTarget) -> IndexMap<String, VerusTarget> {
    let mut result = IndexMap::new();
    let mut visited = std::collections::HashSet::new();

    fn collect_deps_recursive(
        target: &VerusTarget,
        result: &mut IndexMap<String, VerusTarget>,
        visited: &mut std::collections::HashSet<String>,
        _is_root: bool,
    ) {
        let target_name = target.name.replace('-', "_");

        // Prevent infinite recursion
        if visited.contains(&target_name) {
            return;
        }
        visited.insert(target_name.clone());

        // Get direct dependencies
        let direct_deps = get_local_dependency(target);

        // Add direct dependencies to result (unless it's the root target)
        for (dep_name, dep_target) in direct_deps.iter() {
            let dep_key = dep_name.replace('-', "_");
            if !result.contains_key(&dep_key) {
                result.insert(dep_key, dep_target.clone());
            }
            // Recursively collect dependencies of this dependency
            collect_deps_recursive(dep_target, result, visited, false);
        }
    }

    collect_deps_recursive(target, &mut result, &mut visited, true);
    result
}

pub fn get_dependent_targets(target: &VerusTarget, release: bool) -> IndexMap<String, VerusTarget> {
    let mut deps = get_local_dependency_recursive(target);
    let order = resolve_deps_cached(target, release).full_externs;
    deps.sort_by(|a, _, b, _| {
        let x = order.get_index_of(a).unwrap_or(usize::MAX);
        let y = order.get_index_of(b).unwrap_or(usize::MAX);
        x.cmp(&y)
    });
    deps
}

pub fn get_dependent_targets_batch(
    targets: &[VerusTarget],
    release: bool,
) -> IndexMap<String, VerusTarget> {
    let mut deps = IndexMap::new();
    for target in targets.iter() {
        deps.extend(get_local_dependency_recursive(target));
    }
    let order = resolve_deps_cached(targets.first().unwrap(), release).full_externs;
    deps.sort_by(|a, _, b, _| {
        let x = order.get_index_of(a).unwrap_or(usize::MAX);
        let y = order.get_index_of(b).unwrap_or(usize::MAX);
        x.cmp(&y)
    });
    deps
}

pub fn get_remote_dependency(target: &VerusTarget, release: bool) -> IndexMap<String, String> {
    let externs = resolve_deps_cached(target, release).renamed_full_externs();

    let mut deps = IndexMap::new();

    let local_verus = verus_targets()
        .values()
        .map(|t| t.name.replace('-', "_"))
        .collect::<HashSet<_>>();

    for (name, path) in externs.iter() {
        if system_crates().contains(name.as_str()) {
            // Skip system crates
            continue;
        }

        if local_verus.contains(name) {
            // Skip local verus dependencies
            continue;
        }
        deps.insert(name.clone(), path.clone());
    }

    deps
}

pub fn cmd_push_import(cmd: &mut Command, imports: &[&VerusTarget]) {
    for imp in imports.iter() {
        cmd.arg("--import")
            .arg(format!("{}={}", imp.name, imp.library_proof().display()));
        cmd.arg("--extern")
            .arg(format!("{}={}", imp.name, imp.library_path().display()));
    }
}

pub fn check_import(imports: &[&VerusTarget]) -> Result<(), DynError> {
    for imp in imports.iter() {
        if !imp.library_proof().exists() {
            return Err(format!(
                "Cannot find the proof file at `{}` for `{}`",
                imp.library_proof().display(),
                imp.name
            )
            .into());
        }
        if !imp.library_path().exists() {
            return Err(format!(
                "Cannot find the library file at `{}` for `{}`",
                imp.library_path().display(),
                imp.name
            )
            .into());
        }
    }
    Ok(())
}

pub fn check_import_with_auto_compile(
    imports: &[&VerusTarget],
    options: &ExtraOptions,
) -> Result<(), DynError> {
    let mut missing_targets = Vec::new();

    for imp in imports.iter() {
        if !imp.library_proof().exists() || !imp.library_path().exists() {
            missing_targets.push((*imp).clone());
        }
    }

    if !missing_targets.is_empty() {
        info!(
            "Missing verification files for dependencies: {:?}",
            missing_targets.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        info!("Automatically compiling missing dependencies...");

        // Get all dependencies of missing targets to ensure proper compilation order
        let extended_targets = get_dependent_targets_batch(&missing_targets, options.release);
        for target in extended_targets.values() {
            if !target.library_proof().exists() {
                compile_target(target, &vec![], options).map_err(|e| {
                    format!("Failed to auto-compile dependency `{}`: {}", target.name, e)
                })?;
            }
        }

        // Compile the missing targets themselves
        for target in &missing_targets {
            if !target.library_proof().exists() {
                compile_target(target, &vec![], options).map_err(|e| {
                    format!("Failed to auto-compile dependency `{}`: {}", target.name, e)
                })?;
            }
        }
    }

    // Final check
    check_import(imports)
}

pub fn check_externs(externs: &IndexMap<String, String>) -> Result<(), DynError> {
    for (name, path) in externs.iter() {
        if !Path::new(path).exists() {
            return Err(format!(
                "Cannot find the external library file at `{}` for `{}`",
                path, name
            )
            .into());
        }
    }
    Ok(())
}

pub fn cmd_push_externs(cmd: &mut Command, externs: &IndexMap<String, String>) {
    for (name, path) in externs.iter() {
        cmd.arg("--extern").arg(format!("{}={}", name, path));
    }
}

pub fn reorder_deps(target: &VerusTarget, deps: &mut CargoBuildExterns) {
    let raw = dep_tree::cargo_tree(&target.name);
    let tree = dep_tree::CargoTree::parse(&raw);
    let rank = tree.rank();
    let rk = |x: &String| -> usize { *rank.get(x).unwrap_or(&usize::MAX) };

    deps.last_level.sort_by(|a, _, b, _| rk(a).cmp(&rk(b)));

    deps.libraries.sort_by(|_, a, _, b| {
        let a = a.name.replace('-', "_");
        let b = b.name.replace('-', "_");
        rk(&a).cmp(&rk(&b))
    })
}

pub fn resolve_deps(target: &VerusTarget, release: bool) -> CargoBuildExterns {
    let dummy_rs = target.dir.join("src").join(".dummy.rs");
    files::touch(&dummy_rs.to_string_lossy());

    let mut externs = commands::cargo_build_resolve_deps(&target.name, &HashMap::new(), release);

    if externs.deps_ready {
        reorder_deps(target, &mut externs);
        return externs;
    }
    warn!("Unable to resolve dependencies for `{}`", target.name);
    CargoBuildExterns::default()
}

pub fn resolve_deps_cached(target: &VerusTarget, release: bool) -> serialization::Dependencies {
    let deps_path = get_target_dir().join(format!("{}.deps.toml", target.name));
    let cargo_toml = target.dir.join("Cargo.toml");
    if deps_path.exists() && files::newer(&deps_path, &cargo_toml) {
        // cache is up to date, read it directly
        let deps: serialization::Dependencies = serialization::deserialize(&deps_path);
        deps
    } else {
        // rebuild cache
        let externs = resolve_deps(target, release);
        let deps: serialization::Dependencies = externs.into();
        serialization::serialize(&deps_path, &deps);
        deps
    }
}

pub fn gen_extern_crates(target: &VerusTarget, release: bool) {
    let externs = resolve_deps_cached(target, release);
    let mut tmpl = generator::ExternCratesTemplate { crates: Vec::new() };

    let local_deps = target
        .dependencies
        .iter()
        .filter(|dep| dep.path.is_some())
        .map(|dep| dep.name.replace('-', "_"))
        .collect::<HashSet<_>>();

    for name in externs.full_externs.keys() {
        if system_crates().contains(name.as_str()) {
            // Skip system crates
            continue;
        }

        if local_deps.contains(name) {
            // Skip local dependencies
            continue;
        }

        tmpl.crates.push(generator::CrateInfo {
            name: name.clone(),
            alias: None,
        });
    }

    let deps_path = get_target_dir().join(format!("{}.deps.toml", target.name));
    let deps_time = files::modify_time(deps_path);
    let crates_path = get_target_dir().join(format!("{}.extern_crates.rs", target.name));

    tmpl.save_if(&crates_path, &deps_time);
}

pub fn prepare(target: &VerusTarget, release: bool) {
    gen_extern_crates(target, release);
}

fn get_build_dir(release: bool) -> &'static str {
    if release {
        "release"
    } else {
        "debug"
    }
}

pub fn compile_target(
    target: &VerusTarget,
    imports: &[VerusTarget],
    options: &ExtraOptions,
) -> Result<(), DynError> {
    let ts_start = Instant::now();

    let verus = get_verus(options.release);
    let z3 = get_z3();
    let extra_imports = imports
        .iter()
        .map(|target| (target.name.clone(), target.clone()))
        .collect::<IndexMap<_, _>>();

    let out_dir = get_target_dir();
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| {
            error!("Error creating target directory: {}", e);
        });
    }
    let deps_dir = out_dir.join(get_build_dir(options.release)).join("deps");

    prepare(target, options.release);

    let mut deps = get_local_dependency_recursive(target);
    let dep_rebuilt = deps.values().into_iter().any(|t| t.rebuilt == true);

    if !dep_rebuilt && target.is_fresh(&verus_targets()) {
        info!(
            "[Fresh] `{}` is up to date, skipping verification",
            target.name
        );
        return Ok(());
    }

    let cmd = &mut Command::new(&verus);

    // setup the environment
    cmd.env("VERUS_PATH", &verus).env("VERUS_Z3_PATH", &z3);
    cmd.args([
        "-L",
        &format!("dependency={}", deps_dir.display()),
        "-L",
        &get_verus_target_dir().display().to_string(),
    ]);

    if !target.gen_lifetime {
        cmd.arg("--no-lifetime");
    }

    // output options
    cmd.arg("--compile")
        .arg("--export")
        .arg(target.library_proof());

    // imported dependencies
    deps.extend(extra_imports.clone());
    let all_imports = deps.values().collect::<Vec<_>>();
    check_import_with_auto_compile(all_imports.as_slice(), options).unwrap_or_else(|e| {
        error!("Error during verification: {}", e);
    });
    cmd_push_import(cmd, all_imports.as_slice());

    // import external crates
    let externs = get_remote_dependency(target, options.release);
    check_externs(&externs).unwrap_or_else(|e| {
        error!("Error during verification: {}", e);
    });
    cmd_push_externs(cmd, &externs);

    // extra options
    if options.log {
        cmd.arg("--log-all");
    }

    if options.trace {
        cmd.env("RUST_BACKTRACE", "full");
        cmd.arg("--trace");
    }

    if options.release {
        cmd.args(["-C", "opt-level=2"]);
    } else {
        cmd.args(["-C", "opt-level=0"]);
    }

    // input file
    let target_file = target.root_file();
    let crate_type = target.crate_type();
    cmd.arg(target_file)
        .arg(format!("--crate-type={}", crate_type))
        .arg("--expand-errors")
        .arg(format!("--multiple-errors={}", options.max_errors))
        .arg("-o")
        .arg(target.library_path())
        .arg("-V")
        .arg("use-crate-name")
        .args(&options.pass_through)
        .arg("--")
        .arg("-C")
        .arg(format!("metadata={}", target.name));

    for feature in target.features.iter() {
        cmd.args(["--cfg", &format!("feature=\"{}\"", feature)]);
    }
    cmd.stdout(Stdio::inherit());

    info!(
        "  {} {} {}",
        "Verifying (and compiling)".bold().green(),
        target.name.white(),
        target.version.white()
    );
    debug!(">> {:?}", cmd);

    // run the command
    let status = cmd.status().unwrap_or_else(|e| {
        error!("Error during compilation: {}", e);
    });

    if status.success() {
        // duration
        let duration = ts_start.elapsed();
        info!(
            "  {} {} {} in {:.2}s",
            "Verified".bold().green(),
            target.name.white(),
            target.version.white(),
            duration.as_secs_f64()
        );

        // success
        target.save_library_proof_timestamp(&verus_targets());

        // disassemble the output
        if options.disasm {
            disassemble(target).unwrap_or_else(|e| {
                error!("Error during disassembly: {}", e);
            });
        }

        return Ok(());
    }

    // failure
    Err(format!("Error during compilation: `{}`", target.name,).into())
}

pub fn exec_verify(
    targets: &[VerusTarget],
    imports: &[VerusTarget],
    options: &ExtraOptions,
) -> Result<(), DynError> {
    let verus = get_verus(options.release);
    let z3 = get_z3();
    let extra_imports = imports
        .iter()
        .map(|target| (target.name.clone(), target.clone()))
        .collect::<IndexMap<_, _>>();
    let out_dir = get_target_dir();
    let deps_dir = out_dir.join(get_build_dir(options.release)).join("deps");

    let extended_targets = get_dependent_targets_batch(targets, options.release);
    for target in extended_targets.values() {
        compile_target(target, &vec![], options).unwrap_or_else(|e| {
            error!(
                "Unable to build the dependent proof: `{}` ({})",
                target.name, e
            );
        });
    }

    let ts_start = Instant::now();
    // remove the targets that has been compiled
    let remaining_targets = targets
        .iter()
        .filter(|target| {
            let name = target.name.replace('-', "_");
            !extended_targets.contains_key(&name)
        })
        .collect::<Vec<_>>();

    for target in remaining_targets.iter() {
        prepare(target, options.release);

        let cmd = &mut Command::new(&verus);

        // setup the environment
        cmd.env("VERUS_PATH", &verus).env("VERUS_Z3_PATH", &z3);

        cmd.args([
            "-L",
            &format!("dependency={}", deps_dir.display()),
            "-L",
            &get_verus_target_dir().display().to_string(),
        ]);

        if !target.gen_lifetime {
            cmd.arg("--no-lifetime");
        }

        // imported dependencies
        let deps = &mut get_local_dependency_recursive(target);
        deps.extend(extra_imports.clone());
        let all_imports = deps.values().collect::<Vec<_>>();

        check_import_with_auto_compile(all_imports.as_slice(), options).unwrap_or_else(|e| {
            error!("Error during verification: {}", e);
        });
        cmd_push_import(cmd, all_imports.as_slice());

        // import external crates
        let externs = get_remote_dependency(target, options.release);
        check_externs(&externs).unwrap_or_else(|e| {
            error!("Error during verification: {}", e);
        });
        cmd_push_externs(cmd, &externs);

        // extra options
        if options.log {
            cmd.arg("--log-all");
        }
        if options.trace {
            cmd.env("RUST_BACKTRACE", "full");
            cmd.arg("--trace");
        }
        if options.count_line {
            cmd.arg("--emit=dep-info");
        }

        // input file
        let target_file = target.root_file();
        let crate_type = target.crate_type();
        cmd.arg(target_file)
            .arg(format!("--crate-type={}", crate_type))
            .arg("--expand-errors")
            .arg(format!("--multiple-errors={}", options.max_errors))
            .args(&options.pass_through)
            .arg("--")
            .arg("-C")
            .arg(format!("metadata={}", target.name));

        for feature in target.features.iter() {
            cmd.args(["--cfg", &format!("feature=\"{}\"", feature)]);
        }
        cmd.stdout(Stdio::inherit());

        info!(
            "  {} {} {}",
            "Verifying".bold().green(),
            target.name.white(),
            target.version.white()
        );
        debug!(">> {:?}", cmd);

        // run the command
        let status = cmd.status().unwrap_or_else(|e| {
            error!("Error during verification: {}", e);
        });

        if status.success() {
            // duration
            let duration = ts_start.elapsed();
            info!(
                "  {} {} {} in {:.2}s",
                "Verified".bold().green(),
                target.name.white(),
                target.version.white(),
                duration.as_secs_f64()
            );
        }

        if options.count_line {
            let verus_root = install::verus_dir();
            let line_count_dir = verus_root.join("source/tools/line_count");
            let dependency_file = env::current_dir()?.join("lib.d");
            env::set_current_dir(&line_count_dir)?;
            let mut cargo_cmd = Command::new("cargo");
            cargo_cmd
                .arg("run")
                .arg("--release")
                .arg(&dependency_file)
                .arg("-p");

            println!("Counting lines for target: {}", target.name);
            cargo_cmd.status()?;
            fs::remove_file(&dependency_file)?;
        }
    }
    Ok(())
}

pub fn disassemble(target: &VerusTarget) -> Result<(), DynError> {
    let objdump = commands::get_objdump();
    let cmd = &mut Command::new(&objdump);
    let mut status = cmd
        .arg("-d")
        .arg(target.library_path())
        .stdout(Stdio::piped())
        .spawn()?;

    let out = status.stdout.take().unwrap_or_else(|| {
        error!("Error during disassembly: {:?}", cmd);
    });

    let mut rustfilt = Command::new(commands::get_rustfilt());
    let mut status = rustfilt
        .stdin(Stdio::from(out))
        .stdout(Stdio::piped())
        .spawn()?;

    let mut disasm = File::create(format!("{}.S", target.library_path().display()))?;

    let mut out = status.stdout.take().unwrap_or_else(|| {
        error!("Error during disassembly: {:?}", rustfilt);
    });

    let mut content = Vec::<u8>::new();
    out.read_to_end(&mut content)?;
    disasm.write_all(&content)?;
    disasm.flush()?;
    Ok(())
}

pub fn exec_compile(
    targets: &[VerusTarget],
    imports: &[VerusTarget],
    options: &ExtraOptions,
) -> Result<(), DynError> {
    let extended_targets = get_dependent_targets_batch(targets, options.release);
    for target in extended_targets.values() {
        compile_target(target, &[], options).unwrap_or_else(|e| {
            error!(
                "Unable to build the dependent proof: `{}` ({})",
                target.name, e
            );
        });
    }

    // remove the targets that has been compiled
    let remaining_targets = targets
        .iter()
        .filter(|target| {
            let name = target.name.replace('-', "_");
            !extended_targets.contains_key(&name)
        })
        .collect::<Vec<_>>();

    for target in remaining_targets.iter() {
        compile_target(target, imports, options)?;
    }

    Ok(())
}

/// Clean build artefacts produced by `exec_compile`.
pub fn exec_clean(targets: &[VerusTarget], all: bool) -> Result<(), DynError> {
    let out_dir = get_target_dir();

    let to_clean: Vec<VerusTarget> = if all || targets.is_empty() {
        // clean all known targets
        verus_targets().values().cloned().collect()
    } else {
        targets.iter().cloned().collect()
    };

    for target in to_clean.iter() {
        // remove .verusdata
        let proof = target.library_proof();
        if proof.exists() {
            info!("Removing {}", proof.display());
            std::fs::remove_file(&proof).unwrap_or_else(|e| {
                warn!("Failed to remove {}: {}", proof.display(), e);
            });
        }

        // remove .verusdata.timestamp
        let proof_ts = target.library_proof_timestamp();
        if proof_ts.exists() {
            info!("Removing {}", proof_ts.display());
            std::fs::remove_file(&proof_ts).unwrap_or_else(|e| {
                warn!("Failed to remove {}: {}", proof_ts.display(), e);
            });
        }

        // remove lib{name}.rlib
        let lib = target.library_path();
        if lib.exists() {
            info!("Removing {}", lib.display());
            std::fs::remove_file(&lib).unwrap_or_else(|e| {
                warn!("Failed to remove {}: {}", lib.display(), e);
            });
        }

        // remove generated extern_crates
        let extern_crates_path = out_dir.join(format!("{}.extern_crates.rs", target.name));
        if extern_crates_path.exists() {
            info!("Removing {}", extern_crates_path.display());
            std::fs::remove_file(&extern_crates_path).unwrap_or_else(|e| {
                warn!("Failed to remove {}: {}", extern_crates_path.display(), e);
            });
        }

        // remove deps.toml
        let deps_toml_path = out_dir.join(format!("{}.deps.toml", target.name));
        if deps_toml_path.exists() {
            info!("Removing {}", deps_toml_path.display());
            std::fs::remove_file(&deps_toml_path).unwrap_or_else(|e| {
                warn!("Failed to remove {}: {}", deps_toml_path.display(), e);
            });
        }
    }

    Ok(())
}

pub mod install {
    use super::*;
    use crate::toolchain;
    use git2::Repository;

    pub struct VerusInstallOpts {
        pub restart: bool,
        pub release: bool,
        pub branch: Option<String>,
        pub force_reset: bool,
    }

    pub const VERUS_REPO_HTTPS: &str = "https://github.com/asterinas/verus.git";
    pub const VERUS_REPO_SSH: &str = "git@github.com:asterinas/verus.git";

    #[memoize]
    pub fn tools_dir() -> PathBuf {
        projects::get_root().join("tools")
    }

    #[memoize]
    pub fn verus_dir() -> PathBuf {
        tools_dir().join("verus")
    }

    #[memoize]
    pub fn verus_source_dir() -> PathBuf {
        verus_dir().join("source")
    }

    #[memoize]
    pub fn tools_patch_dir() -> PathBuf {
        tools_dir().join("patches")
    }

    pub fn clone_repo(verus_dir: &Path) -> Result<(), DynError> {
        info!("Cloning Verus repo to {} ...", verus_dir.display());

        let mut builder = git2::build::RepoBuilder::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);
        builder.fetch_options(fetch_opts);

        let ssh_result = builder.clone(VERUS_REPO_SSH, verus_dir);
        if ssh_result.is_ok() {
            return Ok(());
        }

        Repository::clone(VERUS_REPO_HTTPS, verus_dir)
            .map_err(|e| format!("Failed to clone verus repo: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn install_z3() -> Result<(), DynError> {
        let z3 = verus_source_dir().join("z3.exe");
        if !z3.exists() {
            info!("Z3 not found, downloading...");
            let mut cmd = executable::get_powershell_command()?;
            cmd.current_dir(verus_source_dir())
                .arg("/c")
                .arg(".\\tools\\get-z3.ps1")
                .status()
                .unwrap_or_else(|e| {
                    error!("Failed to download z3: {}", e);
                });
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn install_z3() -> Result<(), DynError> {
        let z3 = verus_source_dir().join("z3");
        if !z3.exists() {
            info!("Z3 not found, downloading...");
            Command::new("bash")
                .current_dir(verus_source_dir())
                .arg("-c")
                .arg("./tools/get-z3.sh")
                .status()
                .unwrap_or_else(|e| {
                    error!("Failed to download z3: {}", e);
                });
        }
        Ok(())
    }

    fn is_verusfmt_installed() -> bool {
        let output = Command::new("verusfmt").arg("--version").output();
        match output {
            Ok(output) => {
                if output.status.success() {
                    return true;
                }
            }
            Err(_) => {}
        }
        false
    }

    fn install_verusfmt() -> Result<(), DynError> {
        println!("Start to install verusfmt");
        let status = {
            #[cfg(target_os = "windows")]
            {
                // pwsh -ExecutionPolicy Bypass -c "irm https://github.com/verus-lang/verusfmt/releases/latest/download/verusfmt-installer.ps1 | iex"
                let mut cmd = executable::get_powershell_command()?;
                cmd
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-c")
                .arg("irm https://github.com/verus-lang/verusfmt/releases/latest/download/verusfmt-installer.ps1 | iex");
                println!("{:?}", cmd);
                cmd.status()
            }
            #[cfg(not(target_os = "windows"))]
            {
                // curl --proto '=https' --tlsv1.2 -LsSf https://github.com/verus-lang/verusfmt/releases/latest/download/verusfmt-installer.sh | sh
                let mut cmd = Command::new("bash");
                cmd
                .arg("-c")
                .arg("curl --proto '=https' --tlsv1.2 -LsSf https://github.com/verus-lang/verusfmt/releases/latest/download/verusfmt-installer.sh | sh");
                println!("{:?}", cmd);
                cmd.status()
            }
        };
        if let Err(err) = status {
            eprintln!("Failed to install verusfmt {:?}", err);
            return Err(err.into());
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn build_verus(release: bool) -> Result<(), DynError> {
        let mut cmd = executable::get_powershell_command()?;
        cmd.current_dir(verus_source_dir()).arg("/c").arg(format!(
            "& '..\\tools\\activate.ps1'; vargo build {} --features singular",
            if release { "--release" } else { "" }
        ));
        debug!("{:?}", cmd);
        cmd.status().unwrap_or_else(|e| {
            error!("Failed to build verus: {}", e);
        });
        status!("Verus build complete");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn build_verus(release: bool) -> Result<(), DynError> {
        let toolchain = verus_dir().join("rust-toolchain.toml");
        let toolchain_name = toolchain::load_toolchain(&toolchain);

        let cmd = &mut Command::new("bash");
        cmd.current_dir(verus_source_dir())
            .env_remove("RUSTUP_TOOLCHAIN")
            .env("RUSTUP_TOOLCHAIN", toolchain_name)
            .arg("-c")
            .arg(format!(
                "source ../tools/activate; vargo build {} --features singular",
                if release { "--release" } else { "" }
            ));
        debug!("{:?}", cmd);
        cmd.status().unwrap_or_else(|e| {
            error!("Failed to build verus: {}", e);
        });
        status!("Verus build complete");
        Ok(())
    }

    pub fn exec_bootstrap(options: &VerusInstallOpts) -> Result<(), DynError> {
        let verus_dir = verus_dir();

        if options.branch.is_some() {
            error!("Specifying a branch is only supported during upgrade.");
        }

        if options.restart && verus_dir.exists() {
            info!("Removing old verus installation...");
            std::fs::remove_dir_all(&verus_dir)?;
        }

        // Clone the Verus repo if it doesn't exist
        if !verus_dir.exists() {
            clone_repo(&verus_dir)?;
        }

        // Download Z3
        install_z3()?;

        // Build Verus
        build_verus(options.release)?;

        // Update the workspace toolchain
        toolchain::sync_toolchain(
            &verus_dir.join("rust-toolchain.toml"),
            &projects::get_root().join("rust-toolchain.toml"),
        );

        // Install Verusfmt
        if options.restart || !is_verusfmt_installed() {
            install_verusfmt()?;
        }

        status!("Verus installation complete");
        Ok(())
    }

    pub fn git_pull(dir: &Path, branch: Option<&str>, force_reset: bool) -> Result<(), DynError> {
        let repo = Repository::open(dir).unwrap_or_else(|e| {
            error!(
                "Unable to find the git repo of verus under {}: {}",
                dir.display(),
                e
            );
        });

        // Determine target branch (default to "main")
        let target_branch = branch.unwrap_or("main");

        // Find the remote and check its URL to determine authentication method
        let mut remote = repo.find_remote("origin")?;
        let remote_url = remote.url().unwrap_or("");
        let is_ssh = remote_url.starts_with("git@") || remote_url.contains("ssh://");

        let mut callbacks = git2::RemoteCallbacks::new();

        if is_ssh {
            // SSH repository - use SSH key authentication
            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
            });
        }
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);
        remote.fetch(&[target_branch], Some(&mut fetch_opts), None)?;

        // Get the current branch
        let head = repo.head()?;
        if !head.is_branch() {
            return Err("HEAD is not a branch. Cannot pull.".into());
        }

        let _ = head.shorthand().ok_or("Could not get branch name")?;
        let local_commit = head.peel_to_commit()?;

        // Find the matching remote branch
        let upstream_branch = format!("refs/remotes/origin/{}", target_branch);
        let upstream_ref = repo.find_reference(&upstream_branch)?;
        let upstream_commit = upstream_ref.peel_to_commit()?;

        // Check merge analysis
        let annotated_commit = repo.find_annotated_commit(upstream_commit.id())?;
        let analysis = repo.merge_analysis(&[&annotated_commit])?.0;

        if analysis.is_up_to_date() {
            status!("Already up to date");
        } else if analysis.is_fast_forward() {
            // Fast-forward
            let refname = format!("refs/heads/{}", target_branch);

            // Create local branch if it doesn't exist
            if repo.find_reference(&refname).is_err() {
                repo.reference(
                    &refname,
                    upstream_commit.id(),
                    false,
                    &format!("Create branch {}", target_branch),
                )?;
            }

            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(upstream_commit.id(), "Fast-forward")?;
            repo.set_head(&refname)?;

            // Update working directory
            let mut checkout_opts = git2::build::CheckoutBuilder::new();
            checkout_opts.force();
            repo.checkout_head(Some(&mut checkout_opts))?;

            status!(
                "Fast-forwarded {} to {}",
                target_branch,
                upstream_commit.id()
            );
        } else {
            // Need to perform a merge
            let mut merge_opts = git2::MergeOptions::new();
            let mut checkout_opts = git2::build::CheckoutBuilder::new();

            // Start the merge
            repo.merge(
                &[&annotated_commit],
                Some(&mut merge_opts),
                Some(&mut checkout_opts),
            )?;

            // Check for conflicts
            if repo.index()?.has_conflicts() {
                if force_reset {
                    status!(
                        "Conflicts detected, performing force reset to origin/{}",
                        target_branch
                    );

                    // Reset the index to clean state
                    repo.reset(
                        &repo.head()?.peel_to_commit()?.as_object(),
                        git2::ResetType::Hard,
                        None,
                    )?;

                    // Force reset to the remote branch
                    let refname = format!("refs/heads/{}", target_branch);

                    // Create or update the local branch reference
                    if repo.find_reference(&refname).is_err() {
                        repo.reference(
                            &refname,
                            upstream_commit.id(),
                            false,
                            &format!("Force reset to origin/{}", target_branch),
                        )?;
                    } else {
                        let mut reference = repo.find_reference(&refname)?;
                        reference.set_target(
                            upstream_commit.id(),
                            &format!("Force reset to origin/{}", target_branch),
                        )?;
                    }

                    // Set HEAD to the target branch
                    repo.set_head(&refname)?;

                    // Force checkout to update working directory
                    let mut checkout_opts = git2::build::CheckoutBuilder::new();
                    checkout_opts.force();
                    repo.checkout_head(Some(&mut checkout_opts))?;

                    status!("Force reset to origin/{} completed", target_branch);
                    return Ok(());
                } else {
                    error!("There are conflicts between the recent updates and patches. Please resolve them manually.");
                }
            }

            // Create the merge commit
            let sig = repo.signature()?;
            let tree_id = repo.index()?.write_tree()?;
            let tree = repo.find_tree(tree_id)?;

            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Merge remote-tracking branch 'origin/{}'", target_branch),
                &tree,
                &[&local_commit, &upstream_commit],
            )?;

            // Clean up merge state
            repo.cleanup_state()?;

            status!("Merged origin/{} into {}", target_branch, target_branch);
        }

        status!(
            "Repo {} updated to commit {}",
            dir.display(),
            upstream_commit.id()
        );
        Ok(())
    }

    pub fn exec_upgrade(options: &VerusInstallOpts) -> Result<(), DynError> {
        // rebuild if required or if the directory doesn't exist
        if options.restart || !verus_dir().exists() {
            return exec_bootstrap(options);
        }

        // git pull the Verus repo
        let verus_dir = verus_dir();
        git_pull(
            &verus_dir,
            options.branch.as_ref().map(|s| s.as_str()),
            options.force_reset,
        )?;
        status!("Verus repo updated to the latest version");

        // Build Verus
        build_verus(options.release)?;

        // Update the workspace toolchain
        toolchain::sync_toolchain(
            &verus_dir.join("rust-toolchain.toml"),
            &projects::get_root().join("rust-toolchain.toml"),
        );

        // Install Verusfmt
        install_verusfmt()?;

        status!("Verus upgrade complete");
        Ok(())
    }
}
