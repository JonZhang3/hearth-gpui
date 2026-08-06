---
name: align-shadcn-component
description: Compare an existing GPUI Component implementation with the pinned local shadcn/ui sources, produce a decision-complete Vega alignment plan, and implement that plan only after explicit approval. Use for component-by-component shadcn visual, API, state, motion, interaction, accessibility, Story, or documentation alignment work in this repository.
---

# Align shadcn Component

Use a strict two-phase workflow. Never combine analysis and implementation in one phase.

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

Perform this phase when the user requests comparison, optimization, alignment, or a plan and no previously approved plan exists.

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
4. Classify each finding as exact alignment, GPUI semantic equivalent, intentional desktop divergence, defect, missing capability, or deferred platform limitation.
5. Resolve source conflicts using the priority order in `docs/shadcn/README.md`. Preserve accessibility and native desktop behavior ahead of pixel parity.

Return:

- the inspected source paths and revision status;
- a concise difference table with current behavior, shadcn behavior, impact, and recommended decision;
- a decision-complete implementation plan covering API changes, component behavior, Style Metrics, Story/docs/tests, acceptance criteria, and intentional deviations.

Do not edit files in Phase 1. Stop after the plan and wait for an explicit implementation instruction such as `Implement the plan.`

## Phase 2: implement the approved plan

Enter this phase only when the user explicitly requests implementation and an approved plan is present in the conversation or repository. If no approved plan exists, run Phase 1 and stop.

1. Re-read the approved plan and inspect `git status` before editing. Preserve unrelated user changes.
2. Implement only the approved scope. Do not infer permission for breaking API changes unless the user explicitly allowed them.
3. Keep components direct and GPUI-native:
   - read colors from the Color Theme;
   - read geometry, density, radius, elevation, focus treatment, overlays, and motion from semantic Style Metrics;
   - never branch on `vega`, `nova`, or `maia` IDs;
   - add shared metrics only when the repository's Style Preset contract justifies them;
   - preserve focus, keyboard, accessibility, dismissal, and exit-lifecycle contracts.
4. Update the component Story and synchronize English and Chinese documentation when their behavior or API changes.
5. Add or update focused tests for meaningful builders, state transitions, accessibility contracts, and regression-prone behavior. Follow `CLAUDE.md` for the commands that are required in the current task.
6. Record renderer limitations or explicitly deferred parity work in `docs/shadcn/TODO.md`; do not hide them behind approximations.
7. Verify formatting, compilation, linting, focused behavior, and `git diff --check` in proportion to the change. Never claim a check that was not run.

Report only actual changes, reasons, intentional differences, and verification results.
