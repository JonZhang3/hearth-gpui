# Component alignment checklist

This checklist tracks component-level shadcn alignment and GPUI-specific optimization work.

- `[x]` means the component has a dedicated implementation commit covering its relevant API,
  visual states, interaction, accessibility, Story, or documentation scope.
- `[ ]` means no complete component-level alignment or optimization batch has been recorded yet.
- Shared Style Metrics, overlay lifecycle, or accessibility infrastructure alone do not mark a
  component as complete.
- A checked item may still have an intentional renderer limitation recorded in
  [Deferred TODO](./TODO.md), or outstanding cross-platform release verification in
  [Implementation status](./06-implementation-status.md).

## P0: core controls

- [x] Button (`f23bd94d`)
- [x] ButtonIcon (`f23bd94d`)
- [x] ButtonGroup (`f23bd94d`)
- [x] DropdownButton — GPUI-native compound action and Style Preset integration
- [x] Toggle and ToggleGroup
- [x] Checkbox (`d1adc7fd`)
- [x] Radio, RadioGroupItem, and RadioGroup
- [x] Switch (`ea447322`)
- [x] Input and TextArea modes (`75b0f1bc`)
- [x] InputGroup (`09d3799a`)
- [x] NumberInput — GPUI-native optimization and Style Preset integration
- [x] OtpInput
- [x] Select
- [x] Combobox
- [x] Tooltip
- [x] Dialog (`9381cbde`)
- [x] AlertDialog (`4ad50d07`)

## P1: overlays and navigation

- [x] Popover
- [x] HoverCard (`5dc85ea2`)
- [ ] Menu and DropdownMenu
- [ ] ContextMenu
- [ ] AppMenuBar
- [x] Sheet
- [ ] Notification
- [x] Accordion (`7a30099b`)
- [x] Collapsible (`7a30099b`)
- [x] Tabs and TabBar
- [ ] Sidebar
- [ ] Form and Field
- [x] Calendar (`b41326e4`)
- [x] Range Calendar (`b41326e4`)
- [ ] DatePicker
- [ ] Slider

## P2: display and data components

### Status and feedback

- [x] Alert (`dca13b41`)
- [x] Badge (`37bd7931`)
- [x] OverlayBadge (`37bd7931`)
- [x] Progress
- [ ] Spinner
- [ ] Skeleton
- [ ] Rating

### Identity and labels

- [x] Avatar (`e43b5245`)
- [x] AvatarGroup (`e43b5245`)
- [x] Label (`71019db9`)
- [x] Kbd (`708769ce`)
- [ ] Icon
- [x] Breadcrumb (`c1830227`)

### Data surfaces

- [ ] Table
- [ ] DataTable
- [x] List — GPUI-native optimization, not shadcn structural parity (`5d8f21bf`)
- [ ] Tree
- [ ] DescriptionList

### Containers

- [x] Card (`dbea164b`)
- [ ] GroupBox
- [ ] Settings
- [ ] Separator
- [ ] Resizable

### Navigation

- [ ] Pagination
- [ ] Stepper

### Scrolling

- [ ] Scrollable
- [ ] Scrollbar
- [ ] VirtualList

### Pickers

- [ ] ColorPicker

## P3: GPUI-specific surfaces

These components do not require shadcn structural parity. They remain unchecked until a dedicated
GPUI-native optimization batch is completed.

- [ ] Dock and Tiles
- [ ] Editor
- [ ] TextView
- [ ] Markdown and HTML rendering
- [ ] Chart and Plot
- [ ] TitleBar
- [ ] StatusBar
- [ ] WindowBorder
- [ ] NativeMenu
- [ ] FocusTrap and Root infrastructure

## Backlog: shadcn components without a required GPUI equivalent

- [ ] AspectRatio
- [ ] Attachment
- [ ] Bubble
- [ ] Carousel
- [ ] Command as a standalone public component
- [ ] Direction provider
- [ ] Drawer distinct from Sheet
- [ ] Empty
- [ ] Item
- [ ] Marker
- [ ] Message and MessageScroller
- [ ] NativeSelect
- [ ] NavigationMenu

Backlog items remain outside the current release criterion. Checking one requires a separate product
case, API proposal, implementation batch, Story, documentation, and verification scope.
