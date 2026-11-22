# Design notes

More notes are coming in Milestone 10. This page lists, for now, **where the implementation
differs from the original specification, and why**, so nobody has to rediscover it.

## Where the implementation differs from the original spec, and why

"Spec" is the original specification text. "M4" means the implementation value before this
clean-up where the spec says nothing. Every change below has a reason that a test enforces
(the text-contrast hard check, the brightness limits, the visibility numbers) or that a
reviewer saw in a picture.

### Colors and themes (`theme.rs`)

| Item | Original | Now | Why |
|---|---|---|---|
| dusk `sky_bottom` | `#b25f7a` (spec) | `#7d4066` | Light text must reach 4.5:1 on what is really behind it (sky, glow, nebula and band together). On the spec's rose it reached 2.5 to 3.9. An earlier nudge to `#a45770` only fixed the sky color alone. Same hue, deeper. |
| dusk horizon glow | `#f0a070` at 0.35 (M4) | `#e8807f` at 0.22 | Same contrast reason: a bright orange glow under the labels made light text unreadable. |
| dusk `text_dim` | `#e6c3d2` (M4) | `#f9dfeb` | Contrast of the "first light" and caption notes. |
| sakura `text_dim` | `#e6bcd3` (M4) | `#f8d8e8` | Same. |
| paper `text_dim` | `#6e6788` (M4) | `#4f4968` | Contrast of dim notes on cream. |
| mono-dark `sky_bottom` and its (zero-strength) glow color | `#1c1c1c` (spec: near black) | `#242424` | With the softer Milky Way the empty sky averaged 0.076 luma, under the 0.08 floor for dark themes. |
| `accent` color per theme | none | warm yellow `#ffe08a` on the four colored dark themes, ink on paper, text or ink on mono | The HUD sparkle needs a theme color; no colors live outside `theme.rs`. |
| paper watercolor blobs | `#c9b8e8`, `#f3c6b8`, `#b8d8e8` at 0.35 | same colors at 0.45 | At 0.35 they barely registered on cream. |

### Milky Way (`art/milkyway.rs`, per-theme `milkyway_strength`)

| Item | Original | Now | Why |
|---|---|---|---|
| cloud width | 2.8 segment lengths (M4) | 1.6 | The band read as one smeared stroke; busy and quiet weeks looked the same. |
| cloud opacity base / span | 0.04 / 0.06 (M4) | 0.02 / 0.07 | Busiest week is at least 2.5 times as opaque as the quietest. |
| band dots base / span | 3 / 13 (M4) | 2 / 20 | Busiest week has at least 3 times as many dots as the quietest. |
| per-theme strength | midnight 1.0, dusk 1.3, aurora 1.0, sakura 1.2, paper 1.7, mono-dark 0.7, mono-light 1.5 (M4) | 0.7, 1.5, 0.75, 1.0, 3.0, 0.7, 2.5 | Tested limit: on dark themes the band's mean luma is at most 1.35 times that of the sky's lightest color. Paper and mono-light needed more to register on a light page. |
| dust lanes | none | 6 dark ellipses along the band | Gives the band texture. |
| paper fibers | none | 4 faint ink dashes (paper only) | A touch of paper grain without a filter. |

### Stars (`art/stars.rs`, `art/star_parts.rs`)

| Item | Original | Now | Why |
|---|---|---|---|
| sparkle tip / waist | 1.9 r / 0.18 r (spec) | 2.1 r / 0.2 r | With the spec's numbers the short diagonal points came out at 0.8 r, not the spec's own 0.9 r. 2.1 / 0.2 gives 0.88 r. |
| halo falloff | two stops | three stops (at 0.45 of the radius, 0.4 of the opacity) | A straight fade looked like a cone. |
| founding ring | 2.6 r solid, 0.8 px, opacity 0.5 | 3.0 r solid, 0.8 px, opacity 0.55 | Breathing room beyond the sparkle tips (2.1 r). One constant (`FOUNDING_RING`) shared by art, layout and linter. |
| fresh ring (static) | 2.0 r solid (M4) | 2.5 r dotted `1.2 2.4`, 0.7 px, opacity 0.35 | Must look different from the founding ring. A star that is both draws only the founding ring. |
| line gap at the founding star | 2.4 r like the others, then 2.8 r | 3.3 r | Lines must stop outside the 3.0 r ring. |
| flare cross | 1 px, opacity 0.5, 5 r long | 1.3 px, opacity 0.85, 5 r long; arms stop 3 px before any drawn label | The old flare was invisible at scale 2. A hard check keeps flares off labels. |
| paper flare | not specified | two ink strokes, 0.6 px, opacity 0.7, drawn behind the body | A crosshair over the body looked wrong. |
| paper star ink | "darker 18 percent mixes" | 18 percent for tier 3 to 5, 30 percent plus a 0.7 px outline for tier 1 and 2, grayed ink (60 percent toward ink, opacity 0.55) for dormant stars | Pale colors (yellow, mint) vanished on cream. |
| tier share caps | none (only score thresholds) | tier 5 at most 8 percent, tier 4 at most 20 percent of the stars (`rules.rs`) | Keeps a sky from being mostly flagship stars. |

### Labels and the HUD

| Item | Original | Now | Why |
|---|---|---|---|
| constellation name opacity | 0.70 | 0.90 | Text-contrast hard check (at least 4.5:1). |
| repo name opacity | 0.85 | 0.95 | Same. |
| dim notes ("first light", caption) opacity | 0.85 (dim color) | 0.95 | Same. |
| "first light" size | unspecified; implemented at 9 px | 10.5 px | 9 px was under the 10 px readability floor. |
| HUD sparkle and title position | sparkle at (40, 44) | sparkle at (54, 44), title at (76, 50) | The title sits right of the icon. |
| HUD stats line | y = 62 | y = 68 | Room under the 15 px title. |
| HUD username | not limited | at most 24 characters and short enough to end left of x = 326 | A 39-character name ran out of the reserved HUD box. |
| `--labels none` | not specified | draws no label at all, including "first light" and the caption | Only the HUD remains. |

### Linter

| Item | Original | Now | Why |
|---|---|---|---|
| star visibility on paper | center 3 x 3 block, darkest pixel | darkest pixel of the star's whole body disc | The pale core sits in the middle of a paper star; what makes it visible is its inked body. On dark themes the 3 x 3 block still applies. |
| new hard checks | none | labels clear of rings, flares clear of labels, text contrast (at least 4.5:1), SVG size, finite numbers, references, unique ids | Found by review. |
| new soft checks | none | sparse-sky composition, star visibility, lines too short, labels left out | Same. |

## Known caveats

- **PNG text depends on the machine's fonts.** The SVG is deterministic; a PNG draws text with
  whatever system fonts exist, so label glyph shapes differ a little between machines. Tests
  never compare PNG bytes or text pixel positions. CI installs `fonts-dejavu-core` and
  `fonts-liberation` so labels render at all (this workflow step has not been run by the author).
- **Unused dependencies.** `tiny_http` and `ureq` are declared but unused until Milestones 8
  (fetching) and 9 (live preview). Cargo warns about them; that is expected.
- **Review images** are written to `target/previews/` (or `PREVIEW_DIR`) and are not committed.
  Only `docs/preview/sky-medium.png` and `docs/preview/sky-paper.png` and the generated
  `docs/preview/lint-report.txt` live in the repository.
