# Verification strategy

## Verification layers

| Layer | Purpose | Required evidence |
|---|---|---|
| Source comparison | Confirm intended shadcn state and dimensions | Pinned source path, commit, style, base, and state |
| Logic tests | Protect variants, state transitions, fallbacks, and lifecycle | Focused Rust tests |
| Interaction tests | Protect keyboard, pointer, focus, and dismissal behavior | `#[gpui::test]` or the closest deterministic harness |
| Story matrix | Review all visual states together | Light and dark state rows at fixed scale |
| Platform review | Catch font, window, menu, and accessibility differences | macOS, Windows, and Linux checklist |
| Performance review | Prevent animation and rendering regressions | Before and after measurements in release mode |

## Required component state matrix

Every changed interactive component must cover applicable combinations from this table.

| Dimension | Values |
|---|---|
| Theme | Light, dark |
| Style Preset | Vega for full state coverage; Nova and Maia for representative geometry coverage |
| Size | `xs`, `sm`, `md`, `lg`, plus icon sizes where supported |
| Interaction | Default, hover, focus visible, active or pressed |
| Availability | Enabled, disabled, loading, read-only where applicable |
| Semantic | Selected, checked, indeterminate, invalid, destructive where applicable |
| Content | Text, icon plus text, icon only, long content, CJK content |
| Overlay | Four placements, constrained window edge, nested trigger |
| Motion | Enter, exit, interrupted transition, reduced motion |

Avoid a Cartesian-product explosion. Each state must appear at least once, and high-risk intersections must be explicit.

## Logic and interaction tests

### Foundation

- Theme JSON loads color, typography, and syntax data without Style fields.
- Semantic token mapping works for light and dark themes.
- Vega is the explicit default Style Preset.
- Explicit Style Presets change geometry and motion without changing colors.
- Color Theme switching preserves explicit Style Preset metrics.
- Style Preset switching preserves Color Theme and syntax highlighting.
- Unknown preset names return an actionable error and preserve the last valid preset.
- Vega, Nova, and Maia resolve to distinct expected metrics.
- Size metrics resolve consistently across component families.
- Easing functions clamp inputs and produce correct endpoints.
- Transition cancellation, restart, and completion are deterministic.
- Reduced motion produces the final visual state without delayed unmount.

### Controls

- Disabled and loading controls do not dispatch actions.
- Focus-visible styling follows keyboard focus, not every pointer click.
- Invalid state exposes both visual and accessibility information.
- Toggle, checkbox, radio, and switch accessibility values match visual state.
- Inputs preserve IME, selection, masking, and clipboard behavior.

### Overlays

- Escape dismisses the topmost eligible overlay.
- Outside click behavior matches the component contract.
- Modal focus cannot escape while open.
- Focus restores to the correct trigger once.
- Closing callbacks fire once for every dismissal path.
- Nested menu, popover, dialog, and select combinations dismiss in order.
- Reopen during exit is deterministic and leaves no orphaned task.

### Disclosure and navigation

- Accordion and Collapsible support dynamic content height.
- Tabs support keyboard navigation and rapid repeated switching.
- Sidebar collapse and expand complete correctly after interruption.
- Hidden or unmounted controls leave the focus order.

## Visual review protocol

For each pull request:

1. Open the pinned shadcn component using Vega and Radix.
2. Capture the relevant state at a recorded viewport and scale.
3. Render the corresponding GPUI Story state at a recorded window size and scale.
4. Compare geometry, hierarchy, state contrast, shadow, radius, typography, and motion intent.
5. Record intentional native deviations in the component matrix or pull-request description.

For each component family:

- Run the full state comparison with Vega.
- Run representative default, focus, disabled, and overlay geometry with Nova and Maia where applicable.
- Switch between at least two Color Themes while holding Style Preset constant.
- Switch between all built-in Style Presets while holding Color Theme constant.

Exact raster equality is not required because text rendering and platform composition differ. Geometry and semantic state should remain close enough that differences are deliberate and explainable.

## Accessibility review

| Area | Checks |
|---|---|
| Role | Control exposes the correct AccessKit or native role |
| Name | Icon-only controls have an accessible label |
| Value | Checked, selected, expanded, invalid, and editable values are exposed |
| Focus | Focus order is predictable and hidden content is excluded |
| Keyboard | Enter, Space, Escape, arrows, Tab, and Shift-Tab follow the control contract |
| Contrast | Text, borders, focus rings, and semantic states remain distinguishable |
| Motion | Reduced-motion mode removes nonessential interpolation and delay |

Accessibility behavior takes precedence over visual parity.

## Performance scenarios

Measure release builds against the Phase 0 baseline.

| Scenario | Risk |
|---|---|
| Repeated Tooltip or Menu open and close | Task churn and overlay allocation |
| Dialog and Sheet enter or exit | Full-window redraw and compositing cost |
| Rapid Tabs and Sidebar switching | Animation restart and layout work |
| 1,000-row Table or List scrolling | Shared style changes affecting virtualization |
| Skeleton and Progress animation | Continuous repaint cost |
| Multiple Notifications | Concurrent animation and stacking layout |
| Runtime Style Preset switching | Full-window refresh and cached metric invalidation |

Required interpretation:

- Record FPS and frame-time distribution, not FPS alone.
- Compare identical hardware, build profile, window size, and display refresh rate.
- Investigate sustained frame-time or allocation regressions before merge.
- Document platform-specific compositor behavior instead of hiding it in aggregate results.

## Commands

Run focused checks during development, then the complete suite before release.

```bash
cargo fmt --all -- --check
cargo clippy -- --deny warnings
cargo test -p gpui-component
cargo test --all
cargo check -p gpui-component --no-default-features
typos
```

Use the Gallery for visual and interaction review:

```bash
cargo run
```

Use a release build and platform profiler for performance comparisons. On macOS:

```bash
MTL_HUD_ENABLED=1 cargo run --release
samply record cargo run --release
```

## Pull-request evidence checklist

This is a per-change review template, not the current release status. See [implementation status](./06-implementation-status.md) for completed and remaining evidence.

- [ ] Pinned shadcn source files and states are identified.
- [ ] Light and dark Story states are updated.
- [ ] Vega full-state coverage and Nova or Maia representative coverage are updated.
- [ ] Color Theme and Style Preset independence is verified.
- [ ] Applicable interaction states are covered.
- [ ] Focus, keyboard, dismissal, and accessibility behavior is verified.
- [ ] Enter and exit motion is verified, including interruption.
- [ ] Reduced-motion behavior is verified.
- [ ] Focused tests pass.
- [ ] Formatting and Clippy pass.
- [ ] Performance-sensitive changes include before and after evidence.
- [ ] English and Simplified Chinese docs are updated together when public behavior changes.
- [ ] Intentional differences from shadcn are documented.
