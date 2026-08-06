# Repository Instructions

GPUI Component is a Rust workspace for building cross-platform desktop UI components with GPUI.

Read [CLAUDE.md](CLAUDE.md) before starting any task. It is the authority for project commands, repository structure, component architecture, and available project skills.

## shadcn Alignment Baseline

- Vega is the default visual baseline for shadcn alignment.
- Screenshots from the shadcn website commonly use `base-nova`; unless the user explicitly requests Nova, treat those screenshots as references for capabilities, variants, and composition only.
- Do not change Vega dimensions, radii, shadows, or density to match Nova screenshots without explicit approval.
- Nova and Maia remain optional Style Presets and are not the default acceptance baseline.
- Components must consume semantic Style Metrics and must never branch on the `vega`, `nova`, or `maia` preset ID.

## Detailed Context

- Alignment architecture and workflow: [docs/shadcn/README.md](docs/shadcn/README.md)
- Style Preset contracts and values: [docs/shadcn/05-style-presets.md](docs/shadcn/05-style-presets.md)
- Project skills: [`skills/`](skills/) and [`.claude/skills/`](.claude/skills/)
