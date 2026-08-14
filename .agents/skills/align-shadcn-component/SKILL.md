---
name: align-shadcn-component
description: Compare an existing Hearth GPUI implementation with the pinned local shadcn/ui sources, produce a decision-complete Vega alignment plan, and immediately implement and verify that plan unless the user explicitly requests analysis or planning only. Use for component-by-component shadcn visual, API, state, motion, interaction, accessibility, Story, or documentation alignment work in this repository.
---

# Align shadcn Component

Use a strict sequential workflow: complete comparison and planning before editing, then continue directly into implementation in the same task. Do not wait for a separate implementation command unless the user explicitly limits the request to analysis or planning.

## Establish the baseline

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/shadcn/README.md`, and the relevant sections of `docs/shadcn/05-style-presets.md`.
2. Treat `/Users/jon/Desktop/ui` as a read-only shadcn/ui checkout. Never modify it.
3. Compare its current revision with the revision pinned in `docs/shadcn/README.md`. Report drift before using newer upstream behavior as a requirement. When possible, inspect the pinned revision with `git show` instead of silently changing the baseline.
4. Use Vega as the default visual baseline. Treat website screenshots that use `base-nova` as capability, variant, and composition references unless the user explicitly selects Nova.

Use this reference order:

| Concern | Source |
|---|---|
| Visual language and state styles | `apps/v4/registry/styles/style-vega.css` |
| Interaction composition | `apps/v4/registry/bases/radix/ui/<component>.tsx` |
| Accessibility cross-check | `apps/v4/registry/bases/aria/ui/<component>.tsx` and `bases/base/ui/<component>.tsx` |
| API and example coverage | Relevant registry examples, docs, and `new-york-v4` sources |
| Desktop behavior | Existing GPUI contracts and native platform conventions |

## Phase 1: compare and plan

Perform this phase when the user requests comparison, optimization, or alignment and no current plan exists. If the user explicitly asks only for comparison, findings, or a plan, complete this phase and stop without editing.

1. Locate the component implementation, related modules, public exports, Story, English and Chinese documentation, focused tests, and all call sites with `rg`.
2. Inspect the corresponding local shadcn sources and examples. Translate their intent into GPUI concepts; do not copy React, Tailwind, Radix, Base UI, or React Aria implementation code.
3. Compare these dimensions:
   - public API, variants, sizes, and composition;
   - element structure, layout, spacing, radius, border, shadow, typography, and icons;
   - semantic colors in light and dark modes;
   - default, hover, active, focused, selected, disabled, loading, invalid, open, and closed states where applicable;
   - enter and exit motion, lifecycle, interruption, and reduced-motion behavior;
   - pointer, keyboard, focus management, dismissal, and accessibility semantics;
   - Style Preset ownership, Story coverage, documentation, and tests.
4. Audit motion from source evidence for every relevant state change:
   - inspect the component root, indicator, icon, overlay, and content separately;
   - record the exact transitioned properties, duration, delay, easing, and initial and final styles;
   - inspect checked, selected, focused, invalid, open, and closed transitions instead of treating motion as overlay-only;
   - inspect conditional mounting, unmounting, exit retention, interruption, rapid reversal, and reduced-motion behavior;
   - compare Vega, Nova, and Maia separately when their style sources differ, while keeping Vega as the default acceptance baseline.
5. Treat explicit `transition-none` and the absence of a transition as required behavior. Never infer fade, scale, bounce, drawing, or other motion from screenshots or visual impression when the pinned source does not define it.
6. Classify each finding as exact alignment, GPUI semantic equivalent, intentional desktop divergence, defect, missing capability, or deferred platform limitation.
7. Resolve source conflicts using the priority order in `docs/shadcn/README.md`. Preserve accessibility and native desktop behavior ahead of pixel parity.

Produce and communicate:

- the inspected source paths and revision status;
- a concise difference table with current behavior, shadcn behavior, impact, and recommended decision;
- motion evidence naming the source class or token, affected property, timing, lifecycle, and GPUI equivalent for each animated state;
- a decision-complete implementation plan covering API changes, component behavior, Style Metrics, Story/docs/tests, acceptance criteria, and intentional deviations.

Do not edit files until the comparison and plan are complete. Share the decision-complete plan as a concise commentary update, record it with the available planning mechanism when appropriate, and immediately continue into Phase 2. Pause only when:

- the user explicitly says to plan, compare, inspect, or diagnose without modifying files;
- a missing user decision would materially change the result;
- implementation requires new authority, an external write, or a meaningful scope expansion;
- the working tree contains overlapping changes that cannot be preserved safely.

## Phase 2: implement the plan

Enter this phase immediately after Phase 1, or directly when a current decision-complete plan already exists in the conversation or repository. Do not require a separate `Implement the plan.` or `开始实施` instruction. When the user requested planning only, do not enter this phase until they later authorize implementation.

1. Re-read the current plan and inspect `git status` before editing. Preserve unrelated user changes.
2. Implement only the planned scope. Do not infer permission for breaking API changes unless the user explicitly allowed them.
3. Keep components direct and GPUI-native:
   - read colors from the Color Theme;
   - read geometry, density, radius, elevation, focus treatment, overlays, and motion from semantic Style Metrics;
   - animate only properties that the pinned shadcn source transitions, and preserve explicit `transition-none` behavior;
   - use semantic Motion Metrics for duration and easing without branching on preset IDs;
   - preserve renderable Theme backgrounds, gradients, and existing component capabilities when interpolation is unavailable;
   - never branch on `vega`, `nova`, or `maia` IDs;
   - add shared metrics only when the repository's Style Preset contract justifies them;
   - preserve focus, keyboard, accessibility, dismissal, and exit-lifecycle contracts.
4. Update the component Story and synchronize English and Chinese documentation when their behavior or API changes.
5. Add or update focused tests for meaningful builders, state transitions, accessibility contracts, and regression-prone behavior. Motion coverage must verify that initial render is stable, forward and reverse transitions reach the correct state, rapid changes do not leave stale state, keyboard and pointer activation do not double-trigger, and reduced motion reaches the final state immediately where testable. Use Story verification when intermediate frames cannot be asserted reliably. Follow `CLAUDE.md` for the commands that are required in the current task.
6. Record renderer limitations or explicitly deferred parity work in `docs/shadcn/TODO.md`; do not silently substitute an approximate animation or hide a limitation behind visual similarity.
7. Verify formatting, compilation, linting, focused behavior, and `git diff --check` in proportion to the change. Never claim a check that was not run.

Report only actual changes, reasons, intentional differences, and verification results.
