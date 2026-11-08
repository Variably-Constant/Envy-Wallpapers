# Envy Wallpapers

Wallpapers for Envy, a launcher for LG webOS TVs. Envy's Store lists every wallpaper in this repository and downloads the
ones you choose, and the TV checks each against this repository's signed catalog before it shows it.

<details open>
<summary><b>Features</b></summary>

- Pictures, live scenes, and live scenes with a sandboxed WebAssembly script.
- Scenes can answer the pointer, the tile selected in Envy's Bar, a click, a held click, the TV's backlight and the time
  of day.
- Each wallpaper is one folder: a `wallpaper.toml`, a 480 x 270 thumbnail and its images.
- `envy-wallpaper check` says whether a folder is fit to publish, with every problem it finds.
- Every package is listed in a catalog signed with minisign, and the TV checks the signature and each package's SHA-256.
- CC0, CC BY or CC BY-SA, with the author credited in the Store.

</details>

## Quick start

```sh
git clone https://github.com/Variably-Constant/Envy-Wallpapers
cd Envy-Wallpapers
cargo run --release --manifest-path tools/wallpaper-format/Cargo.toml -- check wallpapers/misty-harbour
```

A picture wallpaper is a folder named by its id:

```
wallpapers/misty-harbour/
  wallpaper.toml
  thumb.jpg        480 x 270
  harbour.jpg      up to 3840 x 2160
```

```toml
format = 1
id = "misty-harbour"
title = "Misty Harbour"
version = "1.0.0"
kind = "picture"
author = "Your Name"
license = "CC-BY-4.0"
description = "Boats at dawn."
tags = ["water", "calm"]
thumbnail = "thumb.jpg"

[picture]
image = "harbour.jpg"
pan = { zoom = 1.06, seconds = 90 }
```

Check it, then open a pull request that adds the folder. [CONTRIBUTING.md](CONTRIBUTING.md) has the rules a wallpaper
follows, and [docs/FORMAT.md](docs/FORMAT.md) every key `wallpaper.toml` takes.

## How a wallpaper reaches a TV

1. A pull request adds or changes a folder under `wallpapers/`. The check workflow runs `envy-wallpaper check` on it, and
   builds its script from source if it has one.
2. A maintainer of Variably-Constant reviews and merges it.
3. The publish workflow packs each new version as `<id>-<version>.tar` on a release tagged `<id>-v<version>`. It lists
   every wallpaper in `catalog.json` with its package's size and SHA-256, signs the catalog with minisign, and publishes it
   with the thumbnails on this repository's pages.
4. Envy's Store fetches the catalog and checks its signature against the key built into Envy. It downloads the package you
   choose, checks its SHA-256, and only then unpacks it.

A wallpaper holds data and no code, except a scene's script. The TV runs that in a sandbox that meters every instruction
and caps its memory. A script's crate may not have dependencies, a build script or a toolchain of its own, so building it
runs nothing but the compiler, and it builds in a job that never sees the signing key.

## The catalog's key

Every `catalog.json` is signed with this minisign public key, which Envy builds in:

```
RWSNX6wxMVtn63VcGylBso9QH4KjGg4TwJ9d9bsRhWjhLoeg+4eNcze7
```

Check a copy of the catalog with `minisign -Vm catalog.json -P <key>`, or with
`envy-wallpaper verify catalog.json catalog.json.minisig <key>`.

## Licenses

Each wallpaper names its license in `wallpaper.toml`: `CC0-1.0`, `CC-BY-4.0` or `CC-BY-SA-4.0`. The tools in `tools/` are
MIT ([LICENSE](LICENSE)).

## Repository layout

| Path | What it holds |
|---|---|
| `wallpapers/<id>/` | One wallpaper each |
| `tools/wallpaper-format/` | The format's Rust crate and the `envy-wallpaper` tool |
| `docs/FORMAT.md` | Every key of `wallpaper.toml` |
| `.github/workflows/check.yml` | The checks every pull request runs |
| `.github/workflows/publish.yml` | Releases, the signed catalog and the pages, on every merge |
