# Decisions

## Separate project

**Decision:** Build `fidra-music` as a sibling project instead of modifying `fira`.

**Reason:** Protect the existing codebase and make the music prototype independently buildable.

## Music first

**Decision:** The initial scope is only the music player/island experience.

**Reason:** Establish a small, testable vertical slice before adding unrelated product surfaces.

## Slint candidate

**Decision:** Use Slint with Rust for the first prototype.

**Reason:** Validate declarative state-driven UI and animation without coupling the experiment to the existing WPF implementation.

## Dedicated Background Worker Thread for Media Controller

**Decision:** Offload Windows GSMTC session queries, polling, and transport commands to a dedicated background worker thread communicating with the UI thread via `mpsc` channels and `upgrade_in_event_loop`.

**Reason:** Calling WinRT async methods synchronously (`.get()`) or holding mutable borrows on the main Slint UI thread deadlocks with the Windows COM message pump when user clicks or hovers, freezing the UI into "Not Responding".

## Non-Blocking WinRT Transport Execution

**Decision:** Invoke transport operations (`TryTogglePlayPauseAsync`, `TrySkipNextAsync`, `TrySkipPreviousAsync`, `TryChangePlaybackPositionAsync`) asynchronously without synchronous `.get()` waits.

**Reason:** Windows manages transport calls natively in the background; blocking the thread on transport futures is unnecessary and halts the application.

## Strict Morphing Architecture (Eliminating `if` State Destructuring)

**Decision:** Mandate a single unified morphing container `island` where both compact and expanded layers remain mounted simultaneously, controlled via opacity and vertical translate. Root `Window` is fixed at 480x410px.

**Reason:** Destroying and mounting DOM/UI trees with conditional `if` completely breaks interpolation animations, while per-frame OS window resizing causes severe Win32 DWM stutter.

## Asymmetric Spring & Organic Physics (Nusa-Style Living Motion)

**Decision:** Adopt asymmetric easing with organic overshoot on expand (`cubic-bezier(0.34, 1.45, 0.64, 1.0)`) and crisp deceleration on retract (`cubic-bezier(0.22, 1.0, 0.36, 1.0)`), coupled with a 3-tier staggered entrance cascade (80ms/120ms/160ms).

**Reason:** Eliminates rigid, robotic motion and gives the dynamic island an organic, tactile, and responsive physical feel.

## Win32 Dynamic Hit-Testing via `WM_NCHITTEST`

**Decision:** Intercept `WM_NCHITTEST` on the HWND to return `HTTRANSPARENT` whenever cursor coordinates fall outside the active island contour, returning `HTCLIENT` only within the island.

**Reason:** Prevents the fixed 480x410 transparent bounding window from blocking mouse clicks to underlying desktop applications when in compact notch mode.

## Anti-AI-Slop Standards: Lucide Vector SVG Icon System & Obsidian Palette

**Decision:** Adopt official Lucide vector SVG icons stored as discrete assets under `assets/icons/*.svg`, referenced directly via Slint's `@image-url(...)` with dynamic `colorize`. Ban all OS emojis and inline manual path strings. Enforce unified dark obsidian palette across all states.

**Reason:** Guarantees crisp, high-end desktop aesthetics without cheap-looking mobile emoji glyphs or long maintainability-killing SVG path string literals in UI markup. Slint bakes the SVGs directly into the binary at compile time.

## Unified Island Wrapper & Instant Responsive Collapse

**Decision:** Implement composite hover tracking (`is-mouse-inside`) in Slint covering all interactive elements and background, with an instant 40ms exit buffer in Rust (down from 800ms). Enclose ear flares and the body into a unified `island-wrapper` with a single animated `current-island-width` property and restrained shadows (`#00000040`).

**Reason:** Eliminates the sluggish 800ms collapse delay so the island shrinks immediately when the cursor exits, while preventing double-animation lag and excessive dark smudging.

## Framework validation

**Decision:** The final framework choice remains open until the prototype is evaluated.
