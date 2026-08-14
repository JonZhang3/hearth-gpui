---
title: Avatar
description: 支持显式图片、回退、徽标和分组插槽的头像组件。
---

# Avatar

Avatar 使用 shadcn Vega 的几何规格。图片和回退内容裁剪为圆形，Badge 位于裁剪层之外，不会被截断。

## 导入

```rust
use gpui_component::{
    IconName,
    avatar::{Avatar, AvatarBadge, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage},
};
```

## 基础用法

语义 Avatar 必须提供稳定的元素 ID 和可访问名称。图片加载中或加载失败时都会显示 fallback。

```rust
Avatar::new("jane-avatar", "Jane Smith")
    .image(AvatarImage::new("https://example.com/jane.jpg"))
    .fallback(AvatarFallback::text("JS"))
```

不适合显示首字母时，可以使用图标回退：

```rust
Avatar::new("organization-avatar", "Acme organization")
    .fallback(AvatarFallback::icon(IconName::Building2))
```

只有在相邻内容已经表达同一身份时，才使用 `Avatar::decorative()`。

## Badge

`AvatarBadge` 显示在图片裁剪层之外，并带有背景色描边。

```rust
Avatar::new("online-user", "Online user")
    .image(AvatarImage::new("https://example.com/user.jpg"))
    .fallback(AvatarFallback::text("OU"))
    .badge(AvatarBadge::new().bg(cx.theme().green))

Avatar::new("invited-user", "Invited user")
    .fallback(AvatarFallback::text("IU"))
    .badge(AvatarBadge::new().child(IconName::Plus))
```

## 尺寸

内置尺寸与 Vega 一致：small 为 24 px、默认尺寸为 32 px、large 为 40 px。通过 `Sizable` 仍可使用 extra small 和自定义尺寸。

```rust
Avatar::new("small-avatar", "Small avatar")
    .fallback(AvatarFallback::text("S"))
    .small()

Avatar::new("default-avatar", "Default avatar")
    .fallback(AvatarFallback::text("M"))

Avatar::new("large-avatar", "Large avatar")
    .fallback(AvatarFallback::text("L"))
    .large()

Avatar::new("custom-avatar", "Custom avatar")
    .fallback(AvatarFallback::text("C"))
    .with_size(px(56.))
```

## Avatar Group

AvatarGroup 保持插入顺序，使用 Vega 的重叠距离和背景色描边，并支持显式的尾部数量或图标项。

```rust
AvatarGroup::new()
    .avatar(
        Avatar::new("alice", "Alice")
            .image(AvatarImage::new("https://example.com/alice.jpg"))
            .fallback(AvatarFallback::text("A")),
    )
    .avatar(
        Avatar::new("bob", "Bob")
            .image(AvatarImage::new("https://example.com/bob.jpg"))
            .fallback(AvatarFallback::text("B")),
    )
    .avatar(
        Avatar::new("charlie", "Charlie")
            .fallback(AvatarFallback::text("C")),
    )
    .count(AvatarGroupCount::text("+3"))
```

尾部项也可以显示图标：

```rust
AvatarGroup::new()
    .avatars(avatars)
    .count(AvatarGroupCount::icon(IconName::Plus))
```

AvatarGroup 的尺寸优先级最高，会统一应用到全部 Avatar 和尾部项。

[Avatar]: https://docs.rs/gpui-component/latest/gpui_component/avatar/struct.Avatar.html
[AvatarGroup]: https://docs.rs/gpui-component/latest/gpui_component/avatar/struct.AvatarGroup.html
