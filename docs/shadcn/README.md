# shadcn/ui alignment plan

## Status

This directory defines the implementation plan for aligning existing GPUI Component controls with shadcn/ui. It does not represent completed implementation.

| Baseline | Revision |
|---|---|
| GPUI Component | `e1570bdc` |
| Local shadcn/ui checkout | `/Users/jon/Desktop/ui` |
| shadcn/ui | `607e8a9717fe6ff0d374ba74c651012f9c052534` |
| Baseline date | 2026-08-05 |

The shadcn revision is intentionally pinned. Future upstream changes must be reviewed and recorded before they alter this plan.

## Goal

Improve the visual consistency, interaction states, and motion of existing GPUI Component controls while preserving native desktop behavior and keeping the public Rust API stable where practical.

The work is successful when:

1. Existing components use a coherent visual vocabulary in light and dark themes across multiple Style Presets.
2. Interactive components define default, hover, focus, active, disabled, loading, selected, and invalid states where applicable.
3. Overlay and disclosure motion uses shared duration and easing tokens, including exit motion.
4. Keyboard, focus, dismissal, and accessibility behavior remains correct on macOS, Windows, and Linux.
5. Story examples, focused tests, Clippy, and the cross-platform CI suite pass.
6. Performance-sensitive components show no material regression against the recorded baseline.
7. Color Theme and Style Preset can be selected independently without creating two runtime sources of truth.

## Reference hierarchy

shadcn/ui v4 separates style from behavior. A single source directory is therefore insufficient as the specification.

| Concern | Canonical reference | Usage |
|---|---|---|
| Visual language | `apps/v4/registry/styles/style-vega.css` | Spacing, radius, density, shadow, color roles, component states |
| Interaction composition | `apps/v4/registry/bases/radix/ui` | Trigger/content structure, state model, focus and dismissal intent |
| Compatibility examples | `apps/v4/registry/new-york-v4/ui` | Existing shadcn API vocabulary and example coverage |
| Accessibility cross-check | `apps/v4/registry/bases/aria/ui` and `bases/base/ui` | Keyboard and semantic behavior comparison |
| Native behavior | Existing GPUI Component contracts, Apple HIG, Fluent Design | Final authority when Web and desktop behavior differ |

`Vega` is the default alignment target because shadcn describes it as clean, neutral, and familiar, and it is closest to the current GPUI Component default theme. The first release also includes `Nova` and `Maia` as distinct Style Presets. Nova validates compact density; Maia validates comfortable spacing and larger radii. Mira, Lyra, Luma, Sera, and Rhea remain deferred until the first three presets prove the abstraction.

## Theme architecture decision

Color Theme and Style Preset are independent configuration inputs:

```text
Color Theme + Style Preset -> resolved global Theme -> cx.theme()
```

- Color Theme owns semantic colors, renderable backgrounds, light and dark selection, syntax highlighting, and existing typography settings.
- Style Preset owns density, shared control metrics, radius, elevation, focus treatment, overlay metrics, and motion.
- `Theme` remains the only runtime source read by components.
- `StylePreset` is configuration input. Components never read preset configuration directly.
- Existing `Theme.radius`, `Theme.radius_lg`, and `Theme.shadow` remain canonical runtime fields.
- Existing Theme JSON keeps working through a `ThemeDefault` compatibility mode.

The complete contract is defined in [Style Preset architecture](./05-style-presets.md).

## Scope

### Included

- Existing component visual states and variants.
- Independent Color Theme and Style Preset selection.
- Theme token mapping and shared component metrics.
- Built-in Vega, Nova, and Maia presets.
- Shared motion primitives and reduced-motion behavior.
- Overlay lifecycle, focus, keyboard, and dismissal behavior.
- Story state matrices, focused tests, documentation, and performance checks.
- Additive API improvements required to express existing shadcn states.

### Excluded

- Copying React, Tailwind, Radix, Base UI, or React Aria implementation code.
- Replacing GPUI entities and events with a DOM-like abstraction.
- Pixel parity where it conflicts with desktop behavior or platform accessibility.
- Adding every shadcn-only component during the alignment work.
- Redesigning Dock, Editor, Markdown, HTML, TextView, or native window chrome around Web conventions.
- Implementing all eight shadcn styles in the first release.
- Letting components branch on preset names or read preset configuration directly.

## Constraints

- Preserve existing public methods and serialized theme keys unless a breaking change is separately approved.
- Prefer additive theme fallbacks over mandatory new fields.
- Keep one resolved runtime authority in `Theme`; do not mirror mutable values under `theme.style.*`.
- Switching Color Theme must not change an explicitly selected Style Preset.
- Switching Style Preset must not change colors or syntax highlighting.
- Keep component implementation direct. Shared helpers require at least three real consumers.
- Buttons retain the desktop default cursor; link controls may use the pointer cursor.
- Existing GPUI accessibility roles, labels, values, focus handles, and native menus must not regress.
- Motion communicates state. Decorative bounce and elastic effects are not part of the target language.
- One pull request should cover one foundation or one coherent component family.

## Plan documents

| Document | Purpose |
|---|---|
| [Baseline and gap analysis](./01-baseline-and-gaps.md) | Current capabilities, source findings, and architectural gaps |
| [Component matrix](./02-component-matrix.md) | Existing component scope, priorities, references, and expected work |
| [Implementation roadmap](./03-roadmap.md) | Ordered pull-request batches, dependencies, and exit criteria |
| [Verification strategy](./04-verification.md) | Required state, behavior, visual, platform, and performance checks |
| [Style Preset architecture](./05-style-presets.md) | Runtime authority, registry, compatibility, preset scope, and selection rules |

## Decision rules

When sources disagree, apply the following order:

1. Accessibility and data safety.
2. Native desktop platform behavior.
3. Existing documented GPUI Component contract.
4. shadcn interaction intent.
5. Vega visual specification.
6. Exact Tailwind values.

Any deviation from the pinned references must be recorded in the relevant component row before implementation.
