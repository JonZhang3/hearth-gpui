# Style Preset architecture

## Decision

GPUI Component will support multiple Style Presets that can be selected independently from Color Themes. The first built-in presets are Vega, Nova, and Maia.

`Theme` remains the only resolved runtime source used by components. `StylePreset` is configuration input and must not become a second mutable theme tree.

```text
ThemeConfig + StylePreset -> resolved Theme -> cx.theme() -> components
```

## Ownership boundary

| Concern | Owner |
|---|---|
| Semantic colors | Color Theme |
| Gradient and solid backgrounds | Color Theme through `ThemeTokens` |
| Light and Dark selection | Color Theme |
| Syntax highlighting | Color Theme |
| Existing font configuration | Color Theme for compatibility |
| Density and control geometry | Style Preset |
| Radius and elevation | Style Preset |
| Focus treatment | Style Preset |
| Overlay spacing and geometry | Style Preset |
| Motion duration and easing | Style Preset |
| Reduced motion | Platform or application accessibility setting |
| Keyboard and focus behavior | Component implementation |
| Local exceptional styling | `Styled` component override |

Style Presets never own semantic colors. Components never use a preset name to choose behavior or layout.

## Runtime authority

The resolved `Theme` remains flat and compatible with current component access:

```rust
pub struct Theme {
    pub colors: ThemeColor,
    pub tokens: ThemeTokens,

    pub style_name: SharedString,
    pub density: Density,
    pub control_metrics: ControlMetrics,
    pub overlay_metrics: OverlayMetrics,
    pub focus_metrics: FocusMetrics,
    pub motion: MotionMetrics,

    // Existing canonical runtime fields.
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub shadow: bool,
}
```

Do not introduce duplicate runtime paths such as both `theme.radius` and `theme.style.radius`. A preset applies its values to the canonical `Theme` fields. Preset configuration and resolved runtime data may contain corresponding values, but only resolved `Theme` data is mutable and consumed by components.

## Minimal preset model

Start with stable shared groups only:

```rust
pub struct StylePreset {
    pub name: SharedString,
    pub density: Density,
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub shadow: bool,
    pub controls: ControlMetrics,
    pub overlays: OverlayMetrics,
    pub focus: FocusMetrics,
    pub motion: MotionMetrics,
}
```

Recommended metric responsibilities:

| Group | Initial fields |
|---|---|
| `ControlMetrics` | `xs`, `sm`, `md`, `lg` heights; horizontal padding; icon size; icon gap |
| `OverlayMetrics` | Padding; content gap; radius; side offset |
| `FocusMetrics` | Ring width; ring offset |
| `MotionMetrics` | Fast, normal, slow, emphasis durations; enter and exit easing |

Do not add per-component metric structures until at least three components need the same field. Table density, Sidebar width, Dialog maximum width, and similar exceptional values stay local until real reuse appears.

`Density` is preset metadata and a public semantic label. Resolved metrics, not a global density multiplier, determine component geometry.

## Selection model

```rust
pub enum StyleSelection {
    ThemeDefault,
    Preset(SharedString),
}
```

| Selection | Behavior |
|---|---|
| `ThemeDefault` | Existing Theme JSON `radius`, `radius.lg`, and `shadow` remain authoritative |
| Explicit preset | Preset-owned fields override legacy shape fields |
| Return to `ThemeDefault` | Reapply appearance fields from the currently selected ThemeConfig |
| Unknown preset | Keep the last valid resolved style and return an actionable error |

An invalid preset must not partially mutate `Theme`.

## Independent application

Color and style use separate operations:

```rust
Theme::apply_color_theme(theme_config, cx);
Theme::apply_style_preset(style_preset, cx);
```

Required invariants:

1. Applying a Color Theme updates colors, tokens, Light or Dark selection, syntax highlighting, and existing typography settings.
2. Applying a Color Theme does not change explicit Style Preset fields.
3. Applying a Style Preset updates style-owned resolved fields only.
4. Applying a Style Preset does not change colors, tokens, syntax highlighting, or Theme Mode.
5. Both operations refresh affected windows without rebuilding application business state.
6. Selection order produces the same result for the same Color Theme and Style Preset pair.

## Backward compatibility

Current Theme JSON combines colors with `font.*`, `radius`, `radius.lg`, and `shadow`. These keys remain readable.

Compatibility rules:

- Existing applications that never select a Style Preset remain in `ThemeDefault`.
- `ThemeDefault` preserves current JSON behavior.
- Explicit preset selection takes ownership of radius and shadow without rewriting the loaded ThemeConfig.
- Removing explicit selection restores the current ThemeConfig values.
- Existing public `Theme.radius`, `Theme.radius_lg`, and `Theme.shadow` fields remain available.
- New Style fields use defaults when deserializing older runtime state.
- No existing theme file requires migration for the first release.

Deprecating legacy appearance keys is not part of this project.

## Style Registry

Use a small registry with no first-release file watcher:

```rust
pub struct StyleRegistry {
    presets: HashMap<SharedString, Rc<StylePreset>>,
}
```

Required API capabilities:

- Register built-in or application-provided presets.
- Look up a preset by stable name.
- Return a sorted list for selectors.
- Reject duplicate built-in names predictably.
- Apply only a fully resolved valid preset.

Deferred capabilities:

- External Style JSON.
- Directory watching and hot reload.
- Preset inheritance.
- Runtime downloading.
- Per-component registries.
- Tailwind-like class composition.

## Initial presets

| Preset | shadcn source | Purpose |
|---|---|---|
| Vega | `style-vega.css` | Default, neutral, familiar, standard density |
| Nova | `style-nova.css` | Compact controls with reduced padding and margins |
| Maia | `style-maia.css` | Comfortable spacing and larger radii |

Mira overlaps the compact validation provided by Nova. Lyra, Luma, Sera, and Rhea introduce additional typography or surface decisions. They are deferred until the first three presets prove that components consume resolved metrics without preset-specific branches.

## Preset implementation rules

- Presets provide data, not component implementations.
- A component may branch on semantic state or capability, never preset name.
- A preset cannot alter keyboard, focus, dismissal, selection, or accessibility behavior.
- A preset cannot introduce component-specific public API.
- Shared metrics require at least three consumers.
- Local `Styled` overrides remain the final layer for exceptional application use.
- Motion must communicate state and must not use bounce or elastic easing.

## Resolution order

```text
Library defaults
  -> selected Color Theme
  -> selected Style Preset or ThemeDefault compatibility values
  -> platform accessibility overrides
  -> local component Styled overrides
```

Platform accessibility overrides include reduced motion and other system-enforced behavior. They are not persisted as preset identity.

## Acceptance criteria

- Vega, Nova, and Maia produce visibly distinct geometry with the same Color Theme.
- One preset produces identical geometry across different Color Themes.
- Color switching does not change explicit preset identity or metrics.
- Style switching does not change colors, Theme Mode, or syntax highlighting.
- `ThemeDefault` reproduces existing Theme JSON appearance.
- Components read only resolved `Theme` values.
- No component contains `if preset == ...` rendering branches.
- Invalid selection does not leave partially applied style data.
- Existing custom themes load without modification.
