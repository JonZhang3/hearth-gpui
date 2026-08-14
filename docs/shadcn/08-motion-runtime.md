# Motion runtime architecture

## Decision

GPUI Component uses a renderer-independent `gpui-component-motion` crate for timing and
interruption state. `ui::animation` is the GPUI Adapter that owns element state, frame requests,
style interpolation, and deferred completion callbacks. Components own interaction state and
decide what completion means, such as unmounting an overlay or restoring focus.

This architecture borrows Motion's separation of mutable motion values, animation generators, and
frame drivers. It does not copy Motion's TypeScript implementation or introduce DOM, React, WAAPI,
or a second frame scheduler. GPUI remains the only layout, paint, and frame authority.

## Runtime contract

- Animation time starts during the first GPUI layout sample, not when component state changes.
- Retargeting samples the current visual value before creating the new tween.
- Reversing toward the previous endpoint shortens duration by remaining distance.
- A stable element ID preserves motion state across component rerenders.
- Completion is generation-safe, emitted once, and deferred outside layout and paint.
- Reduced motion writes the final value and completes lifecycle work in the same frame.
- Style Presets provide semantic duration and easing values; the motion crate does not read Theme
  state or branch on preset IDs.

The core v1 supports interruptible scalar tweens. GPUI-specific interpolation for `Pixels`, `Hsla`,
and styled effects remains in `ui::animation`, avoiding an orphan-rule-driven public wrapper API.

## Adoption

HoverCard is the first lifecycle consumer. Its mirrored directional enter and exit translations
complete from sampled motion state, including rapid reversal from the current offset. Opacity motion
is intentionally disabled because GPUI cannot yet composite subtree opacity as one isolated layer.
Sheet uses the same sampled transition adapter to move the surface fully outside its window edge
for enter and exit motion. Auto-sized vertical sheets complete an offscreen measurement frame before
starting the visible transition. Its backdrop remains visually stable for the full lifecycle and is
removed with the Sheet after the exit motion completes.
Select also uses stable sampled motion state for its 100 ms enter/exit translation while retaining
content through exit. Opacity is intentionally omitted until GPUI can composite the complete popup
subtree as one layer. The next adoption group is Popover, Tooltip, Combobox, and
ContextMenu, followed by modal surfaces and disclosure/layout motion.

Spring, inertia, gestures, sequences, layout projection, scroll timelines, and a standalone frame
loop are outside v1. Layout-independent subtree transforms, isolated opacity, backdrop filters, and
GPU compositing remain GPUI renderer work recorded in [Deferred TODO](./TODO.md).

## Verification contract

- Pure motion tests use explicit `Instant` values for deterministic start, midpoint, completion,
  reversal, stale completion, zero-duration, and reduced-motion coverage.
- GPUI tests verify lifecycle retention, input suppression during exit, and generation ownership.
- Story review verifies placement-aware motion, rapid reversal, and final visual state with Vega as
  the default baseline.
