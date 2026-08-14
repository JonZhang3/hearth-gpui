# Baseline and gap analysis

## Current strengths

GPUI Component already contains substantial shadcn-inspired infrastructure. The work is an alignment and completion effort, not a ground-up redesign.

| Area | Current implementation |
|---|---|
| Semantic colors | `ThemeColor` covers primary, secondary, muted, accent, danger, border, input, ring, popover, chart, sidebar, and desktop-specific roles |
| Renderable tokens | `ThemeToken` and `ThemeTokens` support both a representative solid color and a renderable background |
| Theme metrics | Global font, radius, large radius, shadow, scrollbar, list, sheet, and tile settings |
| Theme loading | `ThemeRegistry` loads JSON, tracks Light and Dark configurations, watches Native theme directories, and supports embedded WASM themes |
| Component states | Button and form controls already model multiple variants, sizes, focus rings, disabled states, and active states |
| Overlay infrastructure | `Root`, focus traps, global overlay state, Window extensions, and dismissal actions |
| Motion infrastructure | GPUI `Animation`, a local `Transition`, easing functions, and interpolation for pixels, points, colors, width, and height |
| Desktop behavior | Native menus, window chrome, keyboard focus, AccessKit integration, and platform-specific code |
| Validation surfaces | Desktop Story Gallery, WASM Gallery, unit tests, `#[gpui::test]`, examples, and three-platform CI |

Recent form-control refinement is already present in baseline commit `ab99cfbc`. New work must audit and extend it instead of reverting it to older shadcn values.

## shadcn/ui repository findings

The pinned shadcn repository is a generator and registry, not one fixed component library:

- It supports three behavior bases: Radix UI, Base UI, and React Aria.
- It supports eight visual styles: Vega, Nova, Maia, Lyra, Mira, Luma, Sera, and Rhea.
- Legacy `new-york-v4` remains for compatibility.
- Component structure is separated from `.cn-*` style rules.
- Most overlay entrances use opacity, approximately 95 percent scale, and a small side-aware translation.
- Typical overlay durations are 100 ms; sheets and sidebar transitions commonly use 200 ms; more structural navigation motion uses 300 ms.
- Form controls consistently combine border change, focus ring, invalid ring, dark input surface, and small shadow.
- Component source relies on primitive libraries for focus management and accessibility. Tailwind classes alone are not a behavior specification.

## Gaps to close

### 1. The reference target is not encoded in the repository

The current theme credits shadcn, but it does not identify the exact shadcn commit, behavior base, or visual style. Upstream now has multiple valid outputs.

Required result:

- Keep the pinned source revision in this directory.
- Use Vega plus Radix as the default comparison target.
- Record intentional desktop deviations per component.

### 2. Motion values are component-local

Current components use several hard-coded durations and easing choices. Examples include 150 ms, 200 ms, 250 ms, and component-specific cubic Bézier curves. Skeleton currently uses bounce-based easing, while product motion should avoid bounce.

Required result:

- Introduce a minimal shared motion scale.
- Distinguish enter, exit, state, and structural transitions.
- Add reduced-motion behavior.
- Remove arbitrary timing only after visual comparison, not by mechanical replacement.

Recommended initial tokens:

| Token | Initial value | Intended use |
|---|---:|---|
| `motion_fast` | 100 ms | Tooltip, menu, popover, overlay fade |
| `motion_normal` | 150 ms | Checkbox, switch, small control feedback |
| `motion_slow` | 200 ms | Sheet, sidebar, tab indicator |
| `motion_emphasis` | 250 ms | Notification entry and dismissal |

Values remain provisional until the Phase 0 motion capture is reviewed.

Motion values belong to Style Preset configuration. Reduced-motion preference remains an application or platform accessibility setting and overrides the selected preset.

### 3. `Transition` lacks part of the required vocabulary

The local transition helper supports fade, translation, width, and height. shadcn overlay motion also relies on scale and placement-aware transform origins. Exit lifecycle handling is component-specific.

Required result:

- Add scale only if GPUI can animate it without layout work.
- Support side-aware translation through one placement mapping.
- Define an explicit visible, closing, unmounted lifecycle for overlays.
- Avoid animating layout properties when transform or clipping can express the same behavior.

### 4. Color Theme and component shape are coupled

The baseline `ThemeConfig` stored colors together with font, radius, large radius, and shadow. This prevented a Color Theme from being freely combined with multiple component styles.

Required result:

- Introduce `StylePreset` as a configuration input and `StyleRegistry` as its lookup mechanism.
- Keep `Theme.style` as the single resolved Style authority.
- Remove flat `theme.radius`, `theme.radius_lg`, and `theme.shadow` fields.
- Add independent Color Theme and Style Preset actions.
- Use Vega as the explicit default Style Preset.
- Applying a Color Theme must not overwrite `Theme.style`.
- Reject unknown Style ids without changing the active Style.

The initial built-in presets are Vega, Nova, and Maia. The presets validate standard, compact, and comfortable component geometries without requiring component forks.

### 5. Theme roles and component metrics are partially implicit

The current theme has most shadcn color roles, but card roles are represented indirectly through background or GroupBox. Component height, padding, icon size, focus-ring width, and shadow selection remain distributed across component files.

Required result:

- Publish a documented semantic mapping before adding theme fields.
- Add `card` and `card_foreground` only if an existing component needs a distinct surface.
- Centralize only cross-component metrics such as control heights, focus-ring width, and overlay radius.
- Remove legacy Style keys from Theme JSON so ownership is explicit.

### 6. State coverage is inconsistent

Not every interactive component visibly and behaviorally defines the same applicable state set.

Required state vocabulary:

| State | Required behavior |
|---|---|
| Default | Stable neutral appearance |
| Hover | Pointer feedback without changing layout |
| Focus visible | Keyboard-visible ring and border treatment |
| Active or pressed | Immediate input acknowledgement |
| Disabled | No action dispatch, reduced emphasis, correct accessibility state |
| Loading | No duplicate action, stable width where practical |
| Selected or checked | Persistent semantic state |
| Invalid | Error border and ring without relying on color alone |
| Open or closed | Correct overlay or disclosure lifecycle |

Only states meaningful to a component are required.

### 7. Exit motion is not uniform

Some components delay unmounting for dismissal animation, while others primarily animate entry or immediately change mounted state.

Required result:

- Establish one lifecycle contract for Dialog, AlertDialog, Sheet, Popover, Tooltip, HoverCard, Menu, Select, Combobox, and Notification.
- Preserve focus restoration and prevent input during closing.
- Ensure Escape, outside click, action dismissal, and programmatic dismissal take the same close path.

### 8. Visual verification is mostly manual

The Story Gallery is extensive, but there is no committed state matrix or pinned reference capture for this alignment.

Required result:

- Add deterministic Story sections for component state comparison.
- Capture reference dimensions and states from the pinned shadcn checkout.
- Compare representative P0 states under Vega, Nova, and Maia with the same Color Theme.
- Compare at least one preset under multiple Light and Dark Color Themes.
- Use image comparison where deterministic rendering is available.
- Keep behavior tests authoritative when raster output differs by platform font rendering.

## API policy

| Change type | Policy |
|---|---|
| Existing component behavior | Preserve unless currently incorrect or inaccessible |
| New state or variant | Additive builder method or enum variant |
| Theme/Style API | Breaking redesign is approved |
| Style Preset registration | Rust API with validation and stable ids |
| Existing radius and shadow keys | Removed from Theme JSON and runtime Theme |
| Default appearance | Allowed, documented as visual alignment |
| Default keyboard behavior | Change only to fix a documented mismatch |
| Serialized layout | Must remain readable; Theme appearance keys may break |
| New dependency | Requires a separate justification and size review |

## Known non-equivalences

- Tailwind responsive breakpoints do not map directly to desktop windows.
- DOM Portal maps to GPUI Root and anchored overlay infrastructure.
- `data-state` maps to Rust state and render branches.
- CSS custom properties map to `Theme`, `ThemeColor`, and component settings.
- CSS keyframes map to GPUI animation state and lifecycle.
- ARIA maps to AccessKit and platform accessibility APIs.
- Browser pointer conventions do not override desktop cursor conventions.
