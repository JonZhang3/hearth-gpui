# Component matrix

## Priority model

| Priority | Meaning |
|---|---|
| P0 | Foundation or high-frequency control; blocks consistent work elsewhere |
| P1 | Overlay, navigation, or composite control with shared behavior |
| P2 | Display, data, and supporting controls |
| P3 | GPUI-specific component; retain native design and adopt only shared tokens |
| Backlog | shadcn component without a required existing GPUI equivalent |

## Preset coverage rule

Every component consumes resolved metrics from `Theme`; it must not switch rendering logic on `Vega`, `Nova`, or `Maia` names. Full state matrices use Vega. Nova and Maia require representative geometry coverage for each component family so preset compatibility is verified without tripling every visual case.

## P0: core controls

| GPUI Component | shadcn reference | Primary work | Acceptance focus |
|---|---|---|---|
| Button, ButtonIcon, ButtonGroup, DropdownButton | `button`, `button-group` | Vega variants and dimensions, leading/trailing icon slots, explicit Spinner composition, action-group layout, focus, pressed, and disabled states | Stable dimensions; desktop cursor; all variants in light and dark; child callbacks remain independent |
| Toggle | `toggle`, `toggle-group` | Align on, off, hover, focus, outline, and grouped radii | Selected state is visually persistent and accessible |
| Checkbox | `checkbox` | Align control size, radius, mark, invalid ring, and checked transition | Checked, unchecked, indeterminate, disabled, invalid |
| Radio | `radio-group` | Align ring, indicator, focus, invalid, and disabled states | Arrow-key behavior and group selection remain native |
| Switch | `switch` | Align track, thumb, compact size, checked color, and thumb motion | No delayed state mismatch; AccessKit checked state preserved |
| Input and TextArea modes | `input`, `textarea` | Align height, padding, border, focus ring, invalid state, placeholder, selection, and dark surface | IME, selection, masking, password, disabled, read-only |
| NumberInput | `input`, `input-group` | Apply the control contract without losing numeric step behavior | Keyboard stepping and buttons share focus treatment |
| OtpInput | `input-otp` | Align cells, active ring, caret, invalid state, and grouping | Paste, focus movement, selection, disabled, invalid |
| Select | `select` | Align trigger, value, caret, menu surface, item states, and overlay motion | Keyboard navigation, focus restore, close lifecycle |
| Combobox | `combobox`, `command` | Align trigger, search field, result rows, empty state, and overlay motion | Async content, filtering, selection, escape, focus restore |
| Tooltip | `tooltip` | Align compact typography, arrow, surface, side-aware entry, delay, and exit | No flicker at window edges; reduced motion; action hints |
| Dialog and AlertDialog | `dialog`, `alert-dialog` | Align overlay, content radius, spacing, close button, scale and fade motion | Focus trap, Escape, outside click rules, close animation, focus restore |

## P1: overlays and navigation

| GPUI Component | shadcn reference | Primary work | Acceptance focus |
|---|---|---|---|
| Popover | `popover` | Shared anchored-surface style and enter or exit lifecycle | Side-aware motion, outside click, nested overlay behavior |
| HoverCard | `hover-card` | Surface, timing, safe hover corridor, side-aware motion | Open and close delays; trigger-to-card movement |
| Menu, ContextMenu, AppMenuBar | `dropdown-menu`, `context-menu`, `menubar` | Item density, checked rows, submenu indicator, destructive state, overlay motion | Keyboard traversal, submenus, async content, native menu boundaries |
| Sheet | `sheet` | Overlay, side-specific transition, spacing, close affordance | Four placements, resizing, focus trap, exit before unmount |
| Notification | `sonner` | Surface hierarchy, action placement, stacking, enter or exit motion | Queue replacement, autohide, manual close, action close, no overlap |
| Accordion | `accordion` | Trigger typography, focus, indicator, measured content reveal | No clipped content; repeated toggles; keyboard control |
| Collapsible | `collapsible` | Use the same disclosure lifecycle with a simpler API | Dynamic content height and reduced motion |
| Tabs and TabBar | `tabs` | Align segmented and line variants, focus, selected text, indicator motion | Rapid switching, scrolling tabs, focus navigation |
| Sidebar | `sidebar` | Align density, item states, group labels, floating and inset surfaces, collapse motion | Off-canvas and icon collapse; no layout jitter |
| Form and fields | `field`, `form` | Standardize label, description, required, invalid, error, and group spacing | Error association and consistent control composition |
| Calendar and DatePicker | `calendar`, `popover` | Align day states, range states, navigation, surface, and trigger | Locale, keyboard, disabled dates, range edges |
| Slider | `slider` | Align track, range, thumb, focus ring, hover feedback | Drag, click, keyboard, range mode, release event |

## P2: display and data components

| Family | Components | Alignment work |
|---|---|---|
| Status and feedback | Alert, Badge, Progress, Spinner, Skeleton, Rating | Semantic variants, typography, compact spacing, loading motion, reduced motion |
| Identity and labels | Avatar, Label, Kbd, Icon, Breadcrumb | Size vocabulary, icon proportions, fallback surfaces, muted hierarchy |
| Data surfaces | Table, DataTable, List, Tree, DescriptionList | Header and row hierarchy, hover and selected states, borders, density; retain virtualization |
| Containers | GroupBox, Settings, Separator, Resizable | Surface and border roles, headings, handles, spacing |
| Navigation | Pagination, Stepper | Button state reuse, current state, disabled state, focus order |
| Scrolling | Scrollable, Scrollbar, VirtualList | Shared colors and radii only; retain platform-aware behavior and performance |
| Pickers | ColorPicker | Reuse overlay, form-control, focus, and motion contracts |

## P3: GPUI-specific surfaces

These components do not need shadcn structural parity. They consume the shared color, typography, focus, surface, and motion tokens where appropriate.

| Component family | Policy |
|---|---|
| Dock and Tiles | Preserve drag, split, tabs, serialization, and desktop panel behavior |
| Editor, TextView, Markdown, HTML | Preserve high-performance text architecture and selection behavior |
| Chart and Plot | Preserve chart semantics; use shared chart color roles and tooltip surfaces |
| TitleBar, StatusBar, WindowBorder | Platform behavior and window manager support remain authoritative |
| NativeMenu | Native platform menus are not restyled to match Web menus |
| FocusTrap and Root | Infrastructure only; verify through consumer components |

## Alias mapping

Equivalent concepts use different names. They do not require duplicate components.

| shadcn concept | GPUI Component equivalent |
|---|---|
| `input-otp` | `OtpInput` |
| `radio-group` | `Radio` and `RadioGroup` |
| `scroll-area` | `Scrollable` and `Scrollbar` |
| `dropdown-menu`, `context-menu`, `menubar` | `menu` module and `DropdownButton` |
| `sonner` | `Notification` |
| `textarea` | Multiline `Input` or `TextArea` mode |
| `field` | `form::field` |
| `input-group` | Existing input adornments and button groups; audit before adding a wrapper |

## Backlog, not part of alignment

The following shadcn components have no necessary one-to-one existing target. Adding them requires a separate product case and API proposal:

- AspectRatio
- Attachment
- Bubble
- Carousel
- Command as a standalone public component
- Direction provider
- Drawer distinct from Sheet
- Empty
- Item
- Marker
- Message and MessageScroller
- NativeSelect
- NavigationMenu

Component parity is not a release criterion for the alignment project.
