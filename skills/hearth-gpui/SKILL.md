---
name: hearth-gpui
description: How to use the hearth-gpui UI library in GPUI applications. Use when building UIs with hearth-gpui components (Button, Input, Select, Dialog, Tabs, Sidebar, List, Table, etc.), setting up the library, handling component state, choosing Color Themes or Style Presets, or finding the right component for a given UI need.
---

## Documentation

- **Full reference**: fetch `https://jonzhang3.github.io/hearth-gpui/llms-full.txt`
- **Per-component API**: fetch `https://jonzhang3.github.io/hearth-gpui/docs/components/{name}.md`
  - e.g. `button.md`, `input.md`, `select.md`, `dialog.md`, `data-table.md`
- **Any site page** can be fetched as Markdown by appending `.md` to the URL

## Quick Reference

**Setup** — always required:
```rust
hearth_gpui::init(cx);               // in app.run(), must be first
Root::new(view, window, cx)             // first-level view in every window
```

**Stateless** — use directly in render:
```rust
Button::new("id").label("OK").on_click(|_, _, _| {})
```

**Stateful** — hold `Entity<State>` in struct, pass ref in render:
```rust
// in new():  let input = cx.new(|cx| InputState::new(window, cx));
// in render: Input::new(&self.input)
```

**Sizes**: `.xsmall()` `.small()` `.medium()` (default) `.large()`

**Color Theme**: `cx.theme().primary` · `.background` · `.foreground` · `.border` · `.muted`

**Style Preset**: Vega is the default; Nova and Maia are optional. Switch with
`Theme::set_style("nova", cx)?` and read geometry or motion from `cx.theme().style`.

## Component Catalog

When you need a component, find it here. For full API, fetch its `.md` doc.

### Input & Form
| Component | Import | Notes |
|-----------|--------|-------|
| `Input` | `input::{Input, InputState}` | Stateful. Text, password, mask, validation |
| `NumberInput` | `input::{NumberInput, NumberStep}` | Numeric input built on `InputState` |
| `OtpInput` | `input::{OtpInput, OtpState, OtpInputGroup}` | One-time password composition |
| `InputGroup` | `input::{InputGroup, InputGroupAddon, InputGroupButton}` | Input with semantic addons and actions |
| `Select` | `select::{Select, SelectState}` | Stateful. Dropdown picker |
| `Combobox` | `combobox::{Combobox, ComboboxState}` | Stateful. Searchable select |
| `Checkbox` | `checkbox::Checkbox` | Stateless. `on_click(|&bool, ...|)` |
| `Switch` | `switch::Switch` | Stateless. Toggle |
| `Radio` | `radio::{Radio, RadioGroup}` | Stateless. |
| `Slider` | `slider::{Slider, SliderState}` | Stateful. |
| `Toggle` | `button::{Toggle, ToggleGroup}` | Stateless controlled option set |
| `Rating` | `rating::Rating` | Stateless. |
| `Stepper` | `stepper::Stepper` | Stateless. Increment/decrement |
| `ColorPicker` | `color_picker::{ColorPicker, ColorPickerState}` | Stateful. |
| `DatePicker` | `time::date_picker::{DatePicker, DatePickerState}` | Stateful. |
| `Form` | `form::{v_form, h_form, field}` | Layout container for form fields |

### Display & Feedback
| Component | Import | Notes |
|-----------|--------|-------|
| `Button` | `button::{Button, ButtonGroup}` | Stateless. Default is the primary action variant |
| `Icon` | `{Icon, IconName}` | Stateless. Lucide icons |
| `Badge` | `badge::{Badge, BadgeVariants, OverlayBadge}` | Stateless. Inline labels and target overlays |
| `Avatar` | `avatar::Avatar` | Stateless. |
| `Label` | `label::Label` | Stateless. Form label |
| `Kbd` | `kbd::Kbd` | Stateless. Keyboard key display |
| `Alert` | `alert::Alert` | Stateless. Default or destructive |
| `Spinner` | `spinner::Spinner` | Stateless. Loading indicator |
| `Skeleton` | `skeleton::Skeleton` | Stateless. Loading placeholder |
| `Progress` | `progress::{Progress, ProgressCircle}` | Stateless. |
| `Tooltip` | `tooltip::Tooltip` | Via `.tooltip()` on elements |
| `HoverCard` | `hover_card::{HoverCard, HoverCardState}` | Stateful. |
| `Clipboard` | `clipboard::Clipboard` | Stateless. Copy button |
| `Empty` | `empty::{Empty, EmptyHeader, EmptyContent}` | Empty and no-result states |

### Overlay & Popups
| Component | Import | Notes |
|-----------|--------|-------|
| `Dialog` | `dialog::{Dialog, DialogFooter, DialogClose}` + `WindowExt` | Declarative trigger or `window.open_dialog(...)` |
| `AlertDialog` | `WindowExt` | Via `window.open_alert_dialog(...)` |
| `Sheet` | `sheet::Sheet` + `WindowExt` | Side panel, via `window.open_sheet(...)` |
| `Notification` | `notification::Notification` + `WindowExt` | Via `window.push_notification(...)` |
| `Popover` | `popover::Popover` | Floating overlay |
| `Menu` | `menu::{PopupMenu, DropdownMenu}` | Context menus |
| `DropdownButton` | `button::DropdownButton` | Button with dropdown menu |
| `Command` | `command::{Command, CommandState}` | Searchable command menu or palette |

### Navigation & Layout
| Component | Import | Notes |
|-----------|--------|-------|
| `Tabs` / `TabBar` | `tab::{Tab, TabBar}` | Tabbed interface |
| `Sidebar` | `sidebar::{Sidebar, SidebarMenu, ...}` | App navigation panel |
| `TitleBar` | `title_bar::TitleBar` | Window title bar |
| `Breadcrumb` | `breadcrumb::Breadcrumb` | Navigation breadcrumb |
| `Pagination` | `pagination::Pagination` | Page navigation |
| `Accordion` | `accordion::Accordion` | Collapsible sections |
| `Collapsible` | `collapsible::Collapsible` | Single collapsible |
| `GroupBox` | `group_box::GroupBox` | Labeled container |
| `Card` | `card::{Card, CardHeader, CardContent}` | Structured content surface |
| `AspectRatio` | `aspect_ratio::AspectRatio` | Fixed-ratio layout container |
| `Separator` | `separator::Separator` | Semantic visual divider |
| `Resizable` | `resizable::Resizable` | Draggable split panes |
| `Scrollable` | `scroll::{Scrollable, Scrollbar}` | Scroll container and custom scrollbar |
| `FocusTrapElement` | `FocusTrapElement` | Extension trait for keyboard focus traps |

### Data Display
| Component | Import | Notes |
|-----------|--------|-------|
| `DataTable` | `table::{DataTable, TableState, TableDelegate}` | Stateful. Full-featured table |
| `Table` | `table::{Table, ...}` | Simpler table |
| `VirtualList` | `{v_virtual_list, h_virtual_list}` | High-perf large lists |
| `List` | `list::{List, ListState, ListDelegate}` | Stateful. Searchable list |
| `Tree` | `tree::{Tree, TreeState, TreeDelegate}` | Stateful. Hierarchy |
| `DescriptionList` | `description_list::DescriptionList` | Key-value pairs |
| `Settings` | `setting::Settings` | Settings panel |
| `NativeSelect` | `native_select::NativeSelect` | Compact selector backed by the OS menu |

### Charts
| Component | Import | Notes |
|-----------|--------|-------|
| Charts | `chart::{BarChart, LineChart, AreaChart, PieChart}` | Typed chart components |
| `Plot` | `plot::{Plot, IntoPlot}` | Plot trait and `#[derive(IntoPlot)]` |

## Reference Files

- [usage.md](references/usage.md) — setup patterns, component types, common examples
- [style-guide.md](references/style-guide.md) — code style for contributors

When implementing or changing components, preserve the independent Color Theme / Style Preset
contract. Consume semantic metrics such as `theme.style.controls` and `theme.style.radii`; never
branch on the `vega`, `nova`, or `maia` preset id.
