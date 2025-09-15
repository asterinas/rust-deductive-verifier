use crate::files;
use std::env;
use std::path::{Path, PathBuf};

pub fn locate_from_path<P>(binary: &P) -> Option<PathBuf>
where
    P: AsRef<Path> + ?Sized,
{
    let path = env::var("PATH").ok()?;
    let paths = env::split_paths(&path);
    paths
        .filter_map(|dir| {
            let full = dir.join(binary);
            if full.is_file() {
                Some(full)
            } else {
                None
            }
        })
        .next()
}

pub fn locate_from_hints<P, D>(binary: &P, hints: &[D]) -> Option<PathBuf>
where
    P: AsRef<Path> + ?Sized,
    D: AsRef<Path>,
{
    hints
        .iter()
        .filter_map(|hint| {
            let full = hint.as_ref().join(binary);
            if full.is_file() {
                Some(full)
            } else {
                None
            }
        })
        .next()
}

pub fn locate_from_env<P>(binary: &P, env_var: &str) -> Option<PathBuf>
where
    P: AsRef<Path> + ?Sized,
{
    let env_path = env::var(env_var).ok()?;
    let paths = env::split_paths(&env_path);
    paths
        .filter_map(|dir| {
            let full = dir.join(binary);
            if full.is_file() {
                Some(full)
            } else {
                None
            }
        })
        .next()
}

pub fn locate<P, D>(binary: &P, env_var: Option<&str>, hints: &[D]) -> Option<PathBuf>
where
    P: AsRef<Path> + ?Sized,
    D: AsRef<Path>,
{
    let path = env_var
        .and_then(|e| locate_from_env(binary, e))
        .or_else(|| locate_from_hints(binary, hints))
        .or_else(|| locate_from_path(binary));

    path.map(|path| files::absolutize(&path))
}
