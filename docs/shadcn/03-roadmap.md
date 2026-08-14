# Implementation roadmap

## Delivery model

- Implement foundations before component-specific polish.
- Keep each pull request independently reviewable and revertible.
- Do not mix visual alignment with unrelated API refactors.
- Update the relevant Story and tests in the same pull request as a component change.
- Preserve a working Gallery after every batch.

## Phase 0: freeze references and establish baselines

### Work

1. Record the Hearth GPUI and shadcn commit hashes in Gallery metadata or developer documentation.
2. Add a dedicated `Shadcn Alignment` Story section with light and dark theme switching.
3. Add independent Color Theme and Style Preset controls to the alignment Story.
4. Add deterministic state rows for P0 components.
5. Capture pinned Vega, Nova, and Maia reference states at fixed dimensions and scale.
6. Record current startup, idle, overlay, scrolling, and rapid-toggle performance samples.
7. Inventory current keyboard and accessibility behavior before changing appearance.

### Exit criteria

- P0 components have a before-state capture.
- Reference captures identify shadcn style, base, commit, viewport, and theme.
- The Gallery can demonstrate Color Theme changes without Style changes and Style changes without Color changes.
- Performance measurements include hardware, build profile, display refresh rate, and scenario.
- Known platform differences are documented.

## Phase 1: Color Theme and Style Preset foundation

### Pull request 1: independent selection contract

- Make Vega the explicit default Style Preset.
- Separate Color Theme application from Style Preset application.
- Keep the global `Theme` as the only resolved runtime authority.
- Ensure Color Theme switching preserves an explicit preset.
- Ensure Style Preset switching preserves colors and syntax highlighting.
- Remove Style fields from Theme JSON and keep Color Theme loading independent.
- Persist Color Theme and Style Preset names independently in the Story application.

### Pull request 2: semantic color mapping

- Document and test the mapping between shadcn roles and `ThemeColor` or `ThemeTokens`.
- Add missing roles only when no safe existing semantic role exists.
- Remove legacy `radius`, `radius.lg`, and `shadow` keys from bundled Theme JSON.
- Regenerate and validate the theme schema if fields change.

### Pull request 3: Style Registry and shared metrics

- Add a minimal `StyleRegistry` with registration, lookup, and sorted listing.
- Define built-in Vega, Nova, and Maia presets.
- Define one size contract for control height, horizontal padding, icon size, icon gap, and radius.
- Reuse the existing `Sizable` vocabulary: `xs`, `sm`, `md`, and `lg`.
- Keep exceptional metrics local when fewer than three components share them.
- Add focused tests for metric resolution rather than pixel assertions throughout every component.
- Do not add file watching or external Style JSON in the first implementation.

### Pull request 4: motion contract

- Add named duration and easing values to resolved `Theme.style` metrics.
- Add scale and placement-aware translation only if supported without layout animation.
- Define reduced-motion behavior as an accessibility override, not a preset value.
- Define overlay states: closed, opening, open, closing.
- Create a shared close-completion mechanism that cannot dispatch duplicate dismissal callbacks.
- Replace bounce-based Skeleton motion with a restrained pulse or shimmer.

### Exit criteria

- No P0 component needs a new hard-coded duration.
- Theme tests cover Color Theme loading without Style mutation.
- Vega, Nova, and Maia resolve to distinct metrics without component-name branching.
- Color Theme and Style Preset switching are independent in both orders.
- Vega is selected on initialization and after deserializing runtime Theme state without Style data.
- Motion helpers have focused interpolation, restart, cancellation, and reduced-motion tests.
- The foundation adds no DOM or Tailwind abstraction.

## Phase 2: core controls

Recommended pull-request batches:

1. Button family and Toggle.
2. Checkbox, Radio, and Switch.
3. Input, multiline input, NumberInput, and OtpInput.
4. Select and Combobox triggers.
5. Tooltip.

Each batch must:

- Apply the shared size, focus, invalid, disabled, and loading contracts.
- Include light and dark Story state matrices.
- Verify representative geometry in Vega, Nova, and Maia.
- Use the redesigned Theme/Style API consistently.
- Verify keyboard, mouse, and accessibility state.
- Compare default, hover, focus, active, selected, disabled, loading, and invalid states where applicable.

### Exit criteria

- P0 non-modal controls meet their component-matrix acceptance focus.
- No control changes size when focus, loading, or validation state changes.
- Rapid interactions do not leave stale visual state or detached tasks.

## Phase 3: overlay system

Recommended pull-request batches:

1. Anchored surface primitive for Tooltip, Popover, HoverCard, Menu, Select, and Combobox.
2. Dialog and AlertDialog lifecycle.
3. Sheet lifecycle and placement motion.
4. Notification stacking and dismissal motion.

Required lifecycle:

```text
closed -> opening -> open -> closing -> closed
```

The lifecycle must guarantee:

- Closing content remains mounted only for the exit duration.
- Closing content does not accept new actions.
- Escape, outside click, action close, and programmatic close converge on the same path.
- Focus restores once after unmount.
- Nested overlays close in the correct order.
- Reopening during close has a defined and tested result.

### Exit criteria

- All overlay consumers use the common motion and lifecycle contract or document why they cannot.
- Placement-aware motion is correct at all four sides and window edges.
- Focus trap, focus restoration, and nested dismissal tests pass.

## Phase 4: disclosure, navigation, and forms

Recommended pull-request batches:

1. Accordion and Collapsible.
2. Tabs and TabBar.
3. Sidebar.
4. Form field contract.
5. Calendar, DatePicker, and Slider.

### Exit criteria

- Disclosure animation supports dynamic content without clipping or stale height.
- Tab and sidebar structural transitions do not animate through repeated layout recalculation when transform or clipping is available.
- Forms expose consistent label, description, required, invalid, and error treatment.
- Locale-dependent layouts remain valid for English, Simplified Chinese, and Traditional Chinese.

## Phase 5: display and data surfaces

Recommended pull-request batches:

1. Alert, Badge, Avatar, Kbd, Label.
2. Progress, Spinner, Skeleton.
3. Table and DataTable.
4. List, Tree, DescriptionList, GroupBox, and Settings.
5. Pagination, Stepper, Resizable, and Scrollable.

### Exit criteria

- Data components preserve virtualization and scrolling performance.
- Selected, hovered, focused, and active row states remain distinguishable.
- Loading and empty states do not cause layout instability.
- GPUI-specific extensions retain their existing capabilities.

## Phase 6: documentation and release

### Work

1. Update English and Simplified Chinese component documentation together.
2. Document visual default changes and any additive API.
3. Document Color Theme and Style Preset selection, composition, and custom preset registration.
4. Add a migration section for users overriding component styles.
5. Run the complete verification matrix.
6. Perform an accessibility review on all supported platforms.
7. Compare release performance against Phase 0.
8. Record deferred parity items and upstream shadcn revision used.

### Exit criteria

- Documentation, Story, and implementation describe the same states and defaults.
- CI passes on macOS, Linux, and Windows.
- Performance exceptions are measured and approved.
- No unresolved P0 or P1 accessibility regression remains.

## Dependency order

```text
Reference baseline
  -> Independent Color and Style selection
  -> Style Registry and metrics
  -> Motion and lifecycle
  -> Core controls
  -> Overlay consumers
  -> Disclosure and navigation
  -> Data and supporting components
  -> Release verification
```

## Stop conditions

Pause a batch and split the work when any of these occurs:

- A public API break outside the approved Theme/Style redesign is required.
- A serialized theme or layout format would become unreadable.
- A new runtime dependency is required.
- A GPUI limitation requires layout animation with measurable frame regression.
- Native accessibility behavior would be replaced by visual imitation.
- More than one component family needs unrelated architectural changes.
