# Strict Slint UI, Motion & Rust Windowing Rules (Fidra Music Island)

You MUST obey every rule in this document whenever creating or modifying `.slint` files or Rust windowing code. Violation of any rule below is unacceptable.

---

## 1. Architecture & Morphing Rules (CRITICAL)
- **NEVER use `if is-expanded : ...` and `if !is-expanded : ...` for state transitions.** Conditional `if` destroys and recreates elements instantly, completely killing animations. Both Compact and Expanded layers MUST remain mounted inside a single morphing container, controlled via `opacity`, `y` offset, and `visible: self.opacity > 0.01;`.
- **NEVER animate or bind `width`/`height` dynamically on the root `Window`.** Resizing an OS window per frame causes severe Win32 stutter. Keep the root `Window` fixed at its maximum bounding size (e.g., `width: 480px; height: 410px; background: transparent; no-frame: true; always-on-top: true;`) and animate the centered inner container (`x: (parent.width - self.width) / 2;`) instead.
- **Single Unified Container:** Use ONE primary `Rectangle` for the island body with `clip: true;` so all child elements are cleanly masked during expand/collapse morphing. Ear-flare `Path` elements must dynamically track `island.x` and `island.width`.

---

## 2. Organic Easing & Spring Physics (Nusa-Style Living Motion)
- **Asymmetric Spring & Smooth Curves:** Never use linear transitions or identical open/close curves:
  - **Expand / Pop-Out (Organic Spring Overshoot):** Use `cubic-bezier(0.34, 1.45, 0.64, 1.0)` (or `cubic-bezier(0.34, 1.56, 0.64, 1.0)` for playful micro-elements) so the island expands energetically with a subtle, physical settle at the end.
  - **Collapse / Retract (Snappy Deceleration):** Use `cubic-bezier(0.22, 1.0, 0.36, 1.0)` so shrinking feels crisp, fast, and controlled without bouncing into the screen edge.
  - In Slint, apply this dynamically on property transitions using:
    `easing: is-expanded ? cubic-bezier(0.34, 1.45, 0.64, 1.0) : cubic-bezier(0.22, 1.0, 0.36, 1.0);`
- **Container Morphing:** Animate `width`, `height`, `border-radius`, `drop-shadow-blur`, and `drop-shadow-offset-y` simultaneously over `380ms - 440ms`.
- **Staggered Content Entrance:** When expanding, child sections MUST cascade sequentially using `delay` (`80ms` for header, `120ms` for hero card, `160ms` for footer/volume) combined with a subtle vertical slide (`y: is-expanded ? 0px : 12px`) and `opacity` fade (`260ms - 300ms`). When collapsing, set `delay: 0ms` and fade out fast (`150ms`) so content disappears before the container shrinks.
- **Micro-Interactions (Tactile Feedback):**
  - Compact pill hover: subtly expand width (`+16px` to `+20px`) over `240ms`.
  - Interactive sliders (progress/volume): thicken track height on `has-hover` (`5px` -> `8px`) over `160ms`.
  - Primary action buttons: shrink slightly on `touch.pressed` (`-3px` width/height) and animate `drop-shadow-blur` on hover.

---

## 3. Event-Driven Zero-CPU Idle ("Doze Off" Architecture)
- **Stop All Timers & Loops When Idle:** Never run continuous `Timer` loops, polling animations, or visualizer bar oscillations when `is-playing == false` and no expand/hover transition is active.
- **Let Slint Sleep:** When `is-playing == false`, bind visualizer/waveform heights to static resting values (`4px`) so Slint's GPU renderer halts frame redraws completely (`0%` idle CPU usage). Only start periodic UI updates when media is actively playing or a user interaction occurs.

---

## 4. Win32 Transparent Click-Through & Hit-Testing (Rust Backend)
- **Prevent Invisible Bounding-Box Blocking:** Because the root `Window` (`480px x 410px`) is larger than the collapsed notch pill (`~290px x 38px`) to accommodate shadows and expansion, the transparent outer region MUST NOT block mouse clicks to underlying desktop apps.
- **Dynamic Hit-Test / `WS_EX_TRANSPARENT`:** In the Rust Win32 windowing layer (`windows-rs`), either:
  1. Intercept `WM_NCHITTEST` and return `HTTRANSPARENT` whenever the cursor coordinates fall outside the active `island` bounding rect (`island.x`, `island.y`, `island.width`, `island.height`), **OR**
  2. Toggle `WS_EX_TRANSPARENT` on the HWND dynamically based on whether the cursor is hovering inside the active notch bounds.

---

## 5. Anti-AI-Slop Visual & Styling Standards
- **NO OS Emojis in UI:** NEVER use Unicode emojis (`📌`, `📍`, `🎵`, `🔊`, `▶`, `❚❚`, `⏮`, `⏭`, `▲`) for icons. Always use crisp SVG `Path` elements with explicit `viewbox-width: 24; viewbox-height: 24;` and centered coordinates.
- **Unified Dark Obsidian Palette:** Maintain the exact same surface palette across Compact and Expanded states—never switch color themes between states:
  - Main surface: `@linear-gradient(180deg, #0B0C10 0%, #12141C 100%)`
  - Elevated inner card: `#161822`
  - Rim-light top border: `@linear-gradient(180deg, #38FFFFFF 0%, #10FFFFFF 100%)` (`border-width: 1px`)
  - Outer elevation shadow: `drop-shadow-blur: 24px; drop-shadow-color: #000000CC; drop-shadow-offset-y: 8px;`
- **Typography Hierarchy:**
  - Primary text (Track Title): `#F4F4F6`, `font-weight: 700`
  - Secondary text (Artist): `#B4B8C5`, `font-weight: 500`
  - Muted metadata/timestamps: `#717687`, `font-weight: 500`
- **Safe Math Clamping:** Always wrap dynamic ratio widths (progress bar, volume bar) in `Math.clamp(value, 0.0, 1.0)` so out-of-range values never break layout geometry.

## 6. Seamless Top-Edge Ear Flares (Inverted Corner Fillets)
- **Synchronized Motion:** Both left and right ear-flare `Path` elements MUST define `animate x` with the exact same `duration` and `easing` curve as `island.width` so they never detach or lag during hover/expand/collapse transitions.
- **Zero-Seam Overlap:** Overlap the inner vertical edge of each ear-flare `Path` by `1px` into the main `island` rectangle (`x: island.x - 13px` for left, `x: island.x + island.width - 1px` for right) and match the exact top background color (`#0B0C10`) so no vertical border seam is visible.
- **Smooth Tangential Bezier:** Use smooth cubic bezier SVG paths (`M 0 0 C 8 0 14 5 14 14 L 14 0 Z` for left; `M 0 0 L 0 14 C 0 5 6 0 14 0 Z` for right) so the curve meets both the screen top edge and the notch side wall tangentially.
