---
title: Spinner
description: Displays an indeterminate loading status.
---

# Spinner

`Spinner` communicates that work is in progress when no completion percentage is available. Its default appearance follows the pinned shadcn Spinner: a 16px circular loader that inherits the surrounding text color and rotates continuously.

## Import

```rust
use hearth_gpui::spinner::{Spinner, SpinnerAnimation, SpinnerVariant};
```

## Usage

```rust
Spinner::new()
```

The default accessible name is `Loading`. Provide a task-specific name when it adds useful context:

```rust
Spinner::new()
    .aria_label("Loading projects")
```

## Sizes

The default size is 16px. `Spinner` implements `Sizable`, including exact custom sizes:

```rust
Spinner::new().with_size(px(12.))
Spinner::new()                    // 16px
Spinner::new().with_size(px(24.))
Spinner::new().with_size(px(32.))
```

The GPUI-specific `.xsmall()`, `.small()`, and `.large()` helpers remain available for composition with existing controls.

## Variants and animation

Two built-in variants provide coordinated icon and motion defaults:

```rust
// LoaderCircle + continuous linear rotation (default, shadcn-aligned)
Spinner::new().variant(SpinnerVariant::Circular)

// Original segmented Loader + semantic eased rotation (GPUI classic)
Spinner::new().variant(SpinnerVariant::Classic)
```

The icon and animation can be overridden independently. Explicit overrides remain authoritative regardless of builder call order:

```rust
Spinner::new()
    .icon(IconName::LoaderCircle)
    .animation(SpinnerAnimation::SemanticSpin)
    .variant(SpinnerVariant::Circular)
```

## Color and icon

The Spinner inherits the current text color by default. Override it only when the surrounding semantic color is unsuitable:

```rust
Spinner::new().color(cx.theme().muted_foreground)

Spinner::new()
    .icon(IconName::Loader)
    .color(cx.theme().blue)
```

The default icon is `IconName::LoaderCircle`, matching shadcn's circular Loader2 appearance. Any compatible `Icon` can be supplied through `.icon(...)`.

## Composition

```rust
Button::new("submit")
    .icon(Spinner::new())
    .label("Submitting")
    .disabled(true)

Badge::new()
    .outline()
    .leading(Spinner::new().xsmall())
    .child("Generating")
```

Spinner also composes with `InputGroupAddon`, Empty states, and other element slots.

## Motion

- Rotation: one complete turn.
- Duration: the active Style Preset's semantic `motion.loading()` duration; built-in presets use 1 second.
- Easing: Circular uses linear; Classic uses the active Style Preset's move easing.
- Lifecycle: infinite while mounted, with no enter, exit, opacity, or scale transition.
- Reduced Motion: renders the loader statically.

`SpinnerAnimation::SemanticSpin` restores the original Spinner behavior by applying the active Style Preset's move easing to a complete rotation. `.ease(...)` can override either animation's default easing, while `LinearSpin` remains the shadcn-aligned default.

## Stable IDs

`Spinner::new()` derives a stable ID from its call site. When an iterator creates multiple Spinners from the same source location, provide structural IDs explicitly:

```rust
items.into_iter().enumerate().map(|(index, _)| {
    Spinner::new().id(ElementId::named_usize("row-spinner", index))
})
```

## API

| Method | Purpose |
| --- | --- |
| `new()` | Create a 16px circular loading Spinner |
| `id(id)` | Override the stable element ID |
| `aria_label(text)` | Set the accessible loading status name |
| `variant(variant)` | Select a coordinated icon and motion preset |
| `icon(icon)` | Replace the circular loader icon |
| `animation(animation)` | Independently select LinearSpin or SemanticSpin rotation |
| `color(color)` | Override the inherited text color |
| `with_size(size)` | Set a named or exact size |
| `ease(easing)` | Override the default linear easing |
