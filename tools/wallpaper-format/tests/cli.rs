//! The envy-wallpaper tool, run as a contributor and CI run it: checking a wallpaper's folder, packing it, and listing it
//! in a catalog that points at the store's releases and pages.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use wallpaper_format::catalog::Catalog;
use wallpaper_format::pack;

const RELEASES: &str = "https://github.com/Variably-Constant/Envy-Wallpapers/releases/download";
const PAGES: &str = "https://variably-constant.github.io/Envy-Wallpapers/thumbnails";

const PICTURE: &str = r#"
format = 1
id = "misty-harbour"
title = "Misty Harbour"
version = "1.0.0"
kind = "picture"
author = "A. Creator"
license = "CC0-1.0"
thumbnail = "thumb.png"

[picture]
image = "harbour.png"
"#;

/// A folder for one test, gone when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("envy-wallpaper-cli-{}-{n}", std::process::id()));
        std::fs::create_dir_all(dir.join("packages")).expect("a test folder");
        Scratch(dir)
    }

    fn put(&self, path: &str, bytes: &[u8]) {
        let full = self.0.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("a test folder");
        }
        std::fs::write(full, bytes).expect("a test file");
    }

    fn path(&self, rel: &str) -> String {
        self.0.join(rel).to_string_lossy().into_owned()
    }

    /// The picture wallpaper `misty-harbour`, with its thumbnail `thumb` pixels in size.
    fn picture(&self, thumb: (u32, u32)) {
        self.put("wallpapers/misty-harbour/wallpaper.toml", PICTURE.as_bytes());
        self.put("wallpapers/misty-harbour/thumb.png", &png(thumb.0, thumb.1));
        self.put("wallpapers/misty-harbour/harbour.png", &png(1920, 1080));
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.0) {
            eprintln!("left {}: {e}", self.0.display());
        }
    }
}

/// The first bytes of a PNG of `w` by `h`, as far as its header.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
    bytes.extend_from_slice(&w.to_be_bytes());
    bytes.extend_from_slice(&h.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

/// What the tool says to `args`: whether it succeeded, then its standard output and its errors.
fn tool(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_envy-wallpaper")).args(args).output().expect("the tool runs");
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn checks_packs_and_lists_a_wallpaper() {
    let scratch = Scratch::new();
    scratch.picture((480, 270));
    let wallpaper = scratch.path("wallpapers/misty-harbour");
    let (ok, out, err) = tool(&["check", &wallpaper]);
    assert!(ok, "{err}");
    assert!(out.contains("misty-harbour 1.0.0 is fit to publish"), "{out}");
    let (ok, out, err) = tool(&["name", &wallpaper]);
    assert!(ok, "{err}");
    assert_eq!(out.trim_end(), "misty-harbour-v1.0.0\tmisty-harbour-1.0.0.tar\tthumb.png");
    let (ok, out, err) = tool(&["pack", &wallpaper, &scratch.path("packages")]);
    assert!(ok, "{err}");
    assert!(out.contains("misty-harbour-1.0.0.tar"), "{out}");
    let package = std::fs::read(scratch.0.join("packages/misty-harbour-1.0.0.tar")).expect("the package");
    assert!(out.contains(&pack::sha256(&package)), "{out}");
    let (ok, out, err) = tool(&["catalog", &scratch.path("wallpapers"), &scratch.path("packages"), RELEASES, PAGES]);
    assert!(ok, "{err}");
    let catalog = Catalog::from_json(&out).expect("the tool writes a catalog");
    let entry = &catalog.wallpapers[0];
    assert_eq!(entry.package, format!("{RELEASES}/misty-harbour-v1.0.0/misty-harbour-1.0.0.tar"));
    assert_eq!(entry.thumbnail, format!("{PAGES}/misty-harbour/thumb.png"));
    assert_eq!(entry.sha256, pack::sha256(&package));
    assert_eq!(entry.size, package.len() as u64);
}

#[test]
fn refuses_a_folder_with_problems_and_names_each() {
    let scratch = Scratch::new();
    scratch.picture((640, 360));
    scratch.put("wallpapers/misty-harbour/notes.txt", b"draft");
    let wallpaper = scratch.path("wallpapers/misty-harbour");
    let (ok, _, err) = tool(&["check", &wallpaper]);
    assert!(!ok);
    assert!(err.contains("2 problems"), "{err}");
    assert!(err.contains("thumb.png: is 640 x 360, where a thumbnail is 480 x 270"), "{err}");
    assert!(err.contains("notes.txt: is not named by wallpaper.toml"), "{err}");
    let (ok, _, err) = tool(&["pack", &wallpaper, &scratch.path("packages")]);
    assert!(!ok, "a folder with problems is not packed");
    assert!(err.contains("2 problems"), "{err}");
}

#[test]
fn says_how_it_is_used() {
    let (ok, _, err) = tool(&[]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
    let (ok, _, err) = tool(&["publish"]);
    assert!(!ok);
    assert!(err.contains("no command publish"), "{err}");
}
