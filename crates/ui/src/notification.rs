// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `placement`.
// - Added or exposed behavior through `for_density`, `placement`, `resolved_placement`,
//   `metrics_follow_semantic_style_density`, `settings_preserve_notification_defaults`,
//   `notification_placement_override_falls_back_to_global_default`,
//   `duplicate_dismiss_emits_one_callback_and_reduced_motion_has_no_delay`.
// - Reworked Notification around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density.
use std::{
    any::TypeId,
    borrow::Cow,
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::Duration,
};

use gpui::{
    Anchor, Animation, AnimationExt, AnyElement, App, AppContext, ClickEvent, Context,
    DismissEvent, ElementId, Entity, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Render, Role, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Density, Disableable as _, Edges, Icon, IconName, Sizable as _, StyledExt,
    TITLE_BAR_HEIGHT,
    animation::{OverlayLifecycle, OverlayPhase, effective_motion_duration},
    button::Button,
    h_flex, v_flex,
};

/// Component-local geometry derived from semantic Style Preset density.
#[derive(Debug, Clone, Copy, PartialEq)]
struct NotificationMetrics {
    width: Pixels,
    padding_x: Pixels,
    padding_y: Pixels,
    content_gap: Pixels,
    stack_gap: Pixels,
    motion_offset: Pixels,
}

impl NotificationMetrics {
    /// Resolves Notification geometry without branching on a Style Preset ID.
    fn for_density(density: Density) -> Self {
        match density {
            Density::Compact => Self {
                width: px(384.),
                padding_x: px(12.),
                padding_y: px(10.),
                content_gap: px(8.),
                stack_gap: px(8.),
                motion_offset: px(16.),
            },
            Density::Standard => Self {
                width: px(448.),
                padding_x: px(16.),
                padding_y: px(14.),
                content_gap: px(12.),
                stack_gap: px(12.),
                motion_offset: px(16.),
            },
            Density::Comfortable => Self {
                width: px(448.),
                padding_x: px(20.),
                padding_y: px(16.),
                content_gap: px(12.),
                stack_gap: px(12.),
                motion_offset: px(24.),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum NotificationType {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationType {
    fn icon(&self, cx: &App) -> Icon {
        match self {
            Self::Info => Icon::new(IconName::Info).text_color(cx.theme().info),
            Self::Success => Icon::new(IconName::CircleCheck).text_color(cx.theme().success),
            Self::Warning => Icon::new(IconName::TriangleAlert).text_color(cx.theme().warning),
            Self::Error => Icon::new(IconName::CircleX).text_color(cx.theme().danger),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub(crate) enum NotificationId {
    Id(TypeId),
    IdAndElementId(TypeId, ElementId),
}

impl From<TypeId> for NotificationId {
    fn from(type_id: TypeId) -> Self {
        Self::Id(type_id)
    }
}

impl From<(TypeId, ElementId)> for NotificationId {
    fn from((type_id, id): (TypeId, ElementId)) -> Self {
        Self::IdAndElementId(type_id, id)
    }
}

/// A notification element.
pub struct Notification {
    /// The id is used make the notification unique.
    /// Then you push a notification with the same id, the previous notification will be replaced.
    ///
    /// None means the notification will be added to the end of the list.
    id: NotificationId,
    style: StyleRefinement,
    type_: Option<NotificationType>,
    title: Option<SharedString>,
    message: Option<SharedString>,
    icon: Option<Icon>,
    placement: Option<Anchor>,
    autohide: bool,
    action_builder: Option<Rc<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) -> Button>>,
    content_builder: Option<Rc<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) -> AnyElement>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    lifecycle: OverlayLifecycle,
}

impl From<String> for Notification {
    fn from(s: String) -> Self {
        Self::new().message(s)
    }
}

impl From<SharedString> for Notification {
    fn from(s: SharedString) -> Self {
        Self::new().message(s)
    }
}

impl From<&str> for Notification {
    fn from(s: &str) -> Self {
        Self::new().message(s)
    }
}

impl<'a> From<Cow<'a, str>> for Notification {
    fn from(s: Cow<'a, str>) -> Self {
        Self::new().message(s)
    }
}

impl<T> From<(NotificationType, T)> for Notification
where
    T: Into<SharedString>,
{
    fn from((type_, content): (NotificationType, T)) -> Self {
        Self::new().message(content).with_type(type_)
    }
}

struct DefaultIdType;

impl Notification {
    /// Create a new notification.
    ///
    /// The default id is a random UUID.
    pub fn new() -> Self {
        let id: SharedString = uuid::Uuid::new_v4().to_string().into();
        let id = (TypeId::of::<DefaultIdType>(), id.into());

        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            title: None,
            message: None,
            type_: None,
            icon: None,
            placement: None,
            autohide: true,
            action_builder: None,
            content_builder: None,
            on_click: None,
            on_close: None,
            lifecycle: OverlayLifecycle::opened(),
        }
    }

    /// Set the message of the notification, default is None.
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Create an info notification with the given message.
    pub fn info(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Info)
    }

    /// Create a success notification with the given message.
    pub fn success(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Success)
    }

    /// Create a warning notification with the given message.
    pub fn warning(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Warning)
    }

    /// Create an error notification with the given message.
    pub fn error(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Error)
    }

    /// Set the type for unique identification of the notification.
    ///
    /// ```rs
    /// struct MyNotificationKind;
    /// let notification = Notification::new().message("Hello").id::<MyNotificationKind>();
    /// ```
    pub fn id<T: Sized + 'static>(mut self) -> Self {
        self.id = TypeId::of::<T>().into();
        self
    }

    /// Set the type and id of the notification, used to uniquely identify the notification.
    pub fn id1<T: Sized + 'static>(mut self, key: impl Into<ElementId>) -> Self {
        self.id = (TypeId::of::<T>(), key.into()).into();
        self
    }

    /// Set the title of the notification, default is None.
    ///
    /// If title is None, the notification will not have a title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the placement for this notification.
    ///
    /// Notifications without an override inherit [`NotificationSettings::placement`].
    pub fn placement(mut self, placement: Anchor) -> Self {
        self.placement = Some(placement);
        self
    }

    /// Resolve the notification placement against the active global default.
    fn resolved_placement(&self, default: Anchor) -> Anchor {
        self.placement.unwrap_or(default)
    }

    /// Set the icon of the notification.
    ///
    /// If icon is None, the notification will use the default icon of the type.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the type of the notification, default is NotificationType::Info.
    pub fn with_type(mut self, type_: NotificationType) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Set the auto hide of the notification, default is true.
    pub fn autohide(mut self, autohide: bool) -> Self {
        self.autohide = autohide;
        self
    }

    /// Set the click callback of the notification.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set the close callback of the notification.
    ///
    /// Triggered when the notification is closed by any means
    /// (close button, middle-click, autohide, click handler, or programmatic close).
    pub fn on_close(mut self, on_close: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }

    /// Set the action button of the notification.
    ///
    /// When an action is set, the notification will not autohide.
    pub fn action<F>(mut self, action: F) -> Self
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) -> Button + 'static,
    {
        self.action_builder = Some(Rc::new(action));
        self.autohide = false;
        self
    }

    /// Dismiss the notification.
    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(transition) = self.lifecycle.begin_close() else {
            return;
        };
        cx.notify();

        let on_close = self.on_close.clone();
        let duration = effective_motion_duration(cx.theme().style.motion.emphasis(), cx);
        cx.spawn_in(window, async move |view, cx| {
            cx.background_executor().timer(duration).await;
            let completed = view
                .update_in(cx, |view, _, cx| {
                    let completed = view.lifecycle.complete_close(transition);
                    if completed {
                        cx.emit(DismissEvent);
                        cx.notify();
                    }
                    completed
                })
                .unwrap_or(false);
            if completed && let Some(on_close) = on_close {
                _ = cx.update(|window, cx| on_close(window, cx));
            }
        })
        .detach();
    }

    /// Set the content of the notification.
    pub fn content(
        mut self,
        content: impl Fn(&mut Self, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        self.content_builder = Some(Rc::new(content));
        self
    }
}

impl EventEmitter<DismissEvent> for Notification {}
impl FluentBuilder for Notification {}
impl Styled for Notification {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Notification {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self
            .content_builder
            .clone()
            .map(|builder| builder(self, window, cx));
        let action = self
            .action_builder
            .clone()
            .map(|builder| builder(self, window, cx).small());

        let closing = self.lifecycle.phase() == OverlayPhase::Closing;
        let accepts_input = self.lifecycle.accepts_input();
        let style = cx.theme().style.as_ref();
        let motion = style.motion;
        let metrics = NotificationMetrics::for_density(style.density);
        let icon = self
            .icon
            .clone()
            .or_else(|| self.type_.map(|type_| type_.icon(cx)));
        let placement = self.resolved_placement(cx.theme().notification.placement);
        let margins = &cx.theme().notification.margins;
        let available_width =
            (window.viewport_size().width - margins.left - margins.right).max(px(0.));
        let accessibility_label = self
            .title
            .clone()
            .or_else(|| self.message.clone())
            .unwrap_or_else(|| t!("Common.Notification").into());

        h_flex()
            .id("notification")
            .role(Role::Alert)
            .aria_label(accessibility_label)
            .group("")
            .occlude()
            .relative()
            .items_start()
            .w(metrics.width)
            .max_w(available_width)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tokens.popover)
            .text_color(cx.theme().tokens.popover_foreground)
            .rounded(style.radii.lg)
            .when(style.elevation.enabled, |this| match style.density {
                Density::Compact => this.shadow_sm(),
                Density::Standard => this.shadow_md(),
                Density::Comfortable => this.shadow_lg(),
            })
            .py(metrics.padding_y)
            .pl(metrics.padding_x)
            .pr(metrics.padding_x + px(20.))
            .gap(metrics.content_gap)
            .refine_style(&self.style)
            .when_some(icon, |this, icon| {
                this.child(div().flex_none().pt(px(2.)).child(icon))
            })
            .child(
                v_flex()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .gap(px(2.))
                    .when_some(self.title.clone(), |this, title| {
                        this.child(div().text_sm().font_semibold().child(title))
                    })
                    .when_some(self.message.clone(), |this, message| {
                        this.child(
                            div()
                                .text_sm()
                                .when(self.title.is_some(), |this| {
                                    this.text_color(cx.theme().muted_foreground)
                                })
                                .child(message),
                        )
                    })
                    .when_some(content, |this, content| this.child(content)),
            )
            .when_some(action, |this, action| {
                this.child(div().flex_none().child(action))
            })
            .child(
                div()
                    .absolute()
                    .top_1()
                    .right_1()
                    .invisible()
                    .group_hover("", |this| this.visible())
                    .child(
                        Button::new("close")
                            .aria_label(t!("Common.Close"))
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .disabled(!accepts_input)
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.dismiss(window, cx);
                            })),
                    ),
            )
            .when_some(
                accepts_input.then(|| self.on_click.clone()).flatten(),
                |this, on_click| {
                    this.on_click(cx.listener(move |view, event, window, cx| {
                        view.dismiss(window, cx);
                        on_click(event, window, cx);
                    }))
                },
            )
            .when(accepts_input, |this| {
                this.on_aux_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                    if event.is_middle_click() {
                        view.dismiss(window, cx);
                    }
                }))
            })
            .when(!accepts_input, |this| {
                this.child(div().absolute().top_0().left_0().size_full().occlude())
            })
            .with_animation(
                ElementId::NamedInteger("notification-motion".into(), closing as u64),
                Animation::new(motion.emphasis()).with_easing(move |delta| {
                    if closing {
                        motion.exit_easing.sample(delta)
                    } else {
                        motion.enter_easing.sample(delta)
                    }
                }),
                move |this, delta| {
                    let progress = if closing { delta } else { 1. - delta };
                    let offset = metrics.motion_offset * progress;

                    match placement {
                        Anchor::TopRight | Anchor::BottomRight => this.left(offset),
                        Anchor::TopLeft | Anchor::BottomLeft => this.left(-offset),
                        Anchor::TopCenter => this.top(-offset),
                        Anchor::BottomCenter => this.top(offset),
                        _ => this,
                    }
                },
            )
    }
}

/// The settings for notifications.
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    /// The placement of the notification, default: [`Anchor::TopRight`]
    pub placement: Anchor,
    /// The margins of the notification with respect to the window edges.
    pub margins: Edges<Pixels>,
    /// The maximum number of notifications to show at once, default: 10
    pub max_items: usize,
    /// The automatic dismissal duration, default: 5 seconds.
    pub duration: Duration,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        let offset = px(16.);
        Self {
            placement: Anchor::TopRight,
            margins: Edges {
                top: TITLE_BAR_HEIGHT + offset, // avoid overlap with title bar
                right: offset,
                bottom: offset,
                left: offset,
            },
            max_items: 10,
            duration: Duration::from_secs(5),
        }
    }
}

/// A list of notifications.
pub struct NotificationList {
    /// Notifications that will be auto hidden.
    pub(crate) notifications: VecDeque<Entity<Notification>>,
    _subscriptions: HashMap<NotificationId, Subscription>,
}

impl NotificationList {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            notifications: VecDeque::new(),
            _subscriptions: HashMap::new(),
        }
    }

    pub fn push(
        &mut self,
        notification: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let notification = notification.into();
        let id = notification.id.clone();
        let autohide = notification.autohide;
        let duration = cx.theme().notification.duration;

        // Remove the notification by id, for keep unique.
        self.notifications.retain(|note| note.read(cx).id != id);

        let notification = cx.new(|_| notification);

        self._subscriptions.insert(
            id.clone(),
            cx.subscribe(&notification, move |view, _, _: &DismissEvent, cx| {
                view.notifications.retain(|note| id != note.read(cx).id);
                view._subscriptions.remove(&id);
            }),
        );

        self.notifications.push_back(notification.clone());
        if autohide {
            // Keep auto-dismiss timing under the active Theme configuration.
            cx.spawn_in(window, async move |_, cx| {
                cx.background_executor().timer(duration).await;

                if let Err(err) =
                    notification.update_in(cx, |note, window, cx| note.dismiss(window, cx))
                {
                    tracing::error!("failed to auto hide notification: {:?}", err);
                }
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn close(
        &mut self,
        id: impl Into<NotificationId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id: NotificationId = id.into();
        if let Some(n) = self.notifications.iter().find(|n| n.read(cx).id == id) {
            n.update(cx, |note, cx| note.dismiss(window, cx))
        }
        cx.notify();
    }

    /// Close all notifications whose id matches the given [`TypeId`], regardless of
    /// whether they were registered via [`Notification::id`] or [`Notification::id1`].
    pub(crate) fn close_by_type(
        &mut self,
        type_id: TypeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let matched: Vec<_> = self
            .notifications
            .iter()
            .filter(|n| match &n.read(cx).id {
                NotificationId::Id(t) | NotificationId::IdAndElementId(t, _) => *t == type_id,
            })
            .cloned()
            .collect();
        for n in matched {
            n.update(cx, |note, cx| note.dismiss(window, cx));
        }
        cx.notify();
    }

    pub fn clear(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.notifications.clear();
        cx.notify();
    }

    pub fn notifications(&self) -> Vec<Entity<Notification>> {
        self.notifications.iter().cloned().collect()
    }
}

impl Render for NotificationList {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        const PLACEMENTS: [Anchor; 6] = [
            Anchor::TopLeft,
            Anchor::TopCenter,
            Anchor::TopRight,
            Anchor::BottomLeft,
            Anchor::BottomCenter,
            Anchor::BottomRight,
        ];

        let size = window.viewport_size();
        let settings = &cx.theme().notification;
        let max_items = settings.max_items;
        let default_placement = settings.placement;
        let margins = &cx.theme().notification.margins;
        let metrics = NotificationMetrics::for_density(cx.theme().style.density);
        let groups = PLACEMENTS
            .into_iter()
            .enumerate()
            .filter_map(|(index, placement)| {
                let mut items = self
                    .notifications
                    .iter()
                    .filter(|notification| {
                        notification.read(cx).resolved_placement(default_placement) == placement
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let visible_start = items.len().saturating_sub(max_items);
                let items = items.drain(visible_start..).collect::<Vec<_>>();

                if items.is_empty() {
                    return None;
                }

                Some(
                    v_flex()
                        .id(("notification-list", index))
                        .absolute()
                        .max_h(size.height)
                        .pt(margins.top)
                        .pb(margins.bottom)
                        .gap(metrics.stack_gap)
                        .when(
                            matches!(
                                placement,
                                Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight
                            ),
                            |this| this.top_0(),
                        )
                        .when(
                            matches!(
                                placement,
                                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight
                            ),
                            |this| this.bottom_0().flex_col_reverse(),
                        )
                        .when(
                            matches!(placement, Anchor::TopLeft | Anchor::BottomLeft),
                            |this| this.left_0().pl(margins.left),
                        )
                        .when(
                            matches!(placement, Anchor::TopRight | Anchor::BottomRight),
                            |this| this.right_0().pr(margins.right),
                        )
                        .when(
                            matches!(placement, Anchor::TopCenter | Anchor::BottomCenter),
                            |this| this.left_0().right_0().items_center(),
                        )
                        .children(items),
                )
            });

        div()
            .id("notification-list")
            .relative()
            .size_full()
            .children(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use gpui::{TestAppContext, VisualTestContext};
    use std::{cell::Cell, time::Duration};

    struct FooKind;
    struct BarKind;

    struct TestRoot {
        list: Entity<NotificationList>,
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.list.clone()
        }
    }

    fn ids(list: &Entity<NotificationList>, cx: &mut VisualTestContext) -> Vec<NotificationId> {
        list.read_with(cx, |l, cx| {
            l.notifications
                .iter()
                .map(|n| n.read(cx).id.clone())
                .collect()
        })
    }

    /// Drive the dismiss animation timer + propagate the resulting `DismissEvent`
    /// so that closed notifications are removed from the list.
    fn flush_dismiss(cx: &mut VisualTestContext) {
        cx.background_executor
            .advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
    }

    #[test]
    fn metrics_follow_semantic_style_density() {
        let compact = NotificationMetrics::for_density(Density::Compact);
        let standard = NotificationMetrics::for_density(Density::Standard);
        let comfortable = NotificationMetrics::for_density(Density::Comfortable);

        assert_eq!(compact.width, px(384.));
        assert!(compact.padding_x < standard.padding_x);
        assert!(standard.padding_x < comfortable.padding_x);
        assert!(compact.stack_gap < standard.stack_gap);
        assert!(standard.motion_offset < comfortable.motion_offset);
    }

    #[test]
    fn settings_preserve_notification_defaults() {
        let settings = NotificationSettings::default();

        assert_eq!(settings.placement, Anchor::TopRight);
        assert_eq!(settings.max_items, 10);
        assert_eq!(settings.duration, Duration::from_secs(5));
    }

    #[test]
    fn notification_placement_override_falls_back_to_global_default() {
        let default = Anchor::TopRight;

        assert_eq!(Notification::new().resolved_placement(default), default);
        assert_eq!(
            Notification::new()
                .placement(Anchor::BottomLeft)
                .resolved_placement(default),
            Anchor::BottomLeft
        );
    }

    #[gpui::test]
    fn close_by_type_removes_id_and_all_id1_of_same_type(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("plain").id::<FooKind>().autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("a").id1::<FooKind>(1).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("b").id1::<FooKind>(2).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("bar").id::<BarKind>().autohide(false),
                window,
                cx,
            );
        });
        cx.run_until_parked();
        assert_eq!(ids(&list, cx).len(), 4);

        list.update_in(cx, |list, window, cx| {
            list.close_by_type(TypeId::of::<FooKind>(), window, cx);
        });
        flush_dismiss(cx);

        let remaining = ids(&list, cx);
        assert_eq!(
            remaining,
            vec![NotificationId::Id(TypeId::of::<BarKind>())],
            "only the BarKind notification should survive"
        );
    }

    #[gpui::test]
    fn close_with_id_and_element_id_removes_only_matching_key(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("a").id1::<FooKind>(1).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("b").id1::<FooKind>(2).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("plain").id::<FooKind>().autohide(false),
                window,
                cx,
            );
        });

        list.update_in(cx, |list, window, cx| {
            list.close(
                (TypeId::of::<FooKind>(), ElementId::from(1usize)),
                window,
                cx,
            );
        });
        flush_dismiss(cx);

        let remaining = ids(&list, cx);
        assert_eq!(remaining.len(), 2);
        assert!(remaining.contains(&NotificationId::IdAndElementId(
            TypeId::of::<FooKind>(),
            ElementId::from(2usize),
        )));
        assert!(remaining.contains(&NotificationId::Id(TypeId::of::<FooKind>())));
    }

    #[gpui::test]
    fn close_with_only_type_id_does_not_match_id1_entries(cx: &mut TestAppContext) {
        // The plain `close(TypeId)` form (used by the legacy code path) must keep
        // its narrow semantics: it only matches `NotificationId::Id`, not
        // `NotificationId::IdAndElementId`. The new `close_by_type` is the broad form.
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("a").id1::<FooKind>(1).autohide(false),
                window,
                cx,
            );
        });

        list.update_in(cx, |list, window, cx| {
            list.close(TypeId::of::<FooKind>(), window, cx);
        });
        flush_dismiss(cx);

        assert_eq!(ids(&list, cx).len(), 1, "id1 entry should remain untouched");
    }

    #[gpui::test]
    fn close_by_type_with_no_match_is_noop(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("bar").id::<BarKind>().autohide(false),
                window,
                cx,
            );
        });

        list.update_in(cx, |list, window, cx| {
            list.close_by_type(TypeId::of::<FooKind>(), window, cx);
        });
        flush_dismiss(cx);

        assert_eq!(ids(&list, cx).len(), 1);
    }

    #[gpui::test]
    fn duplicate_dismiss_emits_one_callback_and_reduced_motion_has_no_delay(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
            cx.set_reduce_motion(true);
        });
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
        });
        let list = root.read_with(cx, |root, _| root.list.clone());
        let close_count = Rc::new(Cell::new(0));

        list.update_in(cx, |list, window, cx| {
            let close_count = close_count.clone();
            list.push(
                Notification::info("reduced motion")
                    .id::<FooKind>()
                    .autohide(false)
                    .on_close(move |_, _| close_count.set(close_count.get() + 1)),
                window,
                cx,
            );
            list.close(TypeId::of::<FooKind>(), window, cx);
            list.close(TypeId::of::<FooKind>(), window, cx);
        });
        cx.run_until_parked();

        assert!(ids(&list, cx).is_empty());
        assert_eq!(close_count.get(), 1);
    }
}
