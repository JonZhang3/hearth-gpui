---
title: Form 与 Field
description: 提供组合式表单字段原语、GPUI 原生网格布局和验证语义。
---

# Form 与 Field

`Field` 采用 shadcn 的组合式模型。`Form` 是用于排列 Field 的 GPUI 原生布局层；校验逻辑和提交值仍由应用状态管理。

## 导入

```rust
use hearth_gpui::form::{
    field, v_form, FieldBody, FieldContent, FieldDescription, FieldError,
    FieldGroup, FieldLabel, FieldLegend, FieldSet,
};
```

## 基础字段

每个 Field 都需要稳定 ID。控件本身应设置语义文本，同时为 Field 设置明确的无障碍名称，因为 GPUI 当前尚未提供完整的 `labelled-by` 关联能力。

```rust
field("account-email")
    .aria_label("Email")
    .required(true)
    .content(move |state| {
        FieldBody::new()
            .child(FieldLabel::new("Email").required(state.required()))
            .child(
                FieldContent::new()
                    .child(Input::new(&email).aria_label("Email").disabled(state.disabled()))
                    .child(FieldDescription::new("用于接收账户通知。")),
            )
    })
```

`FieldLabel::for_focus(&focus_handle)` 提供点击标签聚焦控件的原生行为。目标控件仍需设置自身的无障碍名称。

## 表单验证

```rust
field("account-email")
    .aria_label("Email")
    .aria_description("请输入有效的邮箱地址。")
    .required(true)
    .invalid(true)
    .content(move |state| {
        FieldBody::new()
            .child(FieldLabel::new("Email").required(state.required()))
            .child(
                FieldContent::new()
                    .child(Input::new(&email).aria_label("Email").invalid(state.invalid()))
                    .child(FieldError::new(
                        "account-email-error",
                        "请输入有效的邮箱地址。",
                    )),
            )
    })
```

`FieldError` 暴露 `Role::Alert`。`errors(...)` 会保留错误顺序并移除重复内容。`FieldState` 会把 Form 与 Field 的有效状态传入内容构建器，由控件消费其支持的状态。

## 字段分组

```rust
FieldSet::new("notification-preferences")
    .aria_label("通知偏好")
    .content(|state| {
        FieldBody::new()
            .child(FieldLegend::new("通知偏好"))
            .child(
                FieldGroup::new()
                    .selection()
                    .child(Checkbox::new("email-updates").label("邮件通知").disabled(state.disabled()))
                    .child(Checkbox::new("product-updates").label("产品更新").disabled(state.disabled())),
            )
    })
```

组合式组件还包括 `FieldTitle` 和 `FieldSeparator`。`FieldGroup::selection()` 使用 Checkbox 和 Radio 集合所需的紧凑间距。使用 `FieldLegendVariant::Label` 可切换为较小的 Legend 字体。

## Form 布局

```rust
v_form()
    .columns(2)
    .child(
        field("first-name")
            .aria_label("名字")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("名字"))
                .child(FieldContent::new().child(Input::new(&first_name)))),
    )
    .child(
        field("last-name")
            .aria_label("姓氏")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("姓氏"))
                .child(FieldContent::new().child(Input::new(&last_name)))),
    )
    .child(
        field("biography")
            .col_span(2)
            .aria_label("个人简介")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("个人简介"))
                .child(FieldContent::new().child(Input::new(&biography)))),
    )
```

使用 `h_form()` 创建横向 Field。多个横向 Field 需要对齐时，应为 `FieldLabel` 设置明确宽度：

```rust
h_form().child(
    field("username")
        .aria_label("用户名")
        .content(move |_| FieldBody::new()
            .child(FieldLabel::new("用户名").w(px(120.)).flex_shrink_0())
            .child(FieldContent::new().child(Input::new(&username)))),
)
```

`Form` 支持 `Sizable`、`Styled`、`Disableable`、多列布局和 Field 网格定位。`Form::disabled(true)` 会进入每个子 Field 的有效状态。列数为零时会规范为一列。`Field::visible(false)` 会同时从布局和无障碍树中移除 Field。

## Style Preset

组件间距由语义 Style Preset density 解析。Vega 是默认基线，Nova 使用紧凑密度，Maia 使用舒适密度。固定版本的 shadcn Field 没有声明 transition，因此这些组件不增加状态动画。
