use std::{fs::File, io::Read, path::PathBuf};
use walkdir::WalkDir;
use rayon::prelude::*;
use blake3::Hasher;

type Hash = [u8; 32];

pub fn fingerprint_file(path: &PathBuf) -> Hash {
    let mut hasher = Hasher::new();
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    hasher.update(&buffer);
    *hasher.finalize().as_bytes()
}

pub fn fingerprint_dir(root: &PathBuf) -> Hash {
    let mut paths: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<_>>();
    paths.sort();

    let file_hashes: Vec<(PathBuf, Hash)> = paths
        .into_par_iter()
        .map(|path| {
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let h = fingerprint_file(&path);
            (rel, h)
        })
        .collect::<Vec<_>>();

    let mut dir_hasher = Hasher::new();
    for (rel, h) in file_hashes.iter() {
        dir_hasher.update(rel.to_string_lossy().as_bytes());
        dir_hasher.update(h);
    }
    *dir_hasher.finalize().as_bytes()
}

pub fn fingerprint_as_str(hash: &Hash) -> String {
    hex::encode(hash)
}

