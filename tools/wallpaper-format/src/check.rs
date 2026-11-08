//! The checks a wallpaper's folder passes before it is published. Its `wallpaper.toml` reads and says what it must; every
//! file it names is there, of a kind allowed and within its size; nothing else lies in the folder; and nothing in the
//! folder is a link or reaches outside it.

use std::io::Read;
use std::path::Path;

use crate::manifest::{Kind, Manifest, Use, FILE, FORMAT};

/// The limits a wallpaper is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Bytes in the folder in all, a script's source included.
    pub total: u64,
    /// Bytes in `wallpaper.toml`.
    pub manifest: u64,
    /// Pixels on a side of any image.
    pub side: u64,
    /// The largest a still (a picture or a layer) may be, width by height.
    pub still: (u64, u64),
    /// The thumbnail's size, which it must be exactly.
    pub thumbnail: (u64, u64),
    /// Pieces in all of a scene's emitters, bursts included.
    pub pieces: u64,
}

impl Limits {
    /// The store's own limits.
    pub const STORE: Limits = Limits { total: 32 << 20, manifest: 64 << 10, side: 4096, still: (3840, 2160), thumbnail: (480, 270), pieces: 2000 };
}

/// Something wrong with a wallpaper, and where: a file relative to its folder, a part of `wallpaper.toml` such as
/// "emitter 2", or "" for the folder itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub at: String,
    pub what: String,
}

/// The wallpaper in `dir` as its `wallpaper.toml` has it, if that reads, with every problem found. It is fit to publish
/// when there are none.
pub fn check(dir: &Path, limits: &Limits) -> (Option<Manifest>, Vec<Problem>) {
    let mut found = Found::default();
    let files = found.walk(dir);
    let total: u64 = files.iter().map(|(_, size)| size).sum();
    found.need("", total <= limits.total, format!("{total} bytes in all, over the {} allowed", limits.total));
    let manifest = found.read_manifest(dir, &files, limits);
    if let Some(manifest) = &manifest {
        found.header(dir, manifest);
        found.shape(manifest, limits);
        found.files(dir, manifest, &files, limits);
    }
    (manifest, found.problems)
}

#[derive(Default)]
struct Found {
    problems: Vec<Problem>,
}

impl Found {
    /// Notes `what` at `at` unless `ok`.
    fn need(&mut self, at: &str, ok: bool, what: impl Into<String>) {
        if !ok {
            self.problems.push(Problem { at: at.to_string(), what: what.into() });
        }
    }

    /// Every file in `dir` and the folders under it, by its path relative to `dir` with `/` between its parts, with its
    /// size, in the order of their paths; links and hidden files are problems and are left out.
    fn walk(&mut self, dir: &Path) -> Vec<(String, u64)> {
        let mut files = Vec::new();
        let mut folders = vec![(dir.to_path_buf(), String::new())];
        while let Some((folder, rel)) = folders.pop() {
            let entries = match std::fs::read_dir(&folder) {
                Ok(entries) => entries,
                Err(e) => {
                    self.need(&rel, false, format!("cannot be read: {e}"));
                    continue;
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        self.need(&rel, false, format!("cannot be read: {e}"));
                        continue;
                    }
                };
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
                let meta = match std::fs::symlink_metadata(entry.path()) {
                    Ok(meta) => meta,
                    Err(e) => {
                        self.need(&path, false, format!("cannot be read: {e}"));
                        continue;
                    }
                };
                if meta.file_type().is_symlink() {
                    self.need(&path, false, "is a link, which a wallpaper may not hold");
                } else if name.starts_with('.') {
                    self.need(&path, false, "is hidden, which a wallpaper may not hold");
                } else if meta.is_dir() {
                    folders.push((entry.path(), path));
                } else if meta.is_file() {
                    files.push((path, meta.len()));
                } else {
                    self.need(&path, false, "is neither a file nor a folder");
                }
            }
        }
        files.sort();
        files
    }

    /// The folder's `wallpaper.toml`, read, if it is there, within its size and reads.
    fn read_manifest(&mut self, dir: &Path, files: &[(String, u64)], limits: &Limits) -> Option<Manifest> {
        let Some((_, size)) = files.iter().find(|(path, _)| path == FILE) else {
            self.need(FILE, false, "is missing");
            return None;
        };
        if *size > limits.manifest {
            self.need(FILE, false, format!("is {size} bytes, over the {} allowed", limits.manifest));
            return None;
        }
        let text = match std::fs::read_to_string(dir.join(FILE)) {
            Ok(text) => text,
            Err(e) => {
                self.need(FILE, false, format!("cannot be read: {e}"));
                return None;
            }
        };
        match Manifest::parse(&text) {
            Ok(manifest) => Some(manifest),
            Err(e) => {
                self.need(FILE, false, e);
                None
            }
        }
    }

    /// What every wallpaper says of itself: its format, id, names, version, tags and the ways it plays.
    fn header(&mut self, dir: &Path, m: &Manifest) {
        self.need(FILE, m.format == FORMAT, format!("format {} where this checker reads {FORMAT}", m.format));
        self.need(FILE, is_id(&m.id), format!("id \"{}\" is not 3 to 40 lowercase letters, digits and single hyphens", m.id));
        let folder = match dir.canonicalize() {
            Ok(full) => full.file_name().map(|name| name.to_string_lossy().into_owned()),
            Err(e) => {
                self.need("", false, format!("cannot be found: {e}"));
                None
            }
        };
        self.need(FILE, folder.as_deref() == Some(m.id.as_str()), format!("id \"{}\" is not the folder's name", m.id));
        self.text("title", &m.title, 1, 40);
        self.text("author", &m.author, 1, 60);
        self.text("description", &m.description, 0, 300);
        self.need(FILE, is_version(&m.version), format!("version \"{}\" is not major.minor.patch", m.version));
        self.need(FILE, m.tags.len() <= 8, format!("{} tags, over the 8 allowed", m.tags.len()));
        for tag in &m.tags {
            self.need(FILE, is_tag(tag), format!("tag \"{tag}\" is not 2 to 20 lowercase letters, digits and hyphens"));
        }
        self.need(FILE, !duplicated(&m.tags), "a tag is named twice");
        self.need(FILE, !duplicated(&m.plays), "a way it plays is named twice");
    }

    /// Text of `lo` to `hi` characters with none of them a control character.
    fn text(&mut self, name: &str, value: &str, lo: usize, hi: usize) {
        let n = value.chars().count();
        self.need(FILE, (lo..=hi).contains(&n), format!("{name} is {n} characters, not {lo} to {hi}"));
        self.need(FILE, !value.chars().any(char::is_control), format!("{name} holds a control character"));
    }

    /// The parts its kind calls for, and every number within its bounds.
    fn shape(&mut self, m: &Manifest, limits: &Limits) {
        let scene_parts = !m.layers.is_empty() || !m.emitters.is_empty() || !m.actors.is_empty() || !m.objects.is_empty() || m.script.is_some();
        match m.kind {
            Kind::Picture => {
                self.need(FILE, m.picture.is_some(), "a picture with no [picture]");
                self.need(FILE, !scene_parts, "a picture with a scene's layers, emitters, actors, objects or script");
            }
            Kind::Scene => {
                self.need(FILE, m.picture.is_none(), "a scene with a [picture]");
                self.need(FILE, !m.layers.is_empty(), "a scene with no [[layer]]");
            }
        }
        if let Some(picture) = &m.picture {
            self.within("picture", "parallax", picture.parallax, 0.0, 120.0);
            if let Some(pan) = &picture.pan {
                self.within("picture", "pan zoom", pan.zoom, 1.0, 1.25);
                self.within("picture", "pan seconds", pan.seconds, 5.0, 3600.0);
            }
        }
        for (i, layer) in m.layers.iter().enumerate() {
            let at = format!("layer {}", i + 1);
            self.point(&at, "at", layer.at);
            self.within(&at, "parallax", layer.parallax, 0.0, 120.0);
            self.point(&at, "drift", layer.drift);
            if let Some(sway) = &layer.sway {
                self.within(&at, "sway degrees", sway.degrees, 0.0, 45.0);
                self.within(&at, "sway seconds", sway.seconds, 0.5, 600.0);
                self.point(&at, "sway pivot", sway.pivot);
            }
        }
        let mut pieces = 0u64;
        for (i, emitter) in m.emitters.iter().enumerate() {
            let at = format!("emitter {}", i + 1);
            self.need(&at, !emitter.sprites.is_empty(), "no sprites");
            self.need(&at, emitter.count >= 1, "a count of 0");
            pieces += u64::from(emitter.count) + emitter.burst.map_or(0, |burst| u64::from(burst.count));
            let [_, _, w, h] = emitter.area;
            self.need(&at, emitter.area.iter().all(|v| v.is_finite()) && w > 0.0 && h > 0.0, "an area with no width or height");
            if let Some(life) = emitter.life {
                self.range(&at, "life", life, 0.1, 600.0);
            }
            let [x0, x1, y0, y1] = emitter.speed;
            let speeds_ok = emitter.speed.iter().all(|v| v.is_finite() && v.abs() <= 4000.0) && x0 <= x1 && y0 <= y1;
            self.need(&at, speeds_ok, "speed is not two ranges, across then down, of at most 4000 pixels a second");
            self.within(&at, "gravity", emitter.gravity, -4000.0, 4000.0);
            self.range(&at, "spin", emitter.spin, -1440.0, 1440.0);
            self.range(&at, "size", emitter.size, 0.05, 8.0);
            if let Some(twinkle) = emitter.twinkle {
                self.within(&at, "twinkle seconds", twinkle.seconds, 0.1, 60.0);
                self.within(&at, "twinkle low", twinkle.low, 0.0, 1.0);
            }
            if let Some(wind) = emitter.wind {
                self.within(&at, "wind strength", wind.strength, 0.0, 10.0);
                self.within(&at, "wind reach", wind.reach, 1.0, 1920.0);
            }
            if let Some(attract) = emitter.attract {
                self.within(&at, "attract strength", attract.strength, 0.0, 10.0);
                self.within(&at, "attract radius", attract.radius, 0.0, 1920.0);
            }
            if let Some(burst) = emitter.burst {
                self.need(&at, burst.count >= 1, "a burst of 0");
                self.within(&at, "burst speed", burst.speed, 0.0, 4000.0);
            }
            if let Some(sync) = emitter.sync {
                self.within(&at, "sync seconds", sync.seconds, 0.1, 60.0);
                self.within(&at, "sync coupling", sync.coupling, 0.0, 1.0);
                self.within(&at, "sync held", sync.held, 0.0, 1.0);
            }
        }
        self.need(FILE, pieces <= limits.pieces, format!("{pieces} pieces in all the emitters, over the {} allowed", limits.pieces));
        for (i, actor) in m.actors.iter().enumerate() {
            let at = format!("actor {}", i + 1);
            self.need(&at, !actor.frames.is_empty(), "no frames");
            self.within(&at, "fps", actor.fps, 0.5, 60.0);
            self.point(&at, "at", actor.at);
            for point in &actor.path {
                self.point(&at, "path", *point);
            }
            self.within(&at, "speed", actor.speed, 0.0, 4000.0);
            self.need(&at, actor.path.is_empty() || actor.speed > 0.0, "a path with no speed");
            if let Some(follow) = actor.follow {
                self.within(&at, "follow speed", follow.speed, 1.0, 4000.0);
            }
            if let Some(flee) = actor.flee {
                self.within(&at, "flee reach", flee.reach, 1.0, 1920.0);
                self.within(&at, "flee back_after", flee.back_after, 0.0, 600.0);
            }
            if let Some(gaze) = actor.gaze {
                self.within(&at, "gaze degrees", gaze.degrees, 0.0, 180.0);
                self.point(&at, "gaze pivot", gaze.pivot);
            }
        }
        for (i, object) in m.objects.iter().enumerate() {
            let at = format!("object {}", i + 1);
            self.need(&at, !object.states.is_empty(), "no states");
            self.point(&at, "at", object.at);
            self.within(&at, "radius", object.radius, 1.0, 1080.0);
            self.within(&at, "hover", object.hover, 1.0, 2.0);
            if let Some(hold) = object.hold {
                self.within(&at, "hold", hold, 0.1, 30.0);
            }
        }
        if let Some(script) = &m.script {
            self.need("script", (1..=10_000_000).contains(&script.fuel), format!("fuel {} is not 1 to 10 000 000 instructions a frame", script.fuel));
            self.need("script", (1..=16).contains(&script.memory), format!("memory {} is not 1 to 16 megabytes", script.memory));
        }
    }

    /// A number from `lo` to `hi`, which also keeps out the NaN and infinities TOML can write.
    fn within(&mut self, at: &str, name: &str, value: f64, lo: f64, hi: f64) {
        self.need(at, (lo..=hi).contains(&value), format!("{name} {value} is not {lo} to {hi}"));
    }

    /// Two numbers from `lo` to `hi`, the first no larger than the second.
    fn range(&mut self, at: &str, name: &str, [a, b]: [f64; 2], lo: f64, hi: f64) {
        self.need(at, (lo..=hi).contains(&a) && (lo..=hi).contains(&b) && a <= b, format!("{name} [{a}, {b}] is not a range within {lo} to {hi}"));
    }

    /// A point within four frames of the frame each way.
    fn point(&mut self, at: &str, name: &str, [x, y]: [f64; 2]) {
        let reach = -7680.0..=7680.0;
        self.need(at, reach.contains(&x) && reach.contains(&y), format!("{name} [{x}, {y}] is not within 7680 pixels of the frame"));
    }

    /// The files the manifest names, and that the folder holds nothing else.
    fn files(&mut self, dir: &Path, m: &Manifest, files: &[(String, u64)], limits: &Limits) {
        let mut allowed = vec![FILE.to_string()];
        for (path, used) in m.images() {
            if let Err(e) = named(path) {
                self.need(FILE, false, format!("\"{path}\" {e}"));
                continue;
            }
            allowed.push(path.to_string());
            if files.iter().any(|(file, _)| file == path) {
                self.image(dir, path, used, limits);
            } else {
                self.need(path, false, "is named but missing");
            }
        }
        let source = m.script.as_ref().map(|script| script.source.trim_end_matches('/').to_string());
        if let Some(source) = &source {
            match named(source) {
                Err(e) => self.need(FILE, false, format!("the script's source \"{source}\" {e}")),
                Ok(()) => {
                    for needed in ["Cargo.toml", "src/lib.rs"] {
                        let path = format!("{source}/{needed}");
                        self.need(&path, files.iter().any(|(file, _)| *file == path), "is missing from the script's source");
                    }
                    self.script_crate(dir, source, files);
                }
            }
        }
        for (path, _) in files {
            let within = source.as_ref().and_then(|source| path.strip_prefix(&format!("{source}/")));
            if let Some(within) = within {
                let source_file = within == "Cargo.toml" || within == "Cargo.lock" || (within.starts_with("src/") && within.ends_with(".rs")) || within.ends_with(".md");
                self.need(path, source_file, "is not the crate's Cargo.toml or Cargo.lock, Rust source under src/, or Markdown, all a script's source may hold");
            } else {
                self.need(path, allowed.contains(path), "is not named by wallpaper.toml");
            }
        }
    }

    /// A script's crate builds to WebAssembly and runs nothing of its author's while it builds: its Cargo.toml holds only
    /// [package], [lib] as a cdylib and [profile], with no dependency of any kind, no build script and no proc macro. CI
    /// builds every script, and the job that publishes holds the catalog's signing key.
    fn script_crate(&mut self, dir: &Path, source: &str, files: &[(String, u64)]) {
        let path = format!("{source}/Cargo.toml");
        let build = format!("{source}/build.rs");
        self.need(&build, !files.iter().any(|(file, _)| *file == build), "is a build script, which a script may not have");
        let text = match std::fs::read_to_string(dir.join(&path)) {
            Ok(text) => text,
            Err(e) => {
                self.need(&path, false, format!("cannot be read: {e}"));
                return;
            }
        };
        let table: toml::Table = match toml::from_str(&text) {
            Ok(table) => table,
            Err(e) => {
                self.need(&path, false, format!("does not read as TOML: {e}"));
                return;
            }
        };
        for key in table.keys() {
            self.need(&path, ["package", "lib", "profile"].contains(&key.as_str()), format!("holds [{key}], where a script's crate holds only [package], [lib] and [profile]"));
        }
        let package = table.get("package").and_then(toml::Value::as_table);
        for key in ["build", "links"] {
            self.need(&path, !package.is_some_and(|package| package.contains_key(key)), format!("names a {key} in [package], which a script's crate may not"));
        }
        let lib = table.get("lib").and_then(toml::Value::as_table);
        let cdylib = lib
            .and_then(|lib| lib.get("crate-type"))
            .and_then(toml::Value::as_array)
            .is_some_and(|types| types.len() == 1 && types[0].as_str() == Some("cdylib"));
        self.need(&path, cdylib, "has no [lib] crate-type = [\"cdylib\"], which builds it to WebAssembly");
        self.need(&path, !lib.is_some_and(|lib| lib.contains_key("proc-macro")), "makes a proc macro, which a script's crate may not");
    }

    /// An image whose contents are the kind its name says, of a size its use allows.
    fn image(&mut self, dir: &Path, path: &str, used: Use, limits: &Limits) {
        let lower = path.to_ascii_lowercase();
        let wanted = if lower.ends_with(".png") {
            imagesize::ImageType::Png
        } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            imagesize::ImageType::Jpeg
        } else if lower.ends_with(".webp") {
            imagesize::ImageType::Webp
        } else {
            self.need(path, false, "is not a .png, .jpg or .webp");
            return;
        };
        let full = dir.join(path);
        let head = match read_head(&full) {
            Ok(head) => head,
            Err(e) => {
                self.need(path, false, format!("cannot be read: {e}"));
                return;
            }
        };
        match imagesize::image_type(&head) {
            Ok(kind) if kind == wanted => {}
            Ok(kind) => self.need(path, false, format!("holds a {kind:?} image, not the kind its name says")),
            Err(e) => self.need(path, false, format!("is not an image: {e}")),
        }
        let size = match imagesize::size(&full) {
            Ok(size) => size,
            Err(e) => {
                self.need(path, false, format!("gives no size in its header: {e}"));
                return;
            }
        };
        let (w, h) = (size.width as u64, size.height as u64);
        self.need(path, (1..=limits.side).contains(&w) && (1..=limits.side).contains(&h), format!("is {w} x {h}, not 1 to {} a side", limits.side));
        match used {
            Use::Thumbnail => {
                let (tw, th) = limits.thumbnail;
                self.need(path, (w, h) == (tw, th), format!("is {w} x {h}, where a thumbnail is {tw} x {th}"));
            }
            Use::Still => {
                let (sw, sh) = limits.still;
                self.need(path, w <= sw && h <= sh, format!("is {w} x {h}, over the {sw} x {sh} a still may be"));
            }
            Use::Sprite => {}
        }
    }
}

/// The first bytes of the file at `path`, enough for its type to show.
fn read_head(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(64);
    std::fs::File::open(path)?.take(64).read_to_end(&mut head)?;
    Ok(head)
}

/// Whether `path` names a file inside the wallpaper's folder by a path of its own: relative, with `/` between its parts,
/// and no part empty or starting with a dot, which keeps out `.` and `..`.
fn named(path: &str) -> Result<(), &'static str> {
    if path.contains('\\') {
        return Err("has a backslash, where paths take /");
    }
    if path.starts_with('/') || path.contains(':') {
        return Err("is not relative to the wallpaper's folder");
    }
    if path.split('/').any(|part| part.is_empty() || part.starts_with('.')) {
        return Err("has a part that is empty or starts with a dot");
    }
    Ok(())
}

/// 3 to 40 lowercase letters, digits and single hyphens, with no hyphen at either end.
fn is_id(id: &str) -> bool {
    (3..=40).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
}

/// 2 to 20 lowercase letters, digits and hyphens.
fn is_tag(tag: &str) -> bool {
    (2..=20).contains(&tag.len()) && tag.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `major.minor.patch`, each a number with no leading zero.
fn is_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| !part.is_empty() && part.len() <= 9 && part.bytes().all(|b| b.is_ascii_digit()) && (*part == "0" || !part.starts_with('0')))
}

/// Whether any item comes twice.
fn duplicated<T: PartialEq>(items: &[T]) -> bool {
    items.iter().enumerate().any(|(i, item)| items[i + 1..].contains(item))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A folder for one test's wallpaper, named `id`, inside a folder of its own that goes when the test ends.
    pub(crate) struct Folder(pub(crate) PathBuf);

    impl Folder {
        pub(crate) fn new(id: &str) -> Folder {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir().join(format!("wallpaper-format-{}-{n}", std::process::id())).join(id);
            std::fs::create_dir_all(&dir).expect("a test folder");
            Folder(dir)
        }

        pub(crate) fn put(&self, path: &str, bytes: &[u8]) {
            let full = self.0.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("a test folder");
            }
            std::fs::write(full, bytes).expect("a test file");
        }

        pub(crate) fn problems(&self) -> Vec<Problem> {
            check(&self.0, &Limits::STORE).1
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            if let Some(root) = self.0.parent() {
                if let Err(e) = std::fs::remove_dir_all(root) {
                    eprintln!("left {}: {e}", root.display());
                }
            }
        }
    }

    /// The first bytes of a PNG of `w` by `h`, as far as its header, which is all a size is read from.
    pub(crate) fn png(w: u32, h: u32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        bytes.extend_from_slice(&w.to_be_bytes());
        bytes.extend_from_slice(&h.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        bytes
    }

    /// A JPEG of `w` by `h` as far as its frame header.
    fn jpeg(w: u16, h: u16) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08];
        bytes.extend_from_slice(&h.to_be_bytes());
        bytes.extend_from_slice(&w.to_be_bytes());
        bytes.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xD9]);
        bytes
    }

    /// A good picture wallpaper, `misty-harbour`.
    pub(crate) fn picture() -> Folder {
        let folder = Folder::new("misty-harbour");
        folder.put(FILE, crate::manifest::tests::PICTURE.as_bytes());
        folder.put("thumb.png", &png(480, 270));
        folder.put("harbour.png", &png(3840, 2160));
        folder
    }

    /// A good scene wallpaper, `leaf-pile`.
    pub(crate) fn scene() -> Folder {
        let folder = Folder::new("leaf-pile");
        folder.put(FILE, crate::manifest::tests::SCENE.as_bytes());
        folder.put("thumb.png", &png(480, 270));
        folder.put("images/sky.png", &png(1920, 1080));
        folder.put("images/tree.png", &png(800, 960));
        for sprite in ["maple", "oak", "squirrel-1", "squirrel-2", "acorn", "acorn-open"] {
            folder.put(&format!("images/{sprite}.png"), &png(48, 48));
        }
        folder
    }

    /// Whether some problem's message holds `part`.
    fn says(problems: &[Problem], part: &str) -> bool {
        problems.iter().any(|problem| problem.what.contains(part))
    }

    #[test]
    fn a_good_picture_and_a_good_scene_are_fit_to_publish() {
        assert_eq!(picture().problems(), vec![]);
        assert_eq!(scene().problems(), vec![]);
    }

    #[test]
    fn the_id_must_be_well_formed_and_the_folders_name() {
        let folder = Folder::new("harbour");
        folder.put(FILE, crate::manifest::tests::PICTURE.as_bytes());
        folder.put("thumb.png", &png(480, 270));
        folder.put("harbour.png", &png(1920, 1080));
        assert!(says(&folder.problems(), "is not the folder's name"));
        let bad = Folder::new("Misty_Harbour");
        bad.put(FILE, crate::manifest::tests::PICTURE.replace("\"misty-harbour\"", "\"Misty_Harbour\"").as_bytes());
        bad.put("thumb.png", &png(480, 270));
        bad.put("harbour.png", &png(1920, 1080));
        assert!(says(&bad.problems(), "is not 3 to 40 lowercase letters"));
    }

    #[test]
    fn a_bad_version_tag_and_title_are_refused() {
        let folder = picture();
        let text = crate::manifest::tests::PICTURE
            .replace("version = \"1.0.0\"", "version = \"1.02\"")
            .replace("\"water\"", "\"Water!\"")
            .replace("title = \"Misty Harbour\"", "title = \"\"");
        folder.put(FILE, text.as_bytes());
        let problems = folder.problems();
        assert!(says(&problems, "is not major.minor.patch"));
        assert!(says(&problems, "tag \"Water!\""));
        assert!(says(&problems, "title is 0 characters"));
    }

    #[test]
    fn a_missing_file_and_a_path_that_reaches_outside_are_refused() {
        let folder = picture();
        folder.put(FILE, crate::manifest::tests::PICTURE.replace("harbour.png", "../harbour.png").as_bytes());
        let problems = folder.problems();
        assert!(says(&problems, "has a part that is empty or starts with a dot"));
        let absolute = picture();
        absolute.put(FILE, crate::manifest::tests::PICTURE.replace("\"harbour.png\"", "\"C:/harbour.png\"").as_bytes());
        assert!(says(&absolute.problems(), "is not relative"));
        let missing = picture();
        missing.put(FILE, crate::manifest::tests::PICTURE.replace("harbour.png", "dawn.png").as_bytes());
        let problems = missing.problems();
        assert!(says(&problems, "is named but missing"));
        assert!(problems.iter().any(|p| p.at == "harbour.png" && p.what.contains("is not named by wallpaper.toml")));
    }

    #[test]
    fn a_file_left_unnamed_or_hidden_is_refused() {
        let folder = picture();
        folder.put("notes.txt", b"draft");
        folder.put(".DS_Store", b"");
        let problems = folder.problems();
        assert!(problems.iter().any(|p| p.at == "notes.txt" && p.what.contains("is not named")));
        assert!(problems.iter().any(|p| p.at == ".DS_Store" && p.what.contains("is hidden")));
    }

    #[test]
    fn an_image_must_be_the_kind_its_name_says_and_of_its_size() {
        let folder = picture();
        folder.put("thumb.png", &jpeg(480, 270));
        folder.put("harbour.png", &png(4000, 2250));
        let problems = folder.problems();
        assert!(problems.iter().any(|p| p.at == "thumb.png" && p.what.contains("not the kind its name says")));
        assert!(problems.iter().any(|p| p.at == "harbour.png" && p.what.contains("over the 3840 x 2160")));
        let thumb = picture();
        thumb.put("thumb.png", &png(640, 360));
        assert!(says(&thumb.problems(), "where a thumbnail is 480 x 270"));
        let gif = picture();
        gif.put(FILE, crate::manifest::tests::PICTURE.replace("harbour.png", "harbour.gif").as_bytes());
        gif.put("harbour.gif", b"GIF89a");
        assert!(says(&gif.problems(), "is not a .png, .jpg or .webp"));
    }

    #[test]
    fn a_folder_over_its_total_is_refused() {
        let folder = picture();
        let small = Limits { total: 100, ..Limits::STORE };
        assert!(says(&check(&folder.0, &small).1, "bytes in all, over the 100 allowed"));
    }

    #[test]
    fn each_kind_holds_only_its_own_parts() {
        let folder = picture();
        let text = format!("{}\n[[layer]]\nimage = \"harbour.png\"\n", crate::manifest::tests::PICTURE);
        folder.put(FILE, text.as_bytes());
        assert!(says(&folder.problems(), "a picture with a scene's layers"));
        let scene = scene();
        let start = crate::manifest::tests::SCENE.find("[[layer]]").expect("the scene has layers");
        let end = crate::manifest::tests::SCENE.find("[[emitter]]").expect("the scene has an emitter");
        let text = format!("{}{}", &crate::manifest::tests::SCENE[..start], &crate::manifest::tests::SCENE[end..]);
        scene.put(FILE, text.as_bytes());
        assert!(says(&scene.problems(), "a scene with no [[layer]]"));
    }

    #[test]
    fn numbers_out_of_bounds_are_refused() {
        let folder = scene();
        let text = crate::manifest::tests::SCENE
            .replace("count = 160", "count = 0")
            .replace("gravity = 30", "gravity = nan")
            .replace("size = [0.6, 1.0]", "size = [1.0, 0.6]")
            .replace("hover = 1.2", "hover = 3");
        folder.put(FILE, text.as_bytes());
        let problems = folder.problems();
        assert!(says(&problems, "a count of 0"));
        assert!(says(&problems, "gravity NaN is not"));
        assert!(says(&problems, "size [1, 0.6] is not a range"));
        assert!(says(&problems, "hover 3 is not 1 to 2"));
    }

    #[test]
    fn a_script_needs_its_source_and_holds_only_source() {
        let folder = scene();
        let text = format!("{}\n[script]\nsource = \"script\"\nfuel = 2000000\nmemory = 4\n", crate::manifest::tests::SCENE);
        folder.put(FILE, text.as_bytes());
        let problems = folder.problems();
        assert!(problems.iter().any(|p| p.at == "script/Cargo.toml" && p.what.contains("is missing")));
        folder.put("script/Cargo.toml", SCRIPT_CRATE.as_bytes());
        folder.put("script/src/lib.rs", b"#[no_mangle]\npub extern \"C\" fn frame(_dt: f32) {}\n");
        assert_eq!(folder.problems(), vec![]);
        folder.put("script/README.md", b"# Leaf Pile's wind\n");
        folder.put("script/src/wind.rs", b"");
        assert_eq!(folder.problems(), vec![]);
        folder.put("script/build.exe", b"MZ");
        folder.put("script/rust-toolchain.toml", b"[toolchain]\npath = \"/tmp/rust\"\n");
        folder.put("script/tools/Cargo.toml", b"[package]\n");
        let problems = folder.problems();
        for stray in ["script/build.exe", "script/rust-toolchain.toml", "script/tools/Cargo.toml"] {
            assert!(problems.iter().any(|p| p.at == stray && p.what.contains("is not the crate's Cargo.toml")), "{stray}: {problems:?}");
        }
        let greedy = format!("{}\n[script]\nsource = \"script\"\nfuel = 20000000\nmemory = 64\n", crate::manifest::tests::SCENE);
        folder.put(FILE, greedy.as_bytes());
        let problems = folder.problems();
        assert!(says(&problems, "fuel 20000000 is not"));
        assert!(says(&problems, "memory 64 is not"));
    }

    const SCRIPT_CRATE: &str = "[package]\nname = \"leaf-pile-script\"\nversion = \"1.0.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[profile.release]\nopt-level = \"s\"\n";

    #[test]
    fn a_scripts_crate_may_not_build_depend_or_make_a_macro() {
        let folder = scene();
        let text = format!("{}\n[script]\nsource = \"script\"\nfuel = 2000000\nmemory = 4\n", crate::manifest::tests::SCENE);
        folder.put(FILE, text.as_bytes());
        folder.put("script/src/lib.rs", b"");
        let crate_with = |extra: &str| format!("{SCRIPT_CRATE}{extra}");
        folder.put("script/Cargo.toml", crate_with("\n[dependencies]\nserde = \"1\"\n").as_bytes());
        assert!(says(&folder.problems(), "holds [dependencies]"));
        folder.put("script/Cargo.toml", crate_with("\n[build-dependencies]\ncc = \"1\"\n").as_bytes());
        assert!(says(&folder.problems(), "holds [build-dependencies]"));
        folder.put("script/Cargo.toml", SCRIPT_CRATE.replace("edition = \"2021\"", "edition = \"2021\"\nbuild = \"make.rs\"").as_bytes());
        assert!(says(&folder.problems(), "names a build in [package]"));
        folder.put("script/Cargo.toml", SCRIPT_CRATE.replace("crate-type = [\"cdylib\"]", "proc-macro = true").as_bytes());
        let problems = folder.problems();
        assert!(says(&problems, "makes a proc macro"));
        assert!(says(&problems, "has no [lib] crate-type"));
        folder.put("script/Cargo.toml", SCRIPT_CRATE.as_bytes());
        folder.put("script/build.rs", b"fn main() {}\n");
        assert!(folder.problems().iter().any(|p| p.at == "script/build.rs" && p.what.contains("is a build script")));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_refused() {
        let folder = picture();
        std::os::unix::fs::symlink("/etc/passwd", folder.0.join("passwd")).expect("a link");
        assert!(folder.problems().iter().any(|p| p.at == "passwd" && p.what.contains("is a link")));
    }

    #[test]
    fn names_ids_tags_and_versions_read_as_they_should() {
        assert!(named("images/sky.png").is_ok());
        assert!(named("images/../sky.png").is_err());
        assert!(named("images//sky.png").is_err());
        assert!(named("images\\sky.png").is_err());
        assert!(is_id("leaf-pile") && !is_id("leaf--pile") && !is_id("-leaf") && !is_id("lp"));
        assert!(is_tag("new-year") && !is_tag("x") && !is_tag("New"));
        assert!(is_version("1.0.0") && is_version("10.20.30") && !is_version("1.0") && !is_version("01.0.0") && !is_version("1.0.0-beta"));
    }
}
