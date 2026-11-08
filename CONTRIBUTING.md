# Contributing a wallpaper

A wallpaper is a folder under `wallpapers/`, named by its id. This page says what it must hold and how it gets in.

## Steps

1. Fork this repository and add `wallpapers/<your-id>/` with `wallpaper.toml`, a 480 x 270 thumbnail and your images. The
   id is 3 to 40 lowercase letters, digits and single hyphens.
2. Run the checker until it says the folder is fit to publish:

   ```sh
   cargo run --release --manifest-path tools/wallpaper-format/Cargo.toml -- check wallpapers/<your-id>
   ```

3. Open a pull request. The check workflow runs the same checker, and a maintainer reviews the wallpaper itself.

To change a published wallpaper, raise its `version`; TVs replace the older version with it.

## What a wallpaper may be

- Your own work, or work you have the right to license as its `wallpaper.toml` says. Name its author as they wish to be
  credited.
- Licensed `CC0-1.0`, `CC-BY-4.0` or `CC-BY-SA-4.0`.
- Suited to a living room: nothing sexual, violent or hateful, and nothing more frightening than a friendly Halloween.
- Free of other people's characters, logos and trademarks.
- Light on the TV. It runs on a TV's GPU under Envy's Bar at 60 frames a second, so prefer a few large layers and modest
  swarms.

## Limits

| What | Limit |
|---|---|
| The folder | 32 MB in all |
| `wallpaper.toml` | 64 KB |
| Any image | PNG, JPEG or WebP, of the kind its name says, at most 4096 pixels a side |
| A picture or a layer | At most 3840 x 2160 |
| The thumbnail | Exactly 480 x 270 |
| Pieces in a scene's emitters | 2000 in all, bursts included |
| A script | 10 000 000 instructions a frame, 16 MB of memory |

Nothing else may lie in the folder: no file `wallpaper.toml` does not name, no hidden file, no link.

## Scripts

A scene may add a script for behavior its layers, emitters, actors and objects do not have. The script is a Rust crate in
the folder its `[script]` table names, and CI builds it to WebAssembly from your source; never commit a built `.wasm`.

- Its `Cargo.toml` holds only `[package]`, `[lib]` with `crate-type = ["cdylib"]`, and `[profile]`.
- It has no dependencies, no build script, no proc macro and no toolchain file.
- Its folder holds only `Cargo.toml`, `Cargo.lock`, Rust source under `src/`, and Markdown.

The interface a script exports, and the inputs it reads each frame, will be documented in `docs/SCRIPTS.md` with the first
scripted wallpaper.
