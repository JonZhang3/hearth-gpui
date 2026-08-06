# Implementation status

## Implemented

| Area | Result |
|---|---|
| Theme/Style ownership | `Theme.style: Rc<StylePreset>` is the only Style runtime authority |
| Breaking API | Removed flat Theme radius/shadow fields and Theme JSON appearance keys |
| Built-in presets | Vega, Nova, and Maia with stable ids and distinct geometry |
| Custom presets | Rust registration through `StyleRegistry::register` |
| Independent selection | `Theme::set_style` and `Theme::set_color_theme` preserve the other selection |
| Validation | Empty ids/names, invalid control ordering, invalid overlay scale, duplicates, and unknown ids are rejected atomically |
| Core controls | Button, Toggle, Checkbox, Radio, Switch, Input, Select, Combobox, NumberInput, OtpInput, and DatePicker consume shared metrics where applicable |
| State contracts | Checkbox exposes checked/unchecked/mixed; Checkbox, Radio, Input, OtpInput, and Field expose invalid; Input exposes read-only without blocking selection/copy; OTP supports normalized paste and AccessKit SetValue |
| Icon-button names | `Button::aria_label` names icon-only controls without adding visible text; Dialog, Sheet, Notification, Search, and Inspector close buttons use the localized Close label |
| Focus | Shared focus ring width and offset are Style metrics; Color Theme retains ring color |
| Motion contract | Shared fast/normal/slow/emphasis/loading durations, semantic enter/exit/move easing, placement offsets, and reduced-motion duration override |
| Overlay lifecycle | Tooltip, Popover, HoverCard, Dropdown/Context Menu, Select, Combobox, Dialog/AlertDialog, Sheet, and Notification use interruptible open/close ownership or an owning lifecycle wrapper |
| Overlay correctness | Exit content remains mounted and blocks input; stale close completion, duplicate dismissal, nested Dialog close-all, and interrupted reopen paths have focused tests |
| Loading motion | Skeleton uses a restrained pulse; Spinner and indeterminate Progress use shared loading duration and easing |
| Navigation | Tabs and Sidebar consume named motion durations |
| Disclosure | Accordion and keyed Collapsible measure dynamic content height, animate with shared Style motion, retain exit content, and ignore stale close completion after reopen; Accordion triggers support focus-visible, `Enter`/`Space`, expanded state, and explicit accessible names |
| Data surfaces | Table and DataTable row heights and cell padding consume Style data metrics |
| P2/P3 surfaces | Existing radius and elevation consumers use shared semantic Style metrics while retaining GPUI-native behavior |
| Gallery | Settings menu selects all registered presets; Color Theme and Style id persist independently |
| Alignment Story | Deterministic P0/P1 states, independent Light/Dark and registered Style Preset controls, supporting surfaces, Form error contract, and pinned shadcn revision metadata |
| Visual references | 24 fixed macOS Metal captures cover Light/Dark and Vega/Nova/Maia across overlapping pages with persistent identity metadata |
| Overlay references | Eight motion-enabled Popover captures cover four placements, constrained edge snapping, Light/Dark open surfaces, and closing-content retention |
| Locale references | Fixed English, Simplified Chinese, and Traditional Chinese Calendar/DatePicker/Form captures verify long-copy layout without clipping |
| Schema | Removed legacy Style properties from `.theme-schema.json` |
| Public documentation | English and Simplified Chinese Theme and affected component pages document independent selection, custom Rust presets, migration, mixed/invalid/read-only/error states, OTP paste, and accessibility mappings |
| Performance harness | Headless release benchmarks cover startup mount, idle drain, control state changes, loading, interrupted overlay toggles, and 1,000-row virtual scrolling with frame percentiles, budget overruns, and warmed allocation requests; benchmark dependencies remain development-only |

## Verification results

Verified on macOS on 2026-08-06:

| Command or check | Result |
|---|---|
| `cargo check -p gpui-component-story` | Passed |
| `cargo check -p gpui-component --no-default-features` | Passed |
| `cargo clippy -- --deny warnings` | Passed for the workspace |
| `cargo test -p gpui-component` | 399 default-feature library tests passed |
| `cargo test --all` | Passed; 417 feature-unified library tests and all workspace crate and doc tests passed |
| `cargo fmt --all -- --check` | Passed after applying repository-wide rustfmt |
| `git diff --check` | Passed |
| Story Gallery startup | Passed; Theme registry reload completed without runtime error |
| `typos` | Passed with temporary `typos-cli 1.49.0`; no project dependency added |
| `cargo bench -p gpui-component --bench shadcn_alignment --no-run` | Passed in release profile |
| `cargo run -p gpui-component-story --example shadcn_capture --features visual-test` | Generated 24 fixed 1440 x 1000 logical-pixel references at 2x Retina output; all files have distinct hashes |
| Phase 0/current release benchmark | Comparable 30-sample frame-time and allocation results recorded in [release evidence](./07-release-evidence.md) |
| Live macOS Accessibility tree | Attached to a temporary application bundle; verified roles, names, values, disabled/mixed/invalid examples, form error description, Dialog/Sheet/Notification Close labels, and Escape dismissal |

The macOS verification host is macOS 27.0 on Apple M1 Pro. A temporary application bundle was used only to make the local Story executable attachable to the native Accessibility enumerator; no bundle artifact is committed.

## Phase exit audit

| Phase | Status | Remaining evidence or implementation |
|---|---|---|
| Phase 0 | Complete | Pinned identities/source map, frozen before-state Light/Dark captures, pre-change keyboard/accessibility source inventory, independent controls, platform differences, and same-source performance samples are recorded |
| Phase 1 | Complete | Independent Color/Style ownership, registry, metrics, motion lifecycle, reduced motion, validation, and tests are implemented |
| Phase 2 | Partial | TextArea and NumberInput are present in the P0 Story; deterministic IME and keyboard tests pass, while fixed pointer/focus captures and live IME/keyboard checks on all platforms remain |
| Phase 3 | Partial | Fixed open/closing Popover references cover four placements and constrained edges; complete live nested-focus/dismissal review and non-Popover family placement review on all platforms remain |
| Phase 4 | Partial | Disclosure motion, deterministic Accordion focus and `Enter`/`Space` activation, Calendar/DatePicker/Slider fixed states, and English/Simplified Chinese/Traditional Chinese layout captures are complete; live keyboard/drag/date-range interaction review remains |
| Phase 5 | Partial | The 1,000-row uniform-list comparison plus Table default/hover/selected/active and empty/loading layout references are recorded; live focused-row and full DataTable interaction review remain |
| Phase 6 | Partial | Windows/Linux interaction and accessibility review plus the three-platform CI result remain release blockers |

## Release verification still required

- Verify keyboard, focus, IME, dismissal, and accessibility behavior on Windows and Linux.
- Complete macOS live keyboard focus traversal, IME composition, nested focus restoration, and screen-reader announcement review.
- Capture the missing non-Popover overlay-family and pointer/focus evidence listed above.
- Connect the application-level reduced-motion override to product-specific platform preference plumbing where required.

Headless Metal screenshots and the macOS native tree are complementary evidence. Windows and Linux results are not inferred from macOS compilation or tests.

The headless renderer requires unsandboxed access to macOS services. Both the frozen Phase 0 export and the current checkout completed the same release benchmark on the same host. See [release evidence](./07-release-evidence.md).
