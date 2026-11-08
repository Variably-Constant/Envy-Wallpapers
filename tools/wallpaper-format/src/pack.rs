//! A wallpaper's package: a tar of `wallpaper.toml`, its thumbnail and images in the order of their paths, then its built
//! script as `script.wasm`, each entry with the same mode, owner and time. The same files always make the same bytes, so
//! a package's SHA-256 says which files it holds.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::manifest::{Manifest, FILE};

/// The name a built script takes in a package.
pub const SCRIPT: &str = "script.wasm";

/// The file name of `manifest`'s package: `<id>-<version>.tar`.
pub fn file_name(manifest: &Manifest) -> String {
    format!("{}-{}.tar", manifest.id, manifest.version)
}

/// The release tag `manifest`'s package is published under: `<id>-v<version>`.
pub fn tag(manifest: &Manifest) -> String {
    format!("{}-v{}", manifest.id, manifest.version)
}

/// The package of the wallpaper in `dir` with the files `manifest` names, and `script`, its built script, when it has one.
/// A script's source stays out: the TV runs only what CI built from it.
pub fn pack(dir: &Path, manifest: &Manifest, script: Option<&[u8]>) -> Result<Vec<u8>, String> {
    match (script, &manifest.script) {
        (Some(_), None) => return Err("a built script for a wallpaper with no [script]".to_string()),
        (None, Some(_)) => return Err("the wallpaper's script is not built".to_string()),
        (Some(_), Some(_)) | (None, None) => {}
    }
    let mut paths: Vec<&str> = manifest.images().into_iter().map(|(path, _)| path).collect();
    paths.push(FILE);
    paths.sort_unstable();
    paths.dedup();
    let mut builder = tar::Builder::new(Vec::new());
    for path in paths {
        let bytes = std::fs::read(dir.join(path)).map_err(|e| format!("{path}: {e}"))?;
        append(&mut builder, path, &bytes)?;
    }
    if let Some(bytes) = script {
        append(&mut builder, SCRIPT, bytes)?;
    }
    builder.into_inner().map_err(|e| format!("the package: {e}"))
}

/// Adds `bytes` at `path` with the mode, owner and time every entry takes.
fn append(builder: &mut tar::Builder<Vec<u8>>, path: &str, bytes: &[u8]) -> Result<(), String> {
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    builder.append_data(&mut header, path, bytes).map_err(|e| format!("{path}: {e}"))
}

/// The SHA-256 of `bytes`, in lowercase hexadecimal.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::tests::{picture, scene};
    use crate::manifest::FILE;

    fn manifest_of(folder: &crate::check::tests::Folder) -> Manifest {
        let text = std::fs::read_to_string(folder.0.join(FILE)).expect("the manifest");
        Manifest::parse(&text).expect("the manifest reads")
    }

    /// The paths of `package`'s entries, in order.
    fn entries(package: &[u8]) -> Vec<String> {
        let mut archive = tar::Archive::new(package);
        archive
            .entries()
            .expect("the package's entries")
            .map(|entry| entry.expect("an entry").path().expect("its path").to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_same_files_make_the_same_package() {
        let folder = scene();
        let manifest = manifest_of(&folder);
        let one = pack(&folder.0, &manifest, None).expect("a package");
        let two = pack(&folder.0, &manifest, None).expect("a package");
        assert_eq!(one, two);
        assert_eq!(sha256(&one), sha256(&two));
        assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(file_name(&manifest), "leaf-pile-1.0.0.tar");
        assert_eq!(tag(&manifest), "leaf-pile-v1.0.0");
    }

    #[test]
    fn a_package_holds_the_named_files_in_order_and_the_script_last() {
        let folder = picture();
        folder.put("notes.txt", b"left out");
        let manifest = manifest_of(&folder);
        let package = pack(&folder.0, &manifest, None).expect("a package");
        assert_eq!(entries(&package), vec!["harbour.png", "thumb.png", "wallpaper.toml"]);
        let scene = scene();
        let text = format!("{}\n[script]\nsource = \"script\"\nfuel = 2000000\nmemory = 4\n", crate::manifest::tests::SCENE);
        scene.put(FILE, text.as_bytes());
        let scripted = manifest_of(&scene);
        let package = pack(&scene.0, &scripted, Some(b"\0asm\x01\0\0\0")).expect("a package");
        let names = entries(&package);
        assert_eq!(names.last().map(String::as_str), Some(SCRIPT));
        assert!(!names.iter().any(|name| name.starts_with("script/")));
    }

    #[test]
    fn a_script_must_be_built_and_only_for_a_wallpaper_with_one() {
        let folder = picture();
        let manifest = manifest_of(&folder);
        assert!(pack(&folder.0, &manifest, Some(b"\0asm")).is_err());
        let scene = scene();
        let text = format!("{}\n[script]\nsource = \"script\"\nfuel = 2000000\nmemory = 4\n", crate::manifest::tests::SCENE);
        scene.put(FILE, text.as_bytes());
        let scripted = manifest_of(&scene);
        assert!(pack(&scene.0, &scripted, None).is_err());
    }
}
