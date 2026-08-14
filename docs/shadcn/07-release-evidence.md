# Release evidence

## Fixed reference identity

| Item | Value |
|---|---|
| GPUI Component baseline | `e1570bdc` |
| shadcn/ui revision | `607e8a9717fe6ff0d374ba74c651012f9c052534` |
| shadcn style and base | Vega and Radix |
| Alignment Story | `Shadcn Alignment` |
| Required viewport | 1440 x 1000 logical pixels; macOS TestPlatform stores 2x Retina output at 2880 x 2000 |
| Required themes | One light and one dark Color Theme |
| Required presets | Vega full matrix; Nova and Maia representative geometry |

Captures must include the Story header so the Color Theme, Style Preset, and pinned shadcn revision remain visible with the component state.

## Pinned upstream source map

The local shadcn/ui checkout is clean at the pinned revision. Source comparison uses Radix-backed `new-york-v4` components, with the GPUI built-in presets representing the corresponding geometry families rather than copying DOM structure:

| Family | Pinned source |
|---|---|
| Core controls | `apps/v4/registry/new-york-v4/ui/button.tsx`, `checkbox.tsx`, `input.tsx`, `textarea.tsx`, `select.tsx`, and `combobox.tsx` |
| Overlays | `tooltip.tsx` and `dialog.tsx` plus the shared Radix primitives used by the registry |
| Disclosure/forms | `accordion.tsx`, `collapsible.tsx`, `calendar.tsx`, and `slider.tsx` |
| Data | `table.tsx` |

The upstream Accordion source uses an overflow-clipped measured-height enter/exit contract and a 200 ms indicator transition. AlertDialog uses a 100 ms overlay fade plus content fade/scale and derives its accessible name from the visible title. GPUI matches the duration, opacity, lifecycle, focus routing, and AccessKit behavior. The pinned GPUI renderer has no element-level backdrop filter or layout-independent transform for arbitrary element trees, so AlertDialog intentionally omits backdrop blur and scale instead of using reflowing approximations.

Standard Dialog aligns the pinned Vega/Nova/Maia surface, overlay, spacing, typography, close-control, footer, and responsive-width intent. It intentionally retains the existing desktop placement at approximately 10% of the viewport height and the semantic 250 ms opacity, translation, shadow, interruption, reverse-exit, and reduced-motion lifecycle instead of adopting the centered shadcn zoom. Element-level backdrop blur remains a documented renderer limitation.

## Frozen Phase 0 visual baseline

The same `shadcn_phase0_capture` source compiles without warnings against the frozen `e1570bdc` export and the current checkout. The baseline executable generated fixed Light/Dark captures containing representative controls, selection states, inputs, Select, and an open Popover:

| Capture | Resolution | SHA-256 |
|---|---:|---|
| [Phase 0 light](screenshots/phase0/phase0-light.png) | 2880 x 2000 | `df5c8e114db0ed720f576f506488bfd7d86fd9a1cee581371396a89180b8427a` |
| [Phase 0 dark](screenshots/phase0/phase0-dark.png) | 2880 x 2000 | `3234e1fa096b3c4836b9706391c2c6c4cd3692dedb13e8017cb6a37ee3d9d034` |

Both captures visibly identify the frozen revision, Color Theme, and 1440 x 1000 logical-pixel viewport. The controls and Popover were visually reviewed for clipping and state legibility.

Regenerate the frozen references from an exported baseline worktree with:

```bash
SHADCN_CAPTURE_REVISION=e1570bdc \
SHADCN_CAPTURE_OUTPUT=/absolute/path/to/docs/shadcn/screenshots/phase0 \
cargo run -p gpui-component-story \
  --example shadcn_phase0_capture --features visual-test
```

## Intentional differences

| Area | GPUI behavior retained | Reason |
|---|---|---|
| Pointer | Buttons keep the desktop default cursor | Browser pointer conventions do not override native desktop controls |
| Portals | Root and anchored GPUI overlays replace DOM Portal | GPUI owns window composition and focus routing |
| State attributes | Rust state and render branches replace `data-state` | No DOM or CSS selector layer is introduced |
| Accessibility | AccessKit roles, values, actions, focus handles, and native platform adapters replace ARIA | Platform accessibility is authoritative |
| Menus | NativeMenu remains platform-native | Native application menus are not restyled as Web popup menus |
| Typography and icons | Color Theme typography and the GPUI icon source remain independent from Style Preset | Style switching must not replace application content assets |
| P3 surfaces | Dock, Editor, TextView, charting, window chrome, and native scrolling retain GPUI structures | Only shared semantic colors, radii, elevation, focus, and motion are adopted where applicable |
| Raster output | Geometry and semantic states are compared instead of exact pixels | Platform fonts, GPU composition, and native controls differ |

## Visual capture matrix

| Capture | Required states | Status |
|---|---|---|
| Vega light | Full P0/P1 state matrix, Card surfaces, supporting surfaces, all overlay triggers | [Page 1](screenshots/default-light-vega-page-01.png), [page 2](screenshots/default-light-vega-page-02.png), [page 3](screenshots/default-light-vega-page-03.png), [page 4](screenshots/default-light-vega-page-04.png), [page 5](screenshots/default-light-vega-page-05.png) |
| Vega dark | Full P0/P1 state matrix, Card surfaces, supporting surfaces, all overlay triggers | [Page 1](screenshots/default-dark-vega-page-01.png), [page 2](screenshots/default-dark-vega-page-02.png), [page 3](screenshots/default-dark-vega-page-03.png), [page 4](screenshots/default-dark-vega-page-04.png), [page 5](screenshots/default-dark-vega-page-05.png) |
| Nova light | Default, disabled, invalid, loading, Card and supporting surfaces, representative overlays | [Page 1](screenshots/default-light-nova-page-01.png), [page 2](screenshots/default-light-nova-page-02.png), [page 3](screenshots/default-light-nova-page-03.png), [page 4](screenshots/default-light-nova-page-04.png) |
| Nova dark | Default, Card surfaces, and representative overlay geometry | [Page 1](screenshots/default-dark-nova-page-01.png), [page 2](screenshots/default-dark-nova-page-02.png), [page 3](screenshots/default-dark-nova-page-03.png), [page 4](screenshots/default-dark-nova-page-04.png) |
| Maia light | Default, disabled, invalid, loading, Card and supporting surfaces, representative overlays | [Page 1](screenshots/default-light-maia-page-01.png), [page 2](screenshots/default-light-maia-page-02.png), [page 3](screenshots/default-light-maia-page-03.png), [page 4](screenshots/default-light-maia-page-04.png), [page 5](screenshots/default-light-maia-page-05.png) |
| Maia dark | Default, Card surfaces, and representative overlay geometry | [Page 1](screenshots/default-dark-maia-page-01.png), [page 2](screenshots/default-dark-maia-page-02.png), [page 3](screenshots/default-dark-maia-page-03.png), [page 4](screenshots/default-dark-maia-page-04.png), [page 5](screenshots/default-dark-maia-page-05.png) |

The captures were generated from the real `ShadcnAlignmentStory` through GPUI's macOS `HeadlessAppContext` and `MetalHeadlessRenderer`. Reduced motion is forced only for deterministic raster output. Every page has fixed metadata chrome, and adjacent scroll pages overlap so no state row is omitted. Light/Dark colors, Vega/Nova/Maia geometry, invalid states, Select/Combobox values, disclosure/data states, and all page boundaries were visually reviewed. The 24 files have distinct SHA-256 hashes; every variant now uses four pages.

The matrix now also includes multiline Input, NumberInput, open/invalid OTP, expanded/collapsed Accordion rows, measured open Collapsible content, fixed August 2026 Calendar/DatePicker states, and single/range/disabled Slider states. Disclosure and locale-sensitive surfaces are visible without clipping and render through the same implementations used by the interactive Story.

Regenerate the matrix with:

```bash
cargo run -p gpui-component-story --example shadcn_capture --features visual-test
```

## Overlay placement and lifecycle captures

The dedicated 960 x 640 logical-pixel overlay harness runs with motion enabled. Open captures wait past the Vega fast duration; closing captures update the controlled Popover and draw immediately, proving that exit content remains mounted before the completion timer unmounts it.

| State | Evidence |
|---|---|
| Four placements | [Top](screenshots/overlays/popover-top-light-open.png), [bottom](screenshots/overlays/popover-bottom-light-open.png), [left](screenshots/overlays/popover-left-light-open.png), [right](screenshots/overlays/popover-right-light-open.png) |
| Window edge | [Constrained top-left edge](screenshots/overlays/popover-constrained-edge-light-open.png); content is snapped inside the viewport without clipping |
| Closing mounted | [Light closing](screenshots/overlays/popover-right-light-closing.png) and [dark closing](screenshots/overlays/popover-right-dark-closing.png) retain the surface after controlled close begins |
| Dark geometry | [Dark right placement](screenshots/overlays/popover-right-dark-open.png) |

All eight files are 1920 x 1280 Retina output with distinct SHA-256 hashes and visible revision, theme, Style, and scenario metadata. Regenerate them with:

```bash
cargo run -p gpui-component-story \
  --example shadcn_overlay_capture --features visual-test
```

## Locale layout captures

The same fixed August 2026 Calendar, DatePicker, and invalid Form layout was rendered with component locale resolution and representative long copy:

| Locale | Evidence | Result |
|---|---|---|
| English | [en](screenshots/locales/locale-en.png) | Calendar labels and long form copy fit without clipping |
| Simplified Chinese | [zh-CN](screenshots/locales/locale-zh-CN.png) | Month/week labels and long validation copy fit without clipping |
| Traditional Chinese | [zh-TW](screenshots/locales/locale-zh-TW.png) | Month/week labels and long validation copy fit without clipping |

All three files are fixed 2000 x 1400 Retina output, identify locale/Style/revision/viewport, and have distinct hashes. Regenerate them with:

```bash
cargo run -p gpui-component-story \
  --example shadcn_locale_capture --features visual-test
```

## Accessibility and interaction matrix

| Platform | Keyboard/focus | AccessKit or native tree | Reduced motion | Status |
|---|---|---|---|---|
| macOS | Deterministic Rust tests and live Escape dismissal pass; full focus/IME review remains | Live tree verifies core control roles/names/values/states, Form error description, and named overlay close controls | Helper and lifecycle tests pass; application override verified | Partial |
| Windows | Required | Required | Required | Not run |
| Linux | Required | Required | Required | Not run |

The deterministic suite covers invalid, disabled, read-only, checked, mixed, selected and editable values; overlay duplicate dismissal, nested close, focus restoration ownership, interrupted reopen, and reduced-motion unmount. On macOS, the rebuilt live native tree exposed Radio, Checkbox, Switch, TextInput, Select, Combobox, Form, Progress, and overlay-trigger roles. It exposed Button, Toggle, and Switch disabled states; Select `Alpha` and Combobox `Rust` names and values; checked/mixed values; the Form error description; and localized Close names for Dialog, Sheet, and Notification. Dialog Escape dismissal removed the modal close control from the tree after exit completion.

The frozen `e1570bdc` source inventory found correct Button/Link, Checkbox, Radio, Switch, text-input, multiline-input, and Dialog roles, visible-label names, focus handles, and basic checked/selected values. It also recorded the pre-change gaps: icon-only Dialog close had no accessible name; invalid, mixed, and disabled semantic states were not consistently written to AccessKit; and no shared overlay completion contract protected dismissal/focus restoration. The current focused tests and live macOS tree evidence above cover those deltas.

Input automation specifically covers IME underline splitting across syntax boundaries, masked IME ranges, composition selection offsets, multiline replacement, read-only edit rejection, and NumberInput keyboard normalization/escape. OTP automation covers normalized paste and AccessKit SetValue. These are deterministic behavior checks; native IME candidate-window placement and screen-reader announcements remain live platform checks.

Accordion now requires stable item values and owns single/multiple controlled or uncontrolled state at the group level. Its Preset metrics reproduce Vega and Nova divided lists and Maia's unified framed treatment without preset-id branches. Triggers expose expanded and disabled states, retain their indicator while disabled, and support `Enter`, `Space`, `ArrowUp`, `ArrowDown`, `Home`, and `End`. Keyed Collapsible uses the 200 ms slow motion token, measures dynamic content height, keeps closing content mounted, rejects stale completion after interrupted reopen, and unmounts after a matching close. Focused tests cover state ordering, activation, motion lifecycle, and built-in disclosure metrics.

The first live inspection found that icon-only overlay close buttons were unnamed. `Button::aria_label` and localized Close labels were added, covered by a focused AccessKit test, and verified in the rebuilt live Story bundle. Dialog and Sheet set AccessKit dialog roles internally, and declarative Dialog/AlertDialog triggers preserve their role and label, but the current macOS adapter did not enumerate generic modal containers as separate native-tree entries. Static text inside generic overlay content is also not represented as a separate node. Modal-name and screen-reader announcement behavior therefore remain platform review items.

## Release performance benchmark

`crates/ui/benches/shadcn_alignment.rs` uses GPUI's official headless `BenchAppContext`, Criterion iteration measurement, and frame profiler.

| Scenario | Workload |
|---|---|
| `control_state_matrix_render` | 40 rows of Button, Checkbox, Radio, and Switch state changes |
| `loading_surface_render` | 48 rows of Spinner, Skeleton, determinate Progress, and indeterminate Progress |
| `rapid_overlay_toggle_render` | Controlled Popover reverses open/close before prior completion necessarily settles |
| `virtual_scroll_1000_rows_render` | Uniform virtual list alternates between rows 0 and 999 |
| `startup_surface_mount` | Repeatedly mounts a representative 24-row first surface |
| `idle_queue_drain` | Drains an already-idle GPUI dispatcher with a mounted surface |

Run on the same machine and checkout state:

```bash
cargo bench -p gpui-component --bench shadcn_alignment -- \
  --sample-size 30 --warm-up-time 1 --measurement-time 5
```

The benchmark dependencies are development-only. Criterion is also the benchmark driver used by GPUI's `bench` feature; it does not enter the published component's runtime dependency graph.

Current macOS evidence on Apple M1 Pro, macOS 27.0 uses 30 Criterion samples, a 1-second warmup, a 5-second measurement, the same GPUI revision, and the same benchmark source in the frozen `e1570bdc` export and current checkout.

| Scenario | Phase 0 Criterion estimate | Current estimate | Change |
|---|---:|---:|---:|
| Control state matrix | 4.0312 ms | 3.8831 ms | -3.67% |
| Loading surfaces | 799.43 us | 797.71 us | -0.22% |
| Rapid overlay toggle | 184.37 us | 198.03 us | +7.41% |
| 1,000-row virtual scroll | 242.48 us | 248.90 us | +2.65% |
| Startup surface mount | 1.1505 ms | 1.2067 ms | +4.88% |
| Idle queue drain | 28.365 ns | 28.674 ns | +1.09% |

| Scenario | Version | Draw mean | p95 | p99 | Max | 120 Hz overruns |
|---|---|---:|---:|---:|---:|---:|
| Control state matrix | Phase 0 | 3.791 ms | 4.719 ms | 5.423 ms | 6.672 ms | 0 / 1650 |
| Control state matrix | Current | 3.616 ms | 4.395 ms | 4.936 ms | 22.954 ms | 2 / 1906 |
| Loading surfaces | Phase 0 | 0.702 ms | 0.957 ms | 1.295 ms | 41.648 ms | 5 / 8557 |
| Loading surfaces | Current | 0.704 ms | 0.964 ms | 1.311 ms | 2.367 ms | 0 / 8557 |
| Rapid overlay toggle | Phase 0 | 0.090 ms | 0.220 ms | 0.356 ms | 1.201 ms | 0 / 36556 |
| Rapid overlay toggle | Current | 0.063 ms | 0.123 ms | 0.205 ms | 0.930 ms | 0 / 33766 |
| 1,000-row virtual scroll | Phase 0 | 0.191 ms | 0.297 ms | 0.505 ms | 2.501 ms | 0 / 29116 |
| 1,000-row virtual scroll | Current | 0.197 ms | 0.301 ms | 0.481 ms | 3.865 ms | 0 / 24555 |
| Startup surface mount | Phase 0 | 1.095 ms | 1.457 ms | 2.111 ms | 9.396 ms | 2 / 5673 |
| Startup surface mount | Current | 1.136 ms | 1.548 ms | 2.277 ms | 3.893 ms | 0 / 5673 |

The benchmark also performs one warmed synchronous render with its counting allocator enabled. It records allocation requests, not retained heap size.

| Scenario | Version | Allocation operations | Requested bytes | Change |
|---|---|---:|---:|---:|
| Control state matrix | Phase 0 | 13,474 | 10,646,244 | Baseline |
| Control state matrix | Current | 13,752 | 9,928,804 | +2.06% operations; -6.74% bytes |
| Loading surfaces | Phase 0 | 2,945 | 1,751,824 | Baseline |
| Loading surfaces | Current | 3,005 | 1,752,268 | +2.04% operations; +0.03% bytes |
| Rapid overlay toggle | Phase 0 | 213 | 144,894 | Baseline |
| Rapid overlay toggle | Current | 233 | 147,334 | +9.39% operations; +1.68% bytes |
| 1,000-row virtual scroll | Phase 0 | 570 | 765,012 | Baseline |
| 1,000-row virtual scroll | Current | 570 | 765,012 | Unchanged |

No sustained frame-budget regression is present. The largest relative elapsed increase is the rapid overlay workload at 7.41%, but its current p99 is 0.205 ms with no 120 Hz overruns; this remains within the release budget. Startup p99 is 2.277 ms, virtual-scroll p99 improves from 0.505 ms to 0.481 ms, and loading p99 remains 1.311 ms. Control requested bytes decrease 6.74%. Isolated max-frame outliers occur in both versions and do not affect p99 budget compliance.
