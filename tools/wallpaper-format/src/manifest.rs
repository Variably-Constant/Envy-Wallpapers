//! A wallpaper's `wallpaper.toml`: what it is, who made it and under which license, how it plays, and what it shows.
//!
//! Positions and sizes are in the pixels of a 1920 x 1080 frame, x to the right and y downward, whatever the TV's own
//! size; times are in seconds and angles in degrees. Every table refuses keys it does not know, so a misspelt key is an
//! error when the wallpaper is checked, not a setting silently left out.

use serde::{Deserialize, Serialize};

/// The format of `wallpaper.toml` this crate reads.
pub const FORMAT: u32 = 1;

/// The name of a wallpaper's manifest in its folder.
pub const FILE: &str = "wallpaper.toml";

/// A wallpaper as its `wallpaper.toml` describes it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The format the file is written in, [`FORMAT`].
    pub format: u32,
    /// Lowercase letters, digits and single hyphens, 3 to 40 of them, the same as the folder's name.
    pub id: String,
    /// The name the store shows, up to 40 characters.
    pub title: String,
    /// `major.minor.patch`; a newer version replaces an older one on the TV.
    pub version: String,
    pub kind: Kind,
    /// Who made it, as the store credits them, up to 60 characters.
    pub author: String,
    pub license: License,
    /// What the store says of it, up to 300 characters.
    #[serde(default)]
    pub description: String,
    /// Up to 8 words the store files it under: lowercase letters, digits and hyphens, 2 to 20 each.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The ways it answers the viewer, as the store lists them.
    #[serde(default)]
    pub plays: Vec<Play>,
    /// The season the Seasons wallpaper shows it in.
    pub season: Option<Season>,
    /// The holiday the Seasons wallpaper shows it for.
    pub holiday: Option<Holiday>,
    /// The 480 x 270 image the store shows for it.
    pub thumbnail: String,
    /// What a picture shows.
    pub picture: Option<Picture>,
    /// A scene's images, back to front.
    #[serde(default, rename = "layer")]
    pub layers: Vec<Layer>,
    /// A scene's swarms of small pieces: leaves, snow, fireflies.
    #[serde(default, rename = "emitter")]
    pub emitters: Vec<Emitter>,
    /// A scene's pieces that move on their own: a squirrel, a train.
    #[serde(default, rename = "actor")]
    pub actors: Vec<Actor>,
    /// A scene's pieces the viewer plays with: a pumpkin, an envelope.
    #[serde(default, rename = "object")]
    pub objects: Vec<Object>,
    /// A scene's script, for behavior the pieces above lack.
    pub script: Option<Script>,
}

impl Manifest {
    /// Reads `text` as a `wallpaper.toml`.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// Every image the wallpaper names, with what it is for, in the order the file names them.
    pub fn images(&self) -> Vec<(&str, Use)> {
        let mut out = vec![(self.thumbnail.as_str(), Use::Thumbnail)];
        if let Some(picture) = &self.picture {
            out.push((picture.image.as_str(), Use::Still));
        }
        out.extend(self.layers.iter().map(|layer| (layer.image.as_str(), Use::Still)));
        for emitter in &self.emitters {
            out.extend(emitter.sprites.iter().map(|sprite| (sprite.as_str(), Use::Sprite)));
        }
        for actor in &self.actors {
            out.extend(actor.frames.iter().map(|frame| (frame.as_str(), Use::Sprite)));
        }
        for object in &self.objects {
            out.extend(object.states.iter().map(|state| (state.as_str(), Use::Sprite)));
        }
        out
    }
}

/// What an image is for, which sets the size it may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// The store's thumbnail, of exactly the thumbnail's size.
    Thumbnail,
    /// A picture or a layer, no larger than a still may be.
    Still,
    /// A piece of a swarm, a frame of an actor or a state of an object.
    Sprite,
}

/// What a wallpaper is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// One image.
    Picture,
    /// Layers, swarms, actors and objects that move and answer the viewer.
    Scene,
}

/// The licenses the store takes. Each lets it share the wallpaper freely; the two BY licenses keep the author's credit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum License {
    #[serde(rename = "CC0-1.0")]
    Cc0,
    #[serde(rename = "CC-BY-4.0")]
    CcBy,
    #[serde(rename = "CC-BY-SA-4.0")]
    CcBySa,
}

/// A way a wallpaper answers the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Play {
    /// Something follows or flees the pointer.
    Pointer,
    /// The pointer's movement is a wind.
    PointerWind,
    /// Something follows the tile selected in the Bar.
    Selection,
    /// A click does something where it lands.
    Click,
    /// Holding the click does something, and letting go does another.
    Hold,
    /// Pieces of the scene answer when pointed at or clicked.
    Objects,
    /// It follows the TV's backlight.
    Backlight,
    /// It follows the time of day or the date.
    Time,
}

/// A season, as the Seasons wallpaper picks by the date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Autumn,
}

/// A holiday, as the Seasons wallpaper picks by the date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Holiday {
    Halloween,
    WinterHoliday,
    NewYear,
    Valentines,
}

/// What a picture shows: one image, with a slow zoom into it or a lean toward the pointer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Picture {
    pub image: String,
    pub pan: Option<Pan>,
    /// How far, in pixels, the picture leans toward the pointer.
    #[serde(default)]
    pub parallax: f64,
}

/// A slow zoom into the picture and back out, once each `seconds`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pan {
    /// How far it zooms in, from 1 (not at all) to 1.25.
    pub zoom: f64,
    pub seconds: f64,
}

/// What a piece takes its lead from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// The pointer, while it is over the wallpaper.
    Pointer,
    /// The tile selected in the Bar.
    Selection,
    /// The pointer while it leads, else the selected tile, as Steel Eye's gaze has it.
    Gaze,
}

/// An image of a scene, laid with its top left at `at`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub image: String,
    #[serde(default)]
    pub at: [f64; 2],
    pub sway: Option<Sway>,
    /// How far, in pixels, the layer leans toward the gaze; a deeper layer leans less.
    #[serde(default)]
    pub parallax: f64,
    /// Pixels a second it drifts by, across and down, wrapping round the frame.
    #[serde(default)]
    pub drift: [f64; 2],
}

/// A swing to and fro about `pivot`, `degrees` each way, once each `seconds`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sway {
    pub degrees: f64,
    pub seconds: f64,
    pub pivot: [f64; 2],
}

/// A swarm of `count` small pieces, each drawn from one of `sprites`, started in `area` (x, y, width and height).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emitter {
    pub sprites: Vec<String>,
    pub area: [f64; 4],
    pub count: u32,
    /// Seconds each piece lives before another starts in its place, between the two; without it every piece lives on.
    pub life: Option<[f64; 2]>,
    /// The speed each piece starts with, in pixels a second: across between the first two, down between the last two.
    #[serde(default)]
    pub speed: [f64; 4],
    /// Pixels a second gained each second, downward.
    #[serde(default)]
    pub gravity: f64,
    /// Degrees a second each piece turns, between the two.
    #[serde(default)]
    pub spin: [f64; 2],
    /// The scale each piece is drawn at, between the two.
    #[serde(default = "full_size")]
    pub size: [f64; 2],
    pub twinkle: Option<Twinkle>,
    pub wind: Option<Wind>,
    pub attract: Option<Attract>,
    pub burst: Option<Burst>,
    pub sync: Option<Sync>,
}

fn full_size() -> [f64; 2] {
    [1.0, 1.0]
}

/// Each piece's alpha falls to `low` and rises back, once each `seconds`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Twinkle {
    pub seconds: f64,
    pub low: f64,
}

/// A wind from the pointer's movement: the pieces within `reach` pixels of it are pushed by `strength` times its speed.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wind {
    pub strength: f64,
    pub reach: f64,
}

/// A pull of the pieces toward `to` by `strength`, gathering them within `radius` pixels of it.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attract {
    pub to: Target,
    pub strength: f64,
    pub radius: f64,
}

/// `count` more pieces flung out at `speed` pixels a second where a click lands.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Burst {
    pub count: u32,
    pub speed: f64,
}

/// Flashing together, as fireflies do: each piece pulses once each `seconds`, nudged toward its neighbours' timing by
/// `coupling`, and by `held` while a click is held.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sync {
    pub seconds: f64,
    pub coupling: f64,
    pub held: f64,
}

/// A piece that moves on its own: along `path`, after a target, away from the pointer, or turning toward a target.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    /// Its frames, played at `fps`; with one frame it stands as it is.
    pub frames: Vec<String>,
    #[serde(default = "twelve")]
    pub fps: f64,
    pub at: [f64; 2],
    /// Points it travels through in turn, at `speed` pixels a second.
    #[serde(default)]
    pub path: Vec<[f64; 2]>,
    #[serde(default)]
    pub speed: f64,
    pub follow: Option<Follow>,
    pub flee: Option<Flee>,
    pub gaze: Option<Gaze>,
}

fn twelve() -> f64 {
    12.0
}

/// Goes after `target` at up to `speed` pixels a second; an actor on a path stops at the point of it nearest the target.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    pub target: Target,
    pub speed: f64,
}

/// Runs from the pointer when it comes within `reach` pixels, and comes back `back_after` seconds after it has gone.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flee {
    pub reach: f64,
    pub back_after: f64,
}

/// Turns up to `degrees` toward `target`, about `pivot`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gaze {
    pub target: Target,
    pub degrees: f64,
    pub pivot: [f64; 2],
}

/// A piece the viewer plays with, centred on `at`, answering within `radius` pixels of it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Object {
    /// Its states' images, in order; a click moves it to the next, and from the last back to the first.
    pub states: Vec<String>,
    pub at: [f64; 2],
    pub radius: f64,
    /// How much it grows while the pointer is over it, 1 being not at all.
    #[serde(default = "one")]
    pub hover: f64,
    /// Seconds a held click takes to move it to the next state, when holding is what moves it.
    pub hold: Option<f64>,
}

fn one() -> f64 {
    1.0
}

/// A scene's script: the Rust crate in the folder `source`, built to WebAssembly, runs each frame within `fuel`
/// instructions and `memory` megabytes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    pub source: String,
    pub fuel: u64,
    pub memory: u32,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const PICTURE: &str = r#"
format = 1
id = "misty-harbour"
title = "Misty Harbour"
version = "1.0.0"
kind = "picture"
author = "A. Creator"
license = "CC-BY-4.0"
description = "Boats at dawn."
tags = ["water", "calm"]
plays = ["pointer"]
thumbnail = "thumb.png"

[picture]
image = "harbour.png"
pan = { zoom = 1.06, seconds = 90 }
parallax = 12
"#;

    pub(crate) const SCENE: &str = r#"
format = 1
id = "leaf-pile"
title = "Leaf Pile"
version = "1.0.0"
kind = "scene"
author = "Envy"
license = "CC-BY-SA-4.0"
tags = ["seasonal", "autumn"]
plays = ["pointer-wind", "objects"]
season = "autumn"
thumbnail = "thumb.png"

[[layer]]
image = "images/sky.png"

[[layer]]
image = "images/tree.png"
at = [1100, 120]
sway = { degrees = 0.6, seconds = 7, pivot = [1400, 1080] }

[[emitter]]
sprites = ["images/maple.png", "images/oak.png"]
area = [0, 820, 1920, 260]
count = 160
gravity = 30
spin = [-90, 90]
size = [0.6, 1.0]
wind = { strength = 1.4, reach = 240 }

[[actor]]
frames = ["images/squirrel-1.png", "images/squirrel-2.png"]
at = [1310, 470]
flee = { reach = 220, back_after = 6 }

[[object]]
states = ["images/acorn.png", "images/acorn-open.png"]
at = [600, 900]
radius = 40
hover = 1.2
"#;

    #[test]
    fn reads_a_picture_and_a_scene() {
        let picture = Manifest::parse(PICTURE).expect("the picture reads");
        assert_eq!(picture.kind, Kind::Picture);
        assert_eq!(picture.license, License::CcBy);
        assert_eq!(picture.picture.as_ref().and_then(|p| p.pan), Some(Pan { zoom: 1.06, seconds: 90.0 }));
        let scene = Manifest::parse(SCENE).expect("the scene reads");
        assert_eq!(scene.kind, Kind::Scene);
        assert_eq!(scene.season, Some(Season::Autumn));
        assert_eq!(scene.plays, vec![Play::PointerWind, Play::Objects]);
        assert_eq!(scene.layers.len(), 2);
        assert_eq!(scene.layers[0].at, [0.0, 0.0]);
        assert_eq!(scene.emitters[0].wind, Some(Wind { strength: 1.4, reach: 240.0 }));
        assert_eq!(scene.emitters[0].life, None);
        assert_eq!(scene.actors[0].fps, 12.0);
        assert_eq!(scene.objects[0].hover, 1.2);
    }

    #[test]
    fn refuses_an_unknown_key_and_an_unknown_license() {
        let misspelt = SCENE.replace("gravity = 30", "gravty = 30");
        let error = Manifest::parse(&misspelt).expect_err("a misspelt key is refused");
        assert!(error.contains("gravty"), "{error}");
        let license = PICTURE.replace("CC-BY-4.0", "CC-BY-NC-4.0");
        assert!(Manifest::parse(&license).is_err());
    }

    #[test]
    fn lists_every_image_it_names_with_its_use() {
        let scene = Manifest::parse(SCENE).expect("the scene reads");
        let images = scene.images();
        assert_eq!(images[0], ("thumb.png", Use::Thumbnail));
        assert_eq!(images[1], ("images/sky.png", Use::Still));
        assert_eq!(images[2], ("images/tree.png", Use::Still));
        assert_eq!(images[3], ("images/maple.png", Use::Sprite));
        assert_eq!(images.len(), 9);
        assert_eq!(images[8], ("images/acorn-open.png", Use::Sprite));
    }
}
