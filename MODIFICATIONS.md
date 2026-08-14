# Modification Notice

This repository is derived from the original
[`longbridge/gpui-component`](https://github.com/longbridge/gpui-component)
project and contains substantial modifications.

## Subsequent Project Rename

The derived project is distributed as **Hearth GPUI**. Its Rust packages,
import namespaces, documentation links, examples, and project skills use the
`hearth-gpui` / `hearth_gpui` naming family. References that identify the
original project or its historical issues and pull requests retain the
`gpui-component` name.

Commit `4caf202357c787395710679bdcae8c4eca29ad8e` introduced the following
project-wide changes:

- Added semantic Style Presets and motion foundations for theme-driven UI
  components, with Vega as the default shadcn alignment baseline.
- Revised core UI components, interaction behavior, accessibility semantics,
  stories, examples, documentation, localization, and theme data.
- Added shadcn alignment capture examples, benchmarks, fixtures, and visual
  verification assets.

Unless a file contains a more specific notice, files added or modified by that
commit are part of these modifications. The exact file set is recorded in the
Git history for the commit.

## File-level Notice Coverage

The 233 modified Rust, TOML, and YAML files from this commit carry a
format-appropriate notice at the top of each file.

The following modified files are documented centrally because inserting a
notice would either alter rendered documentation, interfere with Markdown
front matter, use comments unsupported by JSON, or modify generated lockfile
content.

### Modified Files Documented Centrally

- `.theme-schema.json` — Changed JSON theme/schema keys: updated `description`, `type`; added
  `card.background`, `card.foreground`; removed `radius`, `default`, `radius.lg`, `shadow`.
- `CLAUDE.md` — Documented Codex and downstream-user skills and added the `align-shadcn-component` workflow.
- `Cargo.lock` — Updated the generated lockfile: added package resolutions for `anes`, `anstyle`, `cast`,
  `ciborium`, `ciborium-io`, `ciborium-ll`, `clap`, `clap_builder` and 12 more.
- `crates/ui/src/theme/default-theme.json` — Changed JSON theme/schema keys: updated `ring`,
  `switch.background`; added `card.background`, `card.foreground`.
- `docs/docs/components/accordion.md` — Reworked accordion documentation by adding
  `Uncontrolled single Accordion`, `Controlled Accordion`, `Multiple items`, `Non-collapsible single item`,
  `Appearance and disabled state` and 1 more; removed or replaced `Usage`, `Basic Accordion`,
  `Multiple Open Items`, `With Borders`, `Different Sizes` and 7 more.
- `docs/docs/components/alert-dialog.md` — Reworked alert dialog documentation by adding `Basic usage`,
  `Small destructive dialog`, `Imperative usage`, `Custom content and accessibility`, `Behavior` and 1
  more; removed or replaced `Differences from Dialog`, `Import`, `Usage`, `Setup Application Root View`,
  `Basic AlertDialog (Declarative API)` and 23 more.
- `docs/docs/components/alert.md` — Reworked alert documentation by adding `Basic`, `Destructive`, `Action`,
  `Custom Colors`, `Closable` and 3 more; removed or replaced `Usage`, `Basic Alert`, `Alert with Title`,
  `Alert Variants`, `Alert Sizes` and 12 more.
- `docs/docs/components/avatar.md` — Reworked avatar documentation by adding `Basic usage`, `Badge`, `Sizes`,
  `Avatar group`; removed or replaced `Usage`, `Basic Avatar`, `Avatar with Fallback Text`,
  `Avatar Placeholder`, `Avatar Sizes` and 13 more.
- `docs/docs/components/badge.md` — Reworked badge documentation by adding `Icons and Spinner`,
  `Custom Colors`, `Interaction`, `OverlayBadge`, `Count` and 3 more; removed or replaced `Usage`,
  `Badge with Count`, `Variants`, `Badge Sizes`, `Badge Colors` and 9 more.
- `docs/docs/components/button.md` — Reworked button documentation by adding `Sizes`, `Icons and loading`,
  `Rounded`, `Button group`; removed or replaced `Import`, `Usage`, `Basic Button`, `Variants`,
  `Outline Buttons` and 17 more.
- `docs/docs/components/calendar.md` — Expanded calendar documentation with `Framed Calendar`,
  `Week Start and Outside Days`, `Month Motion`.
- `docs/docs/components/chart.md` — Expanded chart documentation with `Composition`, `RadialChart`,
  `Tooltip and motion`.
- `docs/docs/components/checkbox.md` — Expanded checkbox documentation with `Indeterminate and invalid states`,
  `Keyboard and focus`.
- `docs/docs/components/collapsible.md` — Updated collapsible documentation: added references to `id`,
  `Collapsible` and removed references to `open`, `content`.
- `docs/docs/components/color-picker.md` — Documented ColorPicker Style Preset metrics and stable structural
  IDs for independent picker state.
- `docs/docs/components/combobox.md` — Expanded combobox documentation with `Invalid`, `Motion`,
  `Accessibility`.
- `docs/docs/components/data-table.md` — Updated data table documentation: added references to `DataMetrics`,
  `Styled`, `muted` and removed references to `DataTable`, `Sizable`.
- `docs/docs/components/date-picker.md` — Expanded date picker documentation with `Interaction`.
- `docs/docs/components/description-list.md` — Updated description list documentation: added references to
  `DescriptionList`, `Term`, `Definition`.
- `docs/docs/components/dialog.md` — Reworked dialog documentation by adding `Declarative trigger`,
  `Imperative dialog`, `Custom title or description`, `Focus and dismissal`, `Size and position` and 1
  more; removed or replaced `Usage`, `Setup application root view for display of dialogs`, `Basic Dialog`,
  `Form Dialog`, `Dialog with Icon` and 25 more.
- `docs/docs/components/dropdown_button.md` — Expanded dropdown button documentation with `Accessibility`.
- `docs/docs/components/form.md` — Reworked form documentation by adding `Form and Field`, `Basic field`,
  `Validation`, `Field groups`, `Form layout` and 1 more; removed or replaced `Form`, `Usage`,
  `Basic Form`, `Horizontal Form Layout`, `Multi-Column Form` and 24 more.
- `docs/docs/components/group-box.md` — Reworked group box documentation by adding `Basic usage`, `Variants`,
  `Theme and Style Presets`, `Styling layers`, `Long content` and 4 more; removed or replaced `Usage`,
  `Basic GroupBox`, `GroupBox Variants`, `With Title`, `Custom ID` and 13 more.
- `docs/docs/components/hover-card.md` — Reworked hover card documentation by adding `Placement`,
  `Controlled state`, `Custom timing and appearance`, `Interaction and accessibility`, `API`; removed or
  replaced `Basic HoverCard`, `User Profile Preview`, `Custom Timing`, `Positioning`,
  `Custom Content Builder` and 9 more.
- `docs/docs/components/icon.md` — Expanded icon documentation with `Informative Icon`.
- `docs/docs/components/index.md` — Added AspectRatio, Breadcrumb, Card, Command, Empty, and NativeSelect
  entries; removed Tag and updated Badge scope.
- `docs/docs/components/input.md` — Expanded input documentation with `Read-only Input`, `Accessibility`,
  `Style Presets and Motion`, `Read-only and invalid states`.
- `docs/docs/components/kbd.md` — Reworked kbd documentation by adding `Basic`, `Group`,
  `Platform-aware keystrokes`, `Icons and text`, `Input Group` and 2 more; removed or replaced `Usage`,
  `Basic Keyboard Shortcut`, `Common Shortcuts`, `Multiple Modifiers`, `Arrow Keys and Function Keys` and
  12 more.
- `docs/docs/components/label.md` — Reworked label documentation by adding `Basic`, `With an Input`,
  `Disabled`, `Composed Content`, `Project Extensions` and 1 more; removed or replaced `Usage`,
  `Basic Label`, `Label with Secondary Text`, `Text Alignment`, `Text Highlighting` and 15 more.
- `docs/docs/components/list.md` — Reworked list documentation by adding `Basic usage`,
  `Search and keyboard behavior`, `Sections`, `Incremental loading`, `Events and accessibility`; removed
  or replaced `Import`, `Usage`, `Basic List`, `List with Sections`, `List Items with Icons and Actions`
  and 13 more.
- `docs/docs/components/menu.md` — Updated menu documentation: added references to `on_open_change`.
- `docs/docs/components/notification.md` — Expanded notification documentation with `Accessibility`.
- `docs/docs/components/number-input.md` — Expanded number input documentation with
  `Read-only and Invalid States`, `Accessibility`.
- `docs/docs/components/otp-input.md` — Reworked otp input documentation by adding `Basic composition`,
  `Patterns and paste`, `States and sizes`, `Events`, `Accessibility` and 1 more; removed or replaced
  `Usage`, `Basic OTP Input`, `With Default Value`, `Masked OTP Input`, `Different Sizes` and 21 more.
- `docs/docs/components/pagination.md` — Expanded pagination documentation with
  `Accessibility and Keyboard Behavior`.
- `docs/docs/components/plot.md` — Updated plot documentation: added references to `text-xs`.
- `docs/docs/components/popover.md` — Reworked popover documentation by adding `Basic usage`, `Placement`,
  `Dynamic content and manual dismissal`, `Controlled state`, `Custom appearance and trigger methods`;
  removed or replaced `Usage`, `Basic Popover`, `Popover with Custom Positioning`, `View in Popover`,
  `Add content by content method` and 5 more.
- `docs/docs/components/progress.md` — Updated progress documentation: added references to `Progress`,
  `Size::Size(height)`, `tokens.muted`, `tokens.progress_bar`, `theme.tokens.progress_bar` and 3 more and
  removed references to `color(c)`, `theme.progress_bar`.
- `docs/docs/components/radio.md` — Reworked radio documentation by adding `Radio Group`, `Basic usage`,
  `Orientation`, `Labels and descriptions`, `Disabled and invalid states` and 5 more; removed or replaced
  `Radio`, `Usage`, `Basic Radio Button`, `Controlled Radio Button`, `Radio Group (Recommended)` and 16
  more.
- `docs/docs/components/rating.md` — Expanded rating documentation with `Read-only State`,
  `Keyboard and Accessibility`, `Theme and Style Presets`.
- `docs/docs/components/scrollable.md` — Documented stable scroll-area IDs, semantic scrollbar metrics,
  pointer targets, auto-hide timing, and overflow-only painting.
- `docs/docs/components/select.md` — Reworked select documentation by adding `Content Position`,
  `Invalid State`, `Accessibility`, `Style Presets`, `Color Theme`; removed or replaced `Theming`.
- `docs/docs/components/settings.md` — Expanded settings documentation with `Styling`.
- `docs/docs/components/sheet.md` — Reworked sheet documentation by adding `Basic usage`,
  `Placement and size`, `Header composition`, `Close button and backdrop`, `Initial focus` and 3 more;
  removed or replaced `Usage`, `Setup application root view for display of sheets`, `Basic Sheet`,
  `Sheet with Placement`, `Sheet with Custom Size` and 19 more.
- `docs/docs/components/sidebar.md` — Documented sidebar keyboard and accessibility behavior; migrated
  examples to `w` and aligned Badge variants.
- `docs/docs/components/skeleton.md` — Updated skeleton documentation: added references to `muted`, `md`,
  `xl`, `secondary()` and removed references to `skeleton`, `secondary`, `secondary(true)`.
- `docs/docs/components/slider.md` — Expanded slider documentation with `Keyboard and Accessibility`.
- `docs/docs/components/spinner.md` — Reworked spinner documentation by adding `Sizes`,
  `Variants and animation`, `Color and icon`, `Composition`, `Motion` and 2 more; removed or replaced
  `Basic`, `Spinner with Custom Color`, `Spinner Sizes`, `Spinner with Custom Icon`, `Available Icons` and
  15 more.
- `docs/docs/components/status-bar.md` — Expanded status bar documentation with `Style Presets`.
- `docs/docs/components/stepper.md` — Expanded stepper documentation with
  `Accessibility and Keyboard Behavior`, `Methods`.
- `docs/docs/components/switch.md` — Expanded switch documentation with `Invalid and Accessible Name`.
- `docs/docs/components/table.md` — Reworked table documentation by adding `Selected Row`,
  `Horizontal Overflow`, `Style Overrides`; removed or replaced `Without Border (via Styled)`.
- `docs/docs/components/tabs.md` — Expanded tabs documentation with `Keyboard Navigation`, `Style Presets`.
- `docs/docs/components/title-bar.md` — Updated title bar documentation: added references to
  `app_owns_titlebar_drag`, `TitleBar`.
- `docs/docs/components/toggle.md` — Reworked toggle documentation by adding `Basic usage`,
  `Variants and sizes`, `States`, `Leading and trailing icons`, `ToggleGroup` and 5 more; removed or
  replaced `Import`, `Usage`, `Basic Toggle`, `Icon Toggle`, `Controlled Toggle` and 17 more.
- `docs/docs/components/tooltip.md` — Reworked tooltip documentation by adding `Built-in component support`,
  `Compositional API`, `Custom content`, `Timing and arrow`, `API reference` and 3 more; removed or
  replaced `Usage`, `Basic Tooltip with Text`, `Button with Tooltip`, `Tooltip with Action/Keybinding`,
  `Custom Element Tooltip` and 19 more.
- `docs/docs/components/tree.md` — Reworked tree documentation by adding `Basic usage`,
  `Size and Style Presets`, `Disabled nodes`, `Context menu`, `Programmatic control` and 4 more; removed or
  replaced `Usage`, `Basic Tree`, `File Tree with Icons`, `Dynamic Tree Loading`,
  `Tree with Selection Handling` and 13 more.
- `docs/docs/getting-started.md` — Replaced the removed Tag example with a secondary Badge and updated
  imports.
- `docs/docs/theme.md` — Expanded theme documentation with `Color Themes and Style Presets`,
  `Migration from flat appearance fields`.
- `docs/zh-CN/docs/components/accordion.md` — Reworked accordion documentation by adding `非受控单选 Accordion`,
  `受控 Accordion`, `多项展开`, `不允许全部关闭`, `外框与禁用状态` and 1 more; removed or replaced `用法`, `基础 Accordion`,
  `允许同时展开多个项`, `带边框`, `不同尺寸` and 7 more.
- `docs/zh-CN/docs/components/alert-dialog.md` — Reworked alert dialog documentation by adding `基础用法`,
  `Small 危险操作`, `命令式调用`, `自定义内容与可访问性`, `行为` and 1 more; removed or replaced `与 Dialog 的区别`, `导入`, `用法`,
  `配置应用根视图`, `基础 AlertDialog：声明式 API` and 23 more.
- `docs/zh-CN/docs/components/alert.md` — Reworked alert documentation by adding `基础用法`, `Destructive`,
  `Action`, `自定义颜色`, `可关闭 Alert` and 3 more; removed or replaced `用法`, `基础 Alert`, `带标题`, `不同变体`,
  `Alert 尺寸` and 12 more.
- `docs/zh-CN/docs/components/avatar.md` — Reworked avatar documentation by adding `基础用法`, `Badge`, `尺寸`,
  `Avatar Group`; removed or replaced `用法`, `基础 Avatar`, `使用首字母回退`, `占位头像`, `不同尺寸` and 13 more.
- `docs/zh-CN/docs/components/badge.md` — Reworked badge documentation by adding `Variants`, `图标和 Spinner`,
  `自定义颜色`, `交互`, `OverlayBadge` and 4 more; removed or replaced `用法`, `显示数字`, `不同变体`, `不同尺寸`, `颜色` and 9
  more.
- `docs/zh-CN/docs/components/button.md` — Reworked button documentation by adding `图标与加载状态`, `圆角`; removed
  or replaced `导入`, `用法`, `基础按钮`, `变体`, `Outline 按钮` and 15 more.
- `docs/zh-CN/docs/components/calendar.md` — Expanded calendar documentation with `带边框的日历`, `周起始日与跨月日期`,
  `月份切换动画`.
- `docs/zh-CN/docs/components/chart.md` — Expanded chart documentation with `组合`, `RadialChart`,
  `Tooltip 与动效`.
- `docs/zh-CN/docs/components/checkbox.md` — Expanded checkbox documentation with `不确定与无效状态`, `键盘与焦点`.
- `docs/zh-CN/docs/components/collapsible.md` — Updated collapsible documentation: added references to `id`
  and removed references to `open`, `false`, `content`.
- `docs/zh-CN/docs/components/color-picker.md` — Added the Simplified Chinese documentation for ColorPicker
  Style Preset metrics and stable structural IDs.
- `docs/zh-CN/docs/components/combobox.md` — Expanded combobox documentation with `无效状态`, `动效`, `可访问性`.
- `docs/zh-CN/docs/components/data-table.md` — Updated data table documentation: added references to
  `DataMetrics`, `Styled`, `muted` and removed references to `DataTable`, `Sizable`.
- `docs/zh-CN/docs/components/date-picker.md` — Expanded date picker documentation with `交互`.
- `docs/zh-CN/docs/components/description-list.md` — Updated description list documentation: added references
  to `DescriptionList`, `Term`, `Definition`.
- `docs/zh-CN/docs/components/dialog.md` — Reworked dialog documentation by adding `声明式 Trigger`,
  `命令式 Dialog`, `自定义标题和描述`, `焦点与关闭行为`, `尺寸和位置` and 1 more; removed or replaced `用法`, `在应用根视图中渲染 Dialog 图层`,
  `基础对话框`, `表单对话框`, `带图标的对话框` and 19 more.
- `docs/zh-CN/docs/components/dropdown_button.md` — Expanded dropdown button documentation with `无障碍`.
- `docs/zh-CN/docs/components/form.md` — Reworked form documentation by adding `Form 与 Field`, `基础字段`,
  `表单验证`, `Form 布局`, `Style Preset`; removed or replaced `Form`, `用法`, `基础表单`, `横向布局`, `多列表单` and 20
  more.
- `docs/zh-CN/docs/components/group-box.md` — Reworked group box documentation by adding `基础用法`, `变体`,
  `Theme 与 Style Preset`, `样式层级`, `长内容` and 3 more; removed or replaced `用法`, `基础 GroupBox`, `不同变体`,
  `带标题`, `自定义 ID` and 12 more.
- `docs/zh-CN/docs/components/hover-card.md` — Reworked hover card documentation by adding `使用`, `受控状态`,
  `自定义延迟与外观`, `交互与可访问性`, `API`; removed or replaced `用法`, `基础 HoverCard`, `用户资料预览`, `自定义时间控制`, `定位` and
  10 more.
- `docs/zh-CN/docs/components/icon.md` — Expanded icon documentation with `信息型图标`.
- `docs/zh-CN/docs/components/index.md` — Added Simplified Chinese index entries for AspectRatio, Breadcrumb,
  Card, Command, Empty, and NativeSelect and updated Badge scope.
- `docs/zh-CN/docs/components/input.md` — Expanded input documentation with `只读态`, `可访问性`,
  `Style Preset 与动效`, `只读与无效状态`.
- `docs/zh-CN/docs/components/kbd.md` — Reworked kbd documentation by adding `基础用法`, `组合按键`, `平台快捷键`,
  `图标与文字`, `Input Group` and 2 more; removed or replaced `用法`, `基础快捷键`, `常见快捷键`, `多修饰键`, `方向键与功能键` and 12
  more.
- `docs/zh-CN/docs/components/label.md` — Reworked label documentation by adding `基础用法`, `与 Input 组合`,
  `禁用状态`, `组合内容`, `项目扩展能力` and 1 more; removed or replaced `用法`, `基础标签`, `带次要文本`, `文本对齐`, `文本高亮` and 15
  more.
- `docs/zh-CN/docs/components/list.md` — Reworked list documentation by adding `基础用法`, `搜索与键盘交互`, `分组`,
  `增量加载`, `事件与无障碍`; removed or replaced `导入`, `用法`, `基础列表`, `分组列表`, `带图标和操作的列表项` and 9 more.
- `docs/zh-CN/docs/components/menu.md` — Updated menu documentation: added references to `on_open_change`.
- `docs/zh-CN/docs/components/notification.md` — Expanded notification documentation with `可访问性`.
- `docs/zh-CN/docs/components/number-input.md` — Expanded number input documentation with `只读与无效状态`, `无障碍`.
- `docs/zh-CN/docs/components/otp-input.md` — Reworked otp input documentation by adding `基础组合`,
  `Pattern 与粘贴`, `状态与尺寸`, `事件`, `辅助功能`; removed or replaced `用法`, `基础 OTP 输入`, `默认值`, `掩码输入`, `不同尺寸`
  and 21 more.
- `docs/zh-CN/docs/components/pagination.md` — Expanded pagination documentation with `可访问性与键盘行为`.
- `docs/zh-CN/docs/components/plot.md` — Updated plot documentation: added references to `text-xs`.
- `docs/zh-CN/docs/components/popover.md` — Reworked popover documentation by adding `基础用法`, `定位`,
  `动态内容与手动关闭`, `受控状态`, `自定义外观与触发方式`; removed or replaced `用法`, `基础 Popover`, `自定义定位`,
  `在 Popover 中渲染 View`, `使用 content 构造动态内容` and 5 more.
- `docs/zh-CN/docs/components/progress.md` — Updated progress documentation: added references to `Progress`,
  `Size::Size(height)`, `tokens.muted`, `tokens.progress_bar`, `normal` and 5 more and removed references
  to `color(c)`, `theme.progress_bar`.
- `docs/zh-CN/docs/components/radio.md` — Reworked radio documentation by adding `Radio Group`, `基本用法`,
  `排列方向`, `标签与描述`, `禁用与无效状态` and 5 more; removed or replaced `Radio`, `用法`, `基础单选按钮`, `受控单选按钮`,
  `RadioGroup（推荐）` and 13 more.
- `docs/zh-CN/docs/components/rating.md` — Expanded rating documentation with `只读状态`, `键盘与可访问性`,
  `Theme 与 Style Preset`.
- `docs/zh-CN/docs/components/scrollable.md` — Added Simplified Chinese guidance for stable scroll IDs,
  semantic scrollbar metrics, pointer targets, and overflow behavior.
- `docs/zh-CN/docs/components/select.md` — Reworked select documentation by adding `Content 定位`, `无效状态`,
  `可访问性`, `Style Preset`, `Color Theme`; removed or replaced `主题`.
- `docs/zh-CN/docs/components/settings.md` — Expanded settings documentation with `样式`.
- `docs/zh-CN/docs/components/sheet.md` — Reworked sheet documentation by adding `基础用法`, `方向与尺寸`,
  `Header 组合`, `Close 按钮与背景层`, `初始焦点` and 2 more; removed or replaced `用法`, `在根视图中渲染 Sheet 图层`, `基础 Sheet`,
  `不同方向`, `自定义尺寸` and 10 more.
- `docs/zh-CN/docs/components/sidebar.md` — Updated the Simplified Chinese sidebar guide for keyboard
  accessibility, `w`, and aligned Badge variants.
- `docs/zh-CN/docs/components/skeleton.md` — Updated skeleton documentation: added references to `muted`,
  `md`, `xl`, `secondary()` and removed references to `skeleton`, `secondary`, `secondary(true)`.
- `docs/zh-CN/docs/components/slider.md` — Expanded slider documentation with `键盘与无障碍`.
- `docs/zh-CN/docs/components/spinner.md` — Reworked spinner documentation by adding `尺寸`, `Variant 和动画`,
  `颜色和图标`, `组合`, `动效` and 2 more; removed or replaced `基础用法`, `自定义颜色`, `不同尺寸`, `自定义图标`, `可用图标` and 13
  more.
- `docs/zh-CN/docs/components/status-bar.md` — Expanded status bar documentation with `Style Preset`.
- `docs/zh-CN/docs/components/stepper.md` — Expanded stepper documentation with `可访问性与键盘行为`, `方法`.
- `docs/zh-CN/docs/components/switch.md` — Expanded switch documentation with `无效状态与可访问名称`.
- `docs/zh-CN/docs/components/table.md` — Reworked table documentation by adding `选中行`, `横向溢出`, `样式覆盖`;
  removed or replaced `去掉边框`.
- `docs/zh-CN/docs/components/tabs.md` — Expanded tabs documentation with `键盘操作`, `Style Preset`.
- `docs/zh-CN/docs/components/title-bar.md` — Updated title bar documentation: added references to
  `app_owns_titlebar_drag`, `TitleBar`.
- `docs/zh-CN/docs/components/toggle.md` — Reworked toggle documentation by adding `基础用法`, `变体与尺寸`, `状态`,
  `前置与后置图标`, `ToggleGroup` and 5 more; removed or replaced `导入`, `用法`, `基础 Toggle`, `图标 Toggle`,
  `受控 Toggle` and 9 more.
- `docs/zh-CN/docs/components/tooltip.md` — Reworked tooltip documentation by adding `组件内置支持`, `组合式 API`,
  `自定义内容`, `延迟和 Arrow`, `TooltipTrigger` and 2 more; removed or replaced `用法`, `纯文本 Tooltip`, `按钮 Tooltip`,
  `携带快捷键信息`, `自定义内容 Tooltip` and 8 more.
- `docs/zh-CN/docs/components/tree.md` — Reworked tree documentation by adding `基础用法`, `尺寸与 Style Preset`,
  `禁用节点`, `Context Menu`, `键盘行为` and 2 more; removed or replaced `用法`, `基础树`, `文件树与图标`, `动态加载`, `选择处理`
  and 7 more.
- `docs/zh-CN/docs/theme.md` — Expanded theme documentation with `Color Theme 与 Style Preset`, `从扁平外观字段迁移`.
- `skills/hearth-gpui/SKILL.md` — Expanded the Badge entry with variant and overlay APIs and removed the
  legacy Tag entry.
- `skills/hearth-gpui/references/style-guide.md` — Replaced legacy Alert status variants with the aligned
  Default and Destructive variant example.
- `skills/hearth-gpui/references/usage.md` — Migrated Form examples to stable field IDs and the FieldBody,
  FieldLabel, and FieldContent composition APIs.
- `themes/hybrid.json` — Changed JSON theme/schema keys: removed `shadow`.
- `themes/macos-classic.json` — Changed JSON theme/schema keys: removed `shadow`.
- `themes/matrix.json` — Changed JSON theme/schema keys: removed `radius`, `radius.lg`.
- `themes/mellifluous.json` — Changed JSON theme/schema keys: removed `shadow`.
- `themes/molokai.json` — Changed JSON theme/schema keys: removed `shadow`.

### Files Added by the Commit

These files were added by the commit rather than modified from an existing
upstream file. They are recorded here with their introduced purpose.

- `.agents/skills/align-shadcn-component/SKILL.md` — Added documentation for Align shadcn Component.
- `.agents/skills/align-shadcn-component/agents/openai.yaml` — Added Codex agent metadata for the shadcn
  component alignment skill.
- `AGENTS.md` — Added documentation for Repository Instructions.
- `crates/motion/Cargo.toml` — Added the manifest for the reusable `hearth-gpui-motion` crate.
- `crates/motion/src/lib.rs` — Added interpolation, transition, easing, reduced-motion, and motion-element
  primitives.
- `crates/story/examples/shadcn_capture.rs` — Added the main deterministic shadcn component screenshot capture
  application.
- `crates/story/examples/shadcn_locale_capture.rs` — Added locale-specific screenshot capture for translated
  component states.
- `crates/story/examples/shadcn_overlay_capture.rs` — Added deterministic capture scenarios for dialogs,
  popovers, sheets, and other overlays.
- `crates/story/examples/shadcn_phase0_capture.rs` — Added baseline capture scenarios for the initial shadcn
  alignment phase.
- `crates/story/src/stories/aspect_ratio_story.rs` — Added the AspectRatio component story and ratio examples.
- `crates/story/src/stories/card_story.rs` — Added the Card component story with semantic header, content, and
  footer composition.
- `crates/story/src/stories/command_story.rs` — Added the Command palette story with grouped, searchable, and
  keyboard-driven examples.
- `crates/story/src/stories/empty_story.rs` — Added the Empty component story for icon, title, description,
  and action composition.
- `crates/story/src/stories/input_group_story.rs` — Added InputGroup stories for addons, buttons, validation,
  and composed controls.
- `crates/story/src/stories/native_select_story.rs` — Added NativeSelect stories for sizing, validation, and
  disabled states.
- `crates/story/src/stories/shadcn_alignment_story.rs` — Added a consolidated story for inspecting
  Vega-aligned component states.
- `crates/ui/benches/shadcn_alignment.rs` — Added benchmarks for aligned component construction and rendering
  paths.
- `crates/ui/src/accessibility.rs` — Added shared accessibility-state builders, stable IDs, and
  active-descendant helpers.
- `crates/ui/src/aspect_ratio.rs` — Added the AspectRatio component with validated ratio-based layout.
- `crates/ui/src/card.rs` — Added Card and its semantic Header, Title, Description, Action, Content, and
  Footer slots.
- `crates/ui/src/chart/common.rs` — Added shared chart configuration, legend, series, and accessibility
  helpers.
- `crates/ui/src/chart/radial_chart.rs` — Added radial chart rendering, configuration, labels, and interaction
  support.
- `crates/ui/src/command.rs` — Added the Command component family with search, groups, items, empty state, and
  keyboard navigation.
- `crates/ui/src/dialog/modal.rs` — Added shared modal metrics and presentation policies for Dialog and
  AlertDialog.
- `crates/ui/src/empty.rs` — Added Empty with semantic header, media, title, description, and content slots.
- `crates/ui/src/input/input_group.rs` — Added InputGroup composition for inputs, text areas, addons, buttons,
  and validation state.
- `crates/ui/src/native_select.rs` — Added NativeSelect with options, sizing, validation, accessibility, and
  platform-native interaction.
- `crates/ui/src/theme/style.rs` — Added semantic Style Presets, Vega/Nova/Maia metrics, validation, registry,
  and motion tokens.
- `docs/docs/components/aspect-ratio.md` — Added documentation for AspectRatio.
- `docs/docs/components/breadcrumb.md` — Added documentation for Breadcrumb.
- `docs/docs/components/card.md` — Added documentation for Card.
- `docs/docs/components/command.md` — Added documentation for Command.
- `docs/docs/components/empty.md` — Added documentation for Empty.
- `docs/docs/components/input-group.md` — Added documentation for Input Group.
- `docs/docs/components/native-select.md` — Added documentation for Native Select.
- `docs/docs/components/separator.md` — Added documentation for Separator.
- `docs/shadcn/01-baseline-and-gaps.md` — Added documentation for Baseline and gap analysis.
- `docs/shadcn/02-component-matrix.md` — Added documentation for Component matrix.
- `docs/shadcn/03-roadmap.md` — Added documentation for Implementation roadmap.
- `docs/shadcn/04-verification.md` — Added documentation for Verification strategy.
- `docs/shadcn/05-style-presets.md` — Added documentation for Style Preset architecture.
- `docs/shadcn/06-implementation-status.md` — Added documentation for Implementation status.
- `docs/shadcn/07-release-evidence.md` — Added documentation for Release evidence.
- `docs/shadcn/08-motion-runtime.md` — Added documentation for Motion runtime architecture.
- `docs/shadcn/09-component-checklist.md` — Added documentation for Component alignment checklist.
- `docs/shadcn/README.md` — Added documentation for shadcn/ui alignment plan.
- `docs/shadcn/TODO.md` — Added documentation for Deferred TODO.
- `docs/shadcn/screenshots/default-dark-maia-page-01.png` — Added the `default dark maia page 01` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-maia-page-02.png` — Added the `default dark maia page 02` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-maia-page-03.png` — Added the `default dark maia page 03` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-maia-page-04.png` — Added the `default dark maia page 04` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-maia-page-05.png` — Added the `default dark maia page 05` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-nova-page-01.png` — Added the `default dark nova page 01` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-nova-page-02.png` — Added the `default dark nova page 02` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-nova-page-03.png` — Added the `default dark nova page 03` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-nova-page-04.png` — Added the `default dark nova page 04` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-vega-page-01.png` — Added the `default dark vega page 01` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-vega-page-02.png` — Added the `default dark vega page 02` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-vega-page-03.png` — Added the `default dark vega page 03` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-vega-page-04.png` — Added the `default dark vega page 04` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-dark-vega-page-05.png` — Added the `default dark vega page 05` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-maia-page-01.png` — Added the `default light maia page 01` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-maia-page-02.png` — Added the `default light maia page 02` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-maia-page-03.png` — Added the `default light maia page 03` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-maia-page-04.png` — Added the `default light maia page 04` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-maia-page-05.png` — Added the `default light maia page 05` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-nova-page-01.png` — Added the `default light nova page 01` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-nova-page-02.png` — Added the `default light nova page 02` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-nova-page-03.png` — Added the `default light nova page 03` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-nova-page-04.png` — Added the `default light nova page 04` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-vega-page-01.png` — Added the `default light vega page 01` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-vega-page-02.png` — Added the `default light vega page 02` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-vega-page-03.png` — Added the `default light vega page 03` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-vega-page-04.png` — Added the `default light vega page 04` visual
  verification capture asset.
- `docs/shadcn/screenshots/default-light-vega-page-05.png` — Added the `default light vega page 05` visual
  verification capture asset.
- `docs/shadcn/screenshots/locales/locale-en.png` — Added the `locale en` visual verification capture asset.
- `docs/shadcn/screenshots/locales/locale-zh-CN.png` — Added the `locale zh CN` visual verification capture
  asset.
- `docs/shadcn/screenshots/locales/locale-zh-TW.png` — Added the `locale zh TW` visual verification capture
  asset.
- `docs/shadcn/screenshots/overlays/popover-bottom-light-open.png` — Added the `popover bottom light open`
  visual verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-constrained-edge-light-open.png` — Added the
  `popover constrained edge light open` visual verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-left-light-open.png` — Added the `popover left light open` visual
  verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-right-dark-closing.png` — Added the `popover right dark closing`
  visual verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-right-dark-open.png` — Added the `popover right dark open` visual
  verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-right-light-closing.png` — Added the `popover right light closing`
  visual verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-right-light-open.png` — Added the `popover right light open`
  visual verification capture asset.
- `docs/shadcn/screenshots/overlays/popover-top-light-open.png` — Added the `popover top light open` visual
  verification capture asset.
- `docs/shadcn/screenshots/phase0/phase0-dark.png` — Added the `phase0 dark` visual verification capture
  asset.
- `docs/shadcn/screenshots/phase0/phase0-light.png` — Added the `phase0 light` visual verification capture
  asset.
- `docs/zh-CN/docs/components/aspect-ratio.md` — Added documentation for AspectRatio.
- `docs/zh-CN/docs/components/breadcrumb.md` — Added documentation for Breadcrumb.
- `docs/zh-CN/docs/components/card.md` — Added documentation for Card.
- `docs/zh-CN/docs/components/command.md` — Added documentation for Command.
- `docs/zh-CN/docs/components/empty.md` — Added documentation for Empty.
- `docs/zh-CN/docs/components/input-group.md` — Added documentation for Input Group.
- `docs/zh-CN/docs/components/native-select.md` — Added documentation for Native Select.
- `docs/zh-CN/docs/components/separator.md` — Added documentation for Separator.

### Files Removed by the Commit

- `crates/story/src/stories/chart_story/stacked_bar_chart.rs` — Removed the standalone stacked-bar story after
  consolidating stacked series into the main chart story.
- `crates/story/src/stories/tag_story.rs` — Removed the legacy Tag story from the shadcn-aligned gallery.
- `crates/ui/src/tag.rs` — Removed the legacy Tag component from the aligned component API.
- `docs/docs/components/tag.md` — Removed the English Tag documentation with the component API.
- `docs/zh-CN/docs/components/tag.md` — Removed the Simplified Chinese Tag documentation with the component
  API.


The original copyright and attribution notices remain applicable to the
upstream work. These modifications do not imply endorsement by or affiliation
with Longbridge. The repository continues to be distributed under the Apache
License, Version 2.0; see [`LICENSE-APACHE`](./LICENSE-APACHE).
