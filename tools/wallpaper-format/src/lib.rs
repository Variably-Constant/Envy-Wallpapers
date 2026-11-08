//! The format of a wallpaper Envy downloads, and what checks, packs and lists it.
//!
//! A wallpaper is a folder named by its id. It holds `wallpaper.toml` ([`manifest`]), a 480 x 270 thumbnail, the images
//! it shows and, for a scene with behavior of its own, the source of its WebAssembly script. [`check`] says whether a
//! folder is fit to publish. [`pack`] makes the package the TV downloads: a tar of the folder's files in a fixed order,
//! with the script built. [`catalog`] lists every wallpaper with its package's size and SHA-256, in a file the store's
//! repository signs with minisign so the TV can tell the list came from it.
//!
//! Nothing a wallpaper holds runs on the TV but its script, and that runs in a metered sandbox; the rest is data.

pub mod catalog;
pub mod check;
pub mod manifest;
pub mod pack;

pub use manifest::Manifest;
