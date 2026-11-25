# UI & Motion Specification

This document is the **Single Source of Truth** for Fidra Island's visual design, dimensional geometry, motion curves, and Slint UI contracts. Any change here MUST match [`ui/main.slint`](../../ui/main.slint) and the hit-test constants in [`src/main.rs`](../../src/main.rs).

---

## 1. Dimensional Geometry (Logical Pixels)

| Element | Compact (Resting) | Compact (Hover) | Compact (Lyrics) | Expanded (Player) | Expanded (2-Col Lyrics) |
|---|---:|---:|---:|---:|---:|
| **Root Window** | `680 x 220` | `680 x 220` | `680 x 220` | `680 x 220` | `680 x 220` |
| **Island Width** | `290px` | `310px` | `280px – 420px` | `320px` | `610px` |
| **Island Height** | `38px` | `38px` | `38px` | `174px` (`175px` hit-test) | `174px` (`175px` hit-test) |
| **Bottom Radius** | `19px` | `19px` | `19px` | `26px` | `26px` |
| **Ear Flares (L/R)** | `14 x 14px` | `14 x 14px` | `14 x 14px` | `14 x 14px` | `14 x 14px` |

> [!IMPORTANT]
> The root `Window` dimensions (`680px x 220px`) are fixed forever. Never animate root window size.

---

## 2. Unified Obsidian Color Palette

| Token / Usage | Value |
|---|---|
| **Island Top Surface & Ear Flares** | `#09090B` (100% match for seamless bezel integration) |
| **Specular Top Hardware Rim** | `1px` `@linear-gradient(180deg, #FFFFFF28 0%, #FFFFFF05 100%)` |
| **Island Body Gradient** | `@linear-gradient(180deg, #09090B 0%, #0E1015 100%)` |
| **Adaptive Cover Art Ambience** | Dynamic vibrant tint extracted from album art with 600ms ease-in-out transition |
| **Island Border** | `#262936` (`1px`), ear flare outer stroke `#222530` |
| **Compact Pill Surface / Border** | `#14161F` / `#242938` |
| **Primary Text (Track / Active Lyric)** | `#FFFFFF` / `#F8FAFC` |
| **Secondary Text (Artist / Album)** | `#A1A1AA` |
| **Muted Metadata (Timestamps)** | `#8E8E93` |
| **Inactive Lyrics (Rest / Hover)** | `#FFFFFF45` / `#FFFFFFAA` |
| **Lyrics & Active Accent** | `#1ED760` (Spotify/live green or adaptive cover accent) |
| **Clock / Battery Healthy Accent** | `#38BDF8` (charging `#22C55E` / `#4ADE80`, low `<20%` `#EF4444`) |

---

## 3. Motion Choreography & Spring Physics

- **Asymmetric Morphing Curves:**
  - **Expand (Organic Overshoot):** `cubic-bezier(0.34, 1.35, 0.64, 1.0)` (`420ms` width, `380ms` height).
  - **Collapse (Crisp Deceleration):** `cubic-bezier(0.22, 1.0, 0.36, 1.0)` (`360ms` width, `320ms` height).
- **Hover Lifecycle Debounce:**
  - Expand delay on hover enter: `80ms`.
  - Collapse grace period on hover exit (when unpinned): `250ms`.
- **Expanded 3-Wave Cascade:**
  - **Wave 1 (Album Art + Track Info):** `delay: 60ms`, `y: -10px -> 0px`, `duration: 320ms`.
  - **Wave 2 (Progress Bar + Timestamps):** `delay: 110ms`, `y: -12px -> 0px`, `duration: 340ms`.
  - **Wave 3 (Transport Controls):** `delay: 150ms`, `y: -14px -> 0px`, `duration: 360ms` with spring overshoot.
  - **Lyrics Column Unfold:** `delay: 130ms`, `x: 14px -> 0px`, `duration: 380ms`.
- **Kinetic Waterfall Lyric Stream:**
  - Unlike rigid clipped boxes, the entire lyric column flows as a continuous physical stream.
  - Incoming active line rolls up `y: +18px -> 0px`, scaling up from `12px` to `15.5px` bold and fading `opacity: 0.4 -> 1.0`.
  - Outgoing previous line rolls up `y: 0px -> -18px`, scaling down from `15.5px` to `12px` and dimming `opacity: 1.0 -> 0.4`.
  - Deceleration easing: `cubic-bezier(0.16, 1.0, 0.3, 1.0)` over `340ms`.
- **Contextual Notification Alert Pulse:**
  - Temporary banner expansions (waktu sholat / reminder minum) pop out for 6–8 seconds with spring overshoot, then automatically retract to compact mode.
- **Tactile Micro-Interactions:**
  - `PlayPauseButton` shrinks `-3px` on press (`180ms`).
  - Progress bar track thickens `4px -> 6px` on hover (`140ms`).
  - Inactive lyric lines nudge right `+4px` and brighten on hover (`180ms`), supporting click-to-seek (`seek-to-lyric(-1 | 1)`).

---

## 4. Iconography & Asset Standards

- **Zero OS Emojis:** Never render Unicode emojis in UI markup.
- **Lucide Vector SVGs:** Store all icons in `assets/icons/*.svg` and load via Slint `@image-url("../assets/icons/<name>.svg")` with dynamic `colorize`.
- **Tangential Ear Flares:** Locked inside `island-wrapper` with `1px` overlap into `island`:
  - Left: `M 0 0 C 8 0 14 5 14 14 L 14 0 Z`
  - Right: `M 0 0 L 0 14 C 0 5 6 0 14 0 Z`
