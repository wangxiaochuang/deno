use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::Result;
use bundler::run_bundle;
use glob::{glob, GlobError};

use crate::BUILD_DIR;

pub(crate) fn get_files_with_exts(dir: &str, exts: &[&str]) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    // for ext in exts {
    //     let rule = format!("{}/**/*.{}", dir, ext);
    //     let paths = glob(&rule)?.collect::<Result<BTreeSet<PathBuf>, GlobError>>()?;
    //     files.extend(paths)
    // }
    let _ = exts
        .iter()
        .map(|ext| format!("{}/**/*.{}", dir, ext))
        .try_fold(&mut files, |files, ref rule| {
            let paths = glob(rule)?.collect::<Result<BTreeSet<PathBuf>, GlobError>>()?;
            files.extend(paths);
            Ok::<&mut BTreeSet<_>, anyhow::Error>(files)
        })?;
    // let files: BTreeSet<PathBuf> = exts
    //     .into_iter()
    //     .flat_map(|ext| {
    //         let rule = format!("{}/**/*.{}", dir, ext);
    //         let a = glob(&rule).ok()?.flatten().map(PathBuf::from);
    //         a
    //     })
    //     .collect::<Result<BTreeSet<PathBuf>, _>>()?;
    Ok(files)
}
pub(crate) fn calc_project_hash(dir: &str) -> Result<String> {
    calc_hash_for_files(dir, &["ts", "js", "json"], 16)
}

pub(crate) fn calc_hash_for_files(dir: &str, exts: &[&str], len: usize) -> Result<String> {
    let files = get_files_with_exts(dir, exts)?;
    let mut hasher = blake3::Hasher::new();
    // for file in files {
    //     hasher.update_reader(File::open(file)?)?;
    // }
    Ok(files
        .into_iter()
        .try_fold(&mut hasher, |hasher, file| {
            hasher.update_reader(File::open(file)?)
        })?
        .finalize()
        .to_string()
        .chars()
        .take(len)
        .collect())
}
pub(crate) fn build_project(dir: &str) -> Result<String> {
    let hash = calc_project_hash(dir)?;
    let filename = format!("{}/{}.mjs", BUILD_DIR, hash);
    let dst = Path::new(&filename);
    if dst.exists() {
        return Ok(filename);
    }
    let content = run_bundle("main.ts", &Default::default())?;
    fs::write(dst, content)?;
    Ok(filename)
}
