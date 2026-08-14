# Style Preset architecture

## Decision

GPUI Component supports Color Themes and Style Presets as independent selections. Vega, Nova, and Maia are built in. Vega is the explicit default.

```text
Color Theme ---------------------> Theme colors, typography, syntax
Style Preset -> validation -> Rc<StylePreset> -> Theme.style
                                              -> components
```

There is one runtime authority: `Theme`. Style data is immutable and nested under `Theme.style`; no flat duplicate radius, shadow, control, or motion fields exist.

## Ownership

| Concern | Owner |
|---|---|
| Semantic colors and renderable backgrounds | Color Theme |
| Light and Dark selection | Color Theme |
| Syntax highlighting | Color Theme |
| Existing font configuration | Color Theme |
| Density and control geometry | Style Preset |
| Radius and elevation | Style Preset |
| Focus ring geometry | Style Preset |
| Overlay spacing and geometry | Style Preset |
| Component motion duration | Style Preset |
| Reduced motion | Platform or application accessibility setting |
| Keyboard, focus, dismissal, and selection behavior | Component implementation |
| Exceptional local styling | `Styled` override |

Style Presets never own semantic colors, fonts, icons, component behavior, or component-specific public APIs.

## Public runtime API

```rust
pub struct Theme {
    pub colors: ThemeColor,
    pub tokens: ThemeTokens,
    pub style: Rc<StylePreset>,
    // Color Theme, typography, syntax, mode, and runtime behavior settings.
}

pub struct StylePreset {
    pub id: SharedString,
    pub name: SharedString,
    pub density: Density,
    pub radii: RadiusMetrics,
    pub controls: ControlMetrics,
    pub avatars: AvatarMetrics,
    pub overlays: OverlayMetrics,
    pub modals: ModalMetrics,
    pub focus: FocusMetrics,
    pub disclosure: DisclosureMetrics,
    pub elevation: ElevationMetrics,
    pub motion: MotionMetrics,
    pub data: DataMetrics,
}
```

Components consume semantic metrics only:

```rust
let metrics = cx.theme().style.controls.for_size(size);

div()
    .h(metrics.height)
    .px(metrics.padding_x)
    .rounded(cx.theme().style.radii.md)
```

The following legacy fields are removed:

- `Theme.radius`
- `Theme.radius_lg`
- `Theme.shadow`
- Theme JSON `radius`
- Theme JSON `radius.lg`
- Theme JSON `shadow`

This is an approved breaking API redesign. No `ThemeDefault`, fallback appearance ownership, or downstream compatibility layer is provided.

## Shared metric contracts

### Controls

`ControlSizeMetrics` is the shared contract for `xs`, `sm`, `md`, and `lg`:

| Field | Meaning |
|---|---|
| `height` | Stable outer control height |
| `padding_x` | Horizontal padding for text controls |
| `icon_edge_padding` | Horizontal padding for compact or icon-leading controls |
| `gap` | Gap between icon, label, and caret |
| `icon_size` | Default icon size inside the control |

Custom `Size::Size(height)` preserves the requested height and uses medium ancillary metrics.

### Radius and elevation

`RadiusMetrics` exposes `sm`, `md`, `lg`, and `xl`. Components choose by surface role, never by preset name. `ElevationMetrics.enabled` controls shared shadows without changing semantic surface colors.

### Overlay

`OverlayMetrics` owns content padding, content gap, side offset, and enter scale. Overlay lifecycle, placement, focus restoration, and dismissal remain component behavior.

### Modal

`ModalMetrics` owns AlertDialog default/small widths, content and header spacing, Media geometry, footer treatment, overlay opacity, and surface ring opacity. Vega uses the full 512 px confirmation surface with an unseparated footer, Nova uses compact geometry and a tinted separated footer, and Maia uses comfortable geometry with a stronger backdrop. Dialog and AlertDialog consume these values without branching on preset ids.

### Avatar

`AvatarMetrics` owns Avatar diameter, fallback text/icon size, badge geometry, outline width, group overlap, and group ring width. The current built-in presets use the pinned shadcn geometry: 24 px small, 32 px default, 40 px large, 8 px group overlap, and a 2 px background ring. Semantic image labels, fallback content, loading behavior, and group ordering remain component behavior.

### Focus

`FocusMetrics` owns ring width and offset. Ring color remains `ThemeColor.ring`.

### Disclosure

`DisclosureMetrics` owns Accordion trigger/content padding, title gap, indicator size, trigger/frame radius, default frame policy, and optional open-state tint. Accordion consumes these semantic metrics without branching on preset ids: Vega and Nova resolve to plain divided lists, while Maia resolves to a unified rounded frame.

### Motion

| Token | Default | Use |
|---|---:|---|
| `fast` | 100 ms | Immediate overlay and feedback transitions |
| `normal` | 150 ms | Standard component state transitions |
| `slow` | 200 ms | Disclosure, indicator, and structural transitions |
| `emphasis` | 250 ms | Standard Dialog, notification, and deliberate emphasis |
| `loading` | 1 s | Repeating Skeleton, Spinner, and indeterminate Progress cycles |

`enter_easing`, `exit_easing`, and `move_easing` select semantic curves from `MotionEasing`; components do not choose curves by preset name. `OverlayMetrics::enter_offset` resolves translation for Top, Right, Bottom, and Left placements. `OverlayLifecycle` owns the interruptible `closed -> opening -> open -> closing -> closed` state machine and rejects stale completion generations.

Transitions use restrained cubic easing. Bounce and elastic easing are excluded. GPUI's application-level `reduce_motion` preference renders static animation states, while `effective_motion_duration` removes delayed unmount. Reduced motion is an accessibility override and is not persisted as Style identity.

### GPUI-native data surfaces

`DataMetrics` centralizes Table and DataTable row heights and cell padding. Virtualization, scrolling, selection, and data behavior remain GPUI-native.

## Built-in values

Button control geometry is pinned to the local shadcn/ui revision:

| Preset | Size | Height | Padding X | Icon edge | Gap | Icon |
|---|---|---:|---:|---:|---:|---:|
| Vega | xs / sm / md / lg | 24 / 32 / 36 / 40 | 8 / 10 / 10 / 10 | 6 / 6 / 8 / 8 | 4 / 4 / 6 / 6 | 12 / 16 / 16 / 16 |
| Nova | xs / sm / md / lg | 24 / 28 / 32 / 36 | 8 / 10 / 10 / 10 | 6 / 6 / 8 / 8 | 4 / 4 / 6 / 6 | 12 / 14 / 16 / 16 |
| Maia | xs / sm / md / lg | 24 / 32 / 36 / 40 | 10 / 12 / 12 / 16 | 8 / 8 / 10 / 12 | 4 / 4 / 6 / 6 | 12 / 16 / 16 / 16 |

Fonts and icon libraries declared by upstream shadcn styles are intentionally excluded. Applications keep their Color Theme typography and GPUI icon source.

## Registry and selection

```rust
StyleRegistry::register(preset, cx)?;
let preset = StyleRegistry::get("vega", cx);
let presets = StyleRegistry::sorted_styles(cx);

Theme::set_style("nova", cx)?;
Theme::set_color_theme(theme_config, cx);
```

Registry rules:

1. Stable ids are non-empty and unique.
2. Control heights are positive and ordered from `xs` to `lg`.
3. Overlay scale is within `0..=1`.
4. Registration validates the whole preset before insertion.
5. Unknown selection returns an actionable error and preserves the active preset.
6. Built-in and application-provided presets use the same API.
7. Sorted selection uses display name; persisted identity uses stable id.

Color and Style selection are order-independent:

1. `Theme::set_color_theme` changes Color Theme data and preserves `Theme.style`.
2. `Theme::set_style` changes `Theme.style` and preserves colors, mode, typography, and syntax highlighting.
3. Both operations refresh windows without rebuilding application state.

Story Gallery persists Color Theme name and Style Preset id separately.

## Extension policy

Deferred until demonstrated by real use:

- External Style JSON.
- Directory watching and hot reload.
- Preset inheritance.
- Runtime downloading.
- Per-component registries.
- Tailwind-like class composition.
- Font or icon switching in Style Presets.

Add a shared metric only when at least three consumers need the same semantic value. Keep one-off widths, maximum sizes, and component-specific layout values local.

## Acceptance criteria

- Vega, Nova, and Maia produce visibly distinct geometry with the same Color Theme.
- One Style Preset produces identical geometry across different Color Themes.
- Color switching preserves Style identity and metrics.
- Style switching preserves colors, mode, typography, and syntax highlighting.
- No component branches on `vega`, `nova`, or `maia`.
- Invalid registration and unknown selection do not partially mutate runtime state.
- Button, form control, focus, overlay, navigation motion, and data metrics use shared semantic fields.
- Avatar, AvatarBadge, AvatarGroup, and AvatarGroupCount consume shared Avatar metrics without preset-id branches.
- P3 components retain GPUI-native behavior and consume shared tokens only where applicable.
