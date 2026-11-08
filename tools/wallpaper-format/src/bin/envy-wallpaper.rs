//! envy-wallpaper: checks, packs and lists the wallpapers of Envy's store.
//!
//!   envy-wallpaper check <folder>...
//!       says whether each wallpaper's folder is fit to publish, and every problem found in it
//!   envy-wallpaper pack <folder> <out-folder> [<built script.wasm>]
//!       writes the wallpaper's package, <id>-<version>.tar, to <out-folder>, with its size and SHA-256
//!   envy-wallpaper catalog <wallpapers-folder> <packages-folder> <release-base> <thumbnail-base>
//!       writes catalog.json to standard output: each wallpaper in <wallpapers-folder>, its package from
//!       <packages-folder> at <release-base>/<id>-v<version>/<id>-<version>.tar, and its thumbnail at
//!       <thumbnail-base>/<id>/<thumbnail>
//!   envy-wallpaper verify <catalog.json> <catalog.json.minisig> <public key>
//!       checks the catalog's minisign signature against the store's public key, in base64
//!   envy-wallpaper name <folder>
//!       prints the wallpaper's release tag, its package's file name and its thumbnail's path, between tabs

use std::path::Path;
use std::process::ExitCode;

use wallpaper_format::catalog::{self, Catalog, Entry};
use wallpaper_format::check::{check, Limits, Problem};
use wallpaper_format::pack;
use wallpaper_format::Manifest;

const USAGE: &str = "usage:
  envy-wallpaper check <folder>...
  envy-wallpaper pack <folder> <out-folder> [<built script.wasm>]
  envy-wallpaper catalog <wallpapers-folder> <packages-folder> <release-base> <thumbnail-base>
  envy-wallpaper verify <catalog.json> <catalog.json.minisig> <public key>
  envy-wallpaper name <folder>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done = match args.first().map(String::as_str) {
        Some("check") => check_all(&args[1..]),
        Some("pack") => pack_one(&args[1..]),
        Some("catalog") => list(&args[1..]),
        Some("verify") => verify(&args[1..]),
        Some("name") => name(&args[1..]),
        Some(other) => Err(format!("no command {other}\n{USAGE}")),
        None => Err(USAGE.to_string()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// The problems as lines under `folder`.
fn told(folder: &str, problems: &[Problem]) -> String {
    let lines: Vec<String> = problems
        .iter()
        .map(|problem| if problem.at.is_empty() { format!("  {}", problem.what) } else { format!("  {}: {}", problem.at, problem.what) })
        .collect();
    format!("{folder}: {} problems\n{}", problems.len(), lines.join("\n"))
}

/// The wallpaper in `folder`, if it is fit to publish.
fn fit(folder: &str) -> Result<Manifest, String> {
    match check(Path::new(folder), &Limits::STORE) {
        (Some(manifest), problems) if problems.is_empty() => Ok(manifest),
        (_, problems) => Err(told(folder, &problems)),
    }
}

fn check_all(folders: &[String]) -> Result<(), String> {
    if folders.is_empty() {
        return Err(USAGE.to_string());
    }
    let mut unfit = Vec::new();
    for folder in folders {
        match fit(folder) {
            Ok(manifest) => println!("{folder}: {} {} is fit to publish", manifest.id, manifest.version),
            Err(e) => unfit.push(e),
        }
    }
    if unfit.is_empty() {
        Ok(())
    } else {
        Err(unfit.join("\n"))
    }
}

fn pack_one(args: &[String]) -> Result<(), String> {
    let (folder, out, script) = match args {
        [folder, out] => (folder, out, None),
        [folder, out, script] => (folder, out, Some(script)),
        _ => return Err(USAGE.to_string()),
    };
    let manifest = fit(folder)?;
    let built = match script {
        Some(path) => Some(std::fs::read(path).map_err(|e| format!("{path}: {e}"))?),
        None => None,
    };
    let package = pack::pack(Path::new(folder), &manifest, built.as_deref())?;
    let to = Path::new(out).join(pack::file_name(&manifest));
    std::fs::write(&to, &package).map_err(|e| format!("{}: {e}", to.display()))?;
    println!("{}: {} bytes, sha256 {}", to.display(), package.len(), pack::sha256(&package));
    Ok(())
}

fn list(args: &[String]) -> Result<(), String> {
    let [wallpapers, packages, release, thumbnails] = args else {
        return Err(USAGE.to_string());
    };
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(wallpapers).map_err(|e| format!("{wallpapers}: {e}"))? {
        let entry = entry.map_err(|e| format!("{wallpapers}: {e}"))?;
        if entry.file_type().map_err(|e| format!("{}: {e}", entry.path().display()))?.is_dir() {
            folders.push(entry.path());
        }
    }
    folders.sort();
    let mut entries = Vec::new();
    for folder in folders {
        let manifest = fit(&folder.to_string_lossy())?;
        let file = pack::file_name(&manifest);
        let path = Path::new(packages).join(&file);
        let package = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let url = format!("{}/{}/{file}", release.trim_end_matches('/'), pack::tag(&manifest));
        let thumbnail = format!("{}/{}/{}", thumbnails.trim_end_matches('/'), manifest.id, manifest.thumbnail);
        entries.push(Entry::new(&manifest, package.len() as u64, pack::sha256(&package), url, thumbnail));
    }
    println!("{}", Catalog::new(entries).to_json()?);
    Ok(())
}

fn name(args: &[String]) -> Result<(), String> {
    let [folder] = args else {
        return Err(USAGE.to_string());
    };
    let manifest = fit(folder)?;
    println!("{}\t{}\t{}", pack::tag(&manifest), pack::file_name(&manifest), manifest.thumbnail);
    Ok(())
}

fn verify(args: &[String]) -> Result<(), String> {
    let [catalog_path, signature_path, public_key] = args else {
        return Err(USAGE.to_string());
    };
    let catalog = std::fs::read(catalog_path).map_err(|e| format!("{catalog_path}: {e}"))?;
    let signature = std::fs::read_to_string(signature_path).map_err(|e| format!("{signature_path}: {e}"))?;
    catalog::verify(&catalog, &signature, public_key)?;
    let listed = Catalog::from_json(&String::from_utf8_lossy(&catalog))?;
    println!("{catalog_path}: signed by the store's key, {} wallpapers", listed.wallpapers.len());
    Ok(())
}
