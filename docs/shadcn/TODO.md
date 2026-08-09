# Deferred TODO

This document records intentionally deferred shadcn alignment work. An item remains here until its implementation scope, platform contract, and verification requirements are approved.

## Input Group custom controls and logical RTL

**Status:** Typed Input integration implemented; shared direction and generic-control contracts deferred

Input Group currently accepts the repository's `Input` as its typed control slot. Addons remain freely composable through `ParentElement`, while focus, disabled, invalid, accessibility, and surface motion ownership stay explicit.

Do not broaden the root to arbitrary `AnyElement` controls until GPUI components share a contract for exposing `FocusHandle`, disabled state, invalid state, and accessibility ownership. Visual introspection of an opaque element is not an acceptable substitute.

`InlineStart` and `InlineEnd` currently resolve to left and right because GPUI Component has no shared logical layout-direction contract. Revisit their physical placement after the repository defines inherited LTR/RTL direction for layout, pointer hit regions, keyboard navigation, and accessibility ordering. Acceptance requires mirrored inline geometry without changing block placement or caller APIs.

## Element-level backdrop filters for Dialog overlays

**Status:** Renderer support deferred; semantic overlay fallback implemented

The pinned shadcn Dialog overlay uses a translucent black surface and conditionally applies `backdrop-blur-xs`. Dialog now resolves the Vega, Nova, and Maia overlay opacity from semantic Modal Metrics, but GPUI cannot blur only the content behind one element subtree.

Do not add a Dialog-only framebuffer path. Revisit this with the shared compositing-layer work described under element-level blend modes. The renderer contract must define offscreen backdrop capture, rounded and nested clipping, scale-factor behavior, animation invalidation, hit testing, and equivalent Metal, WGPU, Direct3D, Web, and headless output.

Acceptance requires Vega and Nova blur parity without changing Maia's stronger overlay, no blur leakage outside the overlay bounds, correct nested modal composition, and no material regression to overlay frame time or batching.

## Rounded overflow masks

**Status:** Renderer support deferred; Card workaround implemented

GPUI currently clips `overflow_hidden` descendants with a rectangular content mask, even when the parent has rounded corners. Card therefore applies its resolved edge radius directly to `CardMedia` and rounded Header/Footer sections, and image media is painted by `CardMedia::image`. Custom media must paint its background on the CardMedia surface rather than on a square child.

Card resolves all four edge radii after `Styled` overrides and applies the relevant corners authoritatively to top/trailing CardMedia and edge Header/Footer sections. Section-specific radius refinements cannot diverge from the owning Card at an outer edge.

### Deferred renderer design

Native CSS-like rounded descendant clipping should use a complete axis-aligned rounded Clip Chain rather than adding one radius to the existing rectangular `ContentMask`. A single rounded rectangle cannot represent the intersection of multiple nested rounded overflow containers.

The preferred design is:

- Represent every rectangular or rounded overflow region as a clip node containing bounds, four corner radii, and a parent clip id.
- Store the resolved clip-chain id on every rendered primitive instead of duplicating one rectangular mask per primitive.
- Preserve conservative rectangular bounds for CPU culling, then evaluate every rounded node in the active chain in the fragment shader with an anti-aliased rounded-rectangle signed-distance function.
- Apply the same clip geometry to hit testing so invisible corner regions cannot receive pointer, hover, scroll, or drag interactions.
- Cover Quad, Shadow, text and monochrome sprites, image and polychrome sprites, Underline, Path, Surface, deferred drawing, and offscreen layers.
- Implement equivalent Metal, WGPU, Direct3D, Web, and headless behavior without component-specific renderer branches.

### Dependency strategy

GPUI is consumed from the pinned Zed Git dependency. Develop this work in an explicit GPUI fork or contribute it upstream. Do not modify Cargo's Git checkout cache or add a Card-only renderer exception.

### Acceptance criteria

- Uniform and asymmetric corner radii clip backgrounds, images, text, paths, shadows, and surfaces consistently.
- Nested rounded and rectangular overflow regions produce their exact geometric intersection.
- Border widths produce the correct inner clipping radii.
- Scroll offsets, transforms, deferred drawing, offscreen layers, and multiple scale factors preserve clip geometry.
- Pointer and scroll hit testing excludes clipped corner regions.
- Metal, WGPU, Direct3D, Web, and headless captures agree within documented anti-aliasing tolerances.
- Rectangular-only clipping retains the existing fast path, batching behavior, and frame-time baseline.

## HoverCard transform and accessibility parity

**Status:** Deferred platform capabilities

HoverCard currently uses mirrored placement-aware translations for enter (`8px -> 0`) and exit
(`0 -> 8px`) while intentionally omitting opacity changes. Closing remains mounted until the exit
translation completes. This avoids the
visible per-primitive alpha composition artifact until GPUI can composite the completed subtree as
one isolated layer. It does not emulate `zoom-in-95` / `zoom-out-95` by changing layout dimensions.
GPUI needs a
layout-independent element-subtree transform before the 0.95 scale can be applied without text
reflow or geometry jitter.

GPUI also multiplies element opacity into each background, border, shadow, glyph, image, and child
primitive independently. CSS opacity composites the completed subtree as an isolated layer. A
shared offscreen compositing layer is required before HoverCard can match overlapping translucent
pixels and text antialiasing without adding a component-specific renderer path.

The pinned web primitive also excludes preview content from the screen-reader tree. GPUI does not
currently expose an AccessKit subtree equivalent to `aria-hidden`. HoverCard therefore remains a
non-modal, non-focusable preview and its documentation requires essential information to remain
available outside the preview.

Remaining renderer acceptance requires interruption-safe enter and exit scaling, isolated subtree
opacity, correct transformed hit testing and clipping, no layout change between animation frames,
and a subtree accessibility-hiding primitive that does not suppress the trigger.

## Element-level blend modes and Avatar outline parity

**Status:** Deferred

### Current behavior

- Avatar uses the Color Theme `border` token with GPUI's normal source-over alpha composition.
- AvatarGroup and AvatarBadge use the Color Theme `background` token for their separation rings.
- Style Presets continue to own only outline width, group ring width, overlap, and other geometry.

### Upstream behavior

The pinned shadcn Avatar renders its outline with `border-border`, `mix-blend-darken` in Light mode, and `mix-blend-lighten` in Dark mode. The semantic border token is aligned, but the final pixel composition is not identical because the pinned GPUI renderer has no element-level blend mode.

### Deferred renderer work

Do not add an Avatar-only rendering exception. Revisit this work when blend modes, backdrop filters, isolated opacity groups, or similar compositing features have multiple real consumers.

The preferred long-term design is a GPUI compositing layer that can render an element subtree into an offscreen target and composite it with the parent target. This layer should support:

- `Normal`, `Darken`, and `Lighten` blend modes as an initial contract.
- Correct alpha composition for translucent source colors.
- Nested layers, content masks, rounded clipping, and stable paint order.
- Shared infrastructure for future backdrop filters and isolated opacity groups.
- Metal, WGPU, Direct3D, Web, and headless renderer parity.

A primitive-only `Min` or `Max` blend pipeline may be used for an isolated experiment, but it is not sufficient as the public element-level contract because translucent CSS blend semantics require backdrop-aware composition.

### Dependency strategy

GPUI is currently consumed from the pinned Zed Git dependency. Renderer work must be developed in an explicit GPUI fork or contributed upstream. Cargo's Git checkout cache must never be modified as project source.

### Acceptance criteria

- Avatar Light and Dark outlines match the pinned shadcn reference without hard-coded component colors.
- Blend behavior is deterministic for opaque and translucent source colors.
- Nested clipping and overlapping elements preserve paint order.
- Metal, WGPU, Direct3D, Web, and headless captures produce equivalent results within documented color-space tolerances.
- Normal blending retains the existing batching and frame-time baseline.
- AvatarGroup and AvatarBadge background separation rings remain unchanged.
