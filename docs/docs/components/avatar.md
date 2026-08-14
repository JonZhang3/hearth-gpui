---
title: Avatar
description: Displays an image with explicit fallback, badge, and group slots.
---

# Avatar

Avatar follows the shadcn Vega geometry. Images and fallback content are clipped to a circle, while badges remain visible outside the clipped media layer.

## Import

```rust
use hearth_gpui::{
    IconName,
    avatar::{Avatar, AvatarBadge, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage},
};
```

## Basic usage

Semantic avatars require a stable element ID and accessible label. The fallback is used while an image loads and when it fails.

```rust
Avatar::new("jane-avatar", "Jane Smith")
    .image(AvatarImage::new("https://example.com/jane.jpg"))
    .fallback(AvatarFallback::text("JS"))
```

Use an icon fallback when initials are not appropriate:

```rust
Avatar::new("organization-avatar", "Acme organization")
    .fallback(AvatarFallback::icon(IconName::Building2))
```

Use `Avatar::decorative()` only when adjacent content already conveys the same identity.

## Badge

`AvatarBadge` renders outside the clipped image surface and includes a background-colored ring.

```rust
Avatar::new("online-user", "Online user")
    .image(AvatarImage::new("https://example.com/user.jpg"))
    .fallback(AvatarFallback::text("OU"))
    .badge(AvatarBadge::new().bg(cx.theme().green))

Avatar::new("invited-user", "Invited user")
    .fallback(AvatarFallback::text("IU"))
    .badge(AvatarBadge::new().child(IconName::Plus))
```

## Sizes

The built-in sizes follow Vega: small 24 px, default 32 px, and large 40 px. Extra small and custom sizes remain available through `Sizable`.

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

## Avatar group

Groups preserve insertion order, apply the Vega overlap and background ring, and accept an explicit trailing count or icon item.

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

The count slot may contain an icon:

```rust
AvatarGroup::new()
    .avatars(avatars)
    .count(AvatarGroupCount::icon(IconName::Plus))
```

The group size is authoritative and is applied uniformly to every Avatar and count item.

[Avatar]: https://docs.rs/hearth-gpui/latest/hearth_gpui/avatar/struct.Avatar.html
[AvatarGroup]: https://docs.rs/hearth-gpui/latest/hearth_gpui/avatar/struct.AvatarGroup.html
