# wallpaper.toml, format 1

Every wallpaper's folder holds a `wallpaper.toml`. Positions and sizes are in the pixels of a 1920 x 1080 frame, x to the
right and y downward, whatever the TV's own size. Times are in seconds and angles in degrees. Every table refuses keys it
does not know, so a misspelt key is an error when the folder is checked. Points may lie up to 7680 pixels outside the
frame. NaN and infinity are refused everywhere.

## The header

| Key | Needed | What |
|---|---|---|
| `format` | yes | `1` |
| `id` | yes | 3 to 40 lowercase letters, digits and single hyphens, and the folder's name |
| `title` | yes | 1 to 40 characters |
| `version` | yes | `major.minor.patch` |
| `kind` | yes | `"picture"` or `"scene"` |
| `author` | yes | 1 to 60 characters, as the Store credits them |
| `license` | yes | `"CC0-1.0"`, `"CC-BY-4.0"` or `"CC-BY-SA-4.0"` |
| `description` | no | Up to 300 characters |
| `tags` | no | Up to 8, each 2 to 20 lowercase letters, digits and hyphens |
| `plays` | no | How it answers the viewer: `"pointer"`, `"pointer-wind"`, `"selection"`, `"click"`, `"hold"`, `"objects"`, `"backlight"`, `"time"` |
| `season` | no | `"winter"`, `"spring"`, `"summer"` or `"autumn"`, for the Seasons wallpaper |
| `holiday` | no | `"halloween"`, `"winter-holiday"`, `"new-year"` or `"valentines"`, for the Seasons wallpaper |
| `thumbnail` | yes | The 480 x 270 image |

## A picture: `[picture]`

| Key | Needed | What |
|---|---|---|
| `image` | yes | The image, at most 3840 x 2160 |
| `pan` | no | `{ zoom, seconds }`: a slow zoom in to `zoom` (1 to 1.25) and back, once each `seconds` (5 to 3600) |
| `parallax` | no | Pixels it leans toward the pointer, 0 to 120 |

A picture has no layers, emitters, actors, objects or script.

## A scene

A scene has at least one `[[layer]]`, and no `[picture]`. Where a piece takes its lead from a target, the target is
`"pointer"` (the pointer, while it is over the wallpaper), `"selection"` (the tile selected in Envy's Bar) or `"gaze"`
(the pointer while it leads, else the selected tile).

### `[[layer]]`: an image, back to front

| Key | Needed | What |
|---|---|---|
| `image` | yes | The image, at most 3840 x 2160 |
| `at` | no | Its top left, `[x, y]` |
| `sway` | no | `{ degrees, seconds, pivot }`: a swing of up to 45 degrees each way about `pivot`, once each `seconds` (0.5 to 600) |
| `parallax` | no | Pixels it leans toward the gaze, 0 to 120; a deeper layer leans less |
| `drift` | no | `[x, y]` pixels a second it drifts by, wrapping round the frame |

### `[[emitter]]`: a swarm of small pieces

| Key | Needed | What |
|---|---|---|
| `sprites` | yes | The images its pieces are drawn from |
| `area` | yes | `[x, y, width, height]` where pieces start |
| `count` | yes | How many pieces there are at once, at least 1 |
| `life` | no | `[lo, hi]` seconds each piece lives before another starts in its place (0.1 to 600); without it pieces live on |
| `speed` | no | `[x lo, x hi, y lo, y hi]` pixels a second each starts with, at most 4000 either way |
| `gravity` | no | Pixels a second gained each second, downward, -4000 to 4000 |
| `spin` | no | `[lo, hi]` degrees a second each turns, -1440 to 1440 |
| `size` | no | `[lo, hi]` scale each is drawn at, 0.05 to 8; `[1, 1]` when left out |
| `twinkle` | no | `{ seconds, low }`: alpha falls to `low` (0 to 1) and back, once each `seconds` |
| `wind` | no | `{ strength, reach }`: the pointer's movement pushes pieces within `reach` pixels by `strength` (0 to 10) times its speed |
| `attract` | no | `{ to, strength, radius }`: a pull toward a target, gathering pieces within `radius` pixels of it |
| `burst` | no | `{ count, speed }`: `count` more pieces flung out at `speed` where a click lands |
| `sync` | no | `{ seconds, coupling, held }`: pieces pulse once each `seconds`, nudged toward their neighbors' timing by `coupling` (0 to 1), and by `held` while a click is held, as fireflies fall into step |

All the emitters together have at most 2000 pieces, bursts included.

### `[[actor]]`: a piece that moves on its own

| Key | Needed | What |
|---|---|---|
| `frames` | yes | Its frames, played at `fps`; with one frame it stands as it is |
| `fps` | no | 0.5 to 60; 12 when left out |
| `at` | yes | Where it starts, `[x, y]` |
| `path` | no | `[[x, y], ...]` points it travels through in turn |
| `speed` | with a path | Pixels a second along its path, up to 4000 |
| `follow` | no | `{ target, speed }`: goes after a target; on a path, it stops at the point nearest the target |
| `flee` | no | `{ reach, back_after }`: runs from the pointer within `reach` pixels, back `back_after` seconds after it has gone |
| `gaze` | no | `{ target, degrees, pivot }`: turns up to `degrees` (0 to 180) toward a target about `pivot` |

### `[[object]]`: a piece the viewer plays with

| Key | Needed | What |
|---|---|---|
| `states` | yes | Its states' images; a click moves it to the next, and from the last back to the first |
| `at` | yes | Its middle, `[x, y]` |
| `radius` | yes | Pixels from its middle it answers within, 1 to 1080 |
| `hover` | no | How much it grows while the pointer is over it, 1 to 2; 1 when left out |
| `hold` | no | Seconds a held click takes to move it to the next state, 0.1 to 30, when holding is what moves it |

### `[script]`: behavior of a scene's own

| Key | Needed | What |
|---|---|---|
| `source` | yes | The folder of the script's Rust crate |
| `fuel` | yes | Instructions a frame may take, 1 to 10 000 000 |
| `memory` | yes | Megabytes it may grow to, 1 to 16 |

[CONTRIBUTING.md](../CONTRIBUTING.md) has what a script's crate may hold.

## Images

PNG, JPEG or WebP, of the kind the file's name says, at most 4096 pixels a side. Pictures and layers are at most 3840 x 2160,
and the thumbnail is exactly 480 x 270. A path is relative to the wallpaper's folder, uses `/` between its parts, and no part
is empty or starts with a dot.
