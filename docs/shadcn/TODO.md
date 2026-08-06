# Deferred TODO

This document records intentionally deferred shadcn alignment work. An item remains here until its implementation scope, platform contract, and verification requirements are approved.

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
