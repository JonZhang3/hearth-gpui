use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, Window, prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, ElementExt as _, StyledExt,
    animation::{Transition, effective_motion_duration},
    v_flex,
};

enum CollapsibleChild {
    Element(AnyElement),
    Content(AnyElement),
}

impl CollapsibleChild {
    fn is_content(&self) -> bool {
        matches!(self, CollapsibleChild::Content(_))
    }
}

/// An interactive element which expands/collapses.
#[derive(IntoElement)]
pub struct Collapsible {
    id: Option<ElementId>,
    style: StyleRefinement,
    children: Vec<CollapsibleChild>,
    open: bool,
}

#[derive(Clone, Copy, Debug)]
struct CollapsibleMotionState {
    desired_open: bool,
    render_content: bool,
    measured_height: Pixels,
    from_height: Pixels,
    target_height: Pixels,
    epoch: u64,
}

impl CollapsibleMotionState {
    /// Creates persistent motion state for a keyed Collapsible content region.
    fn new(open: bool) -> Self {
        Self {
            desired_open: open,
            render_content: open,
            measured_height: px(0.),
            from_height: px(0.),
            target_height: px(0.),
            epoch: 0,
        }
    }

    /// Applies an external open-state change and returns the close epoch to finish later.
    fn update_open(&mut self, open: bool) -> Option<u64> {
        if self.desired_open == open {
            return None;
        }

        self.desired_open = open;
        self.epoch = self.epoch.wrapping_add(1);
        if open {
            self.render_content = true;
            self.from_height = px(0.);
            self.target_height = self.measured_height;
            None
        } else {
            self.from_height = self.measured_height;
            self.target_height = px(0.);
            Some(self.epoch)
        }
    }

    /// Records natural content height and restarts an open animation when layout changes.
    fn measure(&mut self, height: Pixels) -> bool {
        if height <= px(0.) || height == self.measured_height {
            return false;
        }

        let previous_height = self.measured_height;
        self.measured_height = height;
        if self.desired_open {
            self.from_height = if previous_height > px(0.) {
                previous_height
            } else {
                px(0.)
            };
            self.target_height = height;
            self.epoch = self.epoch.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Unmounts closed content only when the matching close animation is still current.
    fn finish_close(&mut self, epoch: u64) -> bool {
        if self.epoch != epoch || self.desired_open || !self.render_content {
            return false;
        }

        self.render_content = false;
        true
    }
}

/// Renders one keyed content region while preserving surrounding always-visible children.
fn render_animated_content(
    id: &ElementId,
    index: usize,
    content: AnyElement,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let state_key = format!("{id}-content-{index}-motion");
    let state = window.use_keyed_state(state_key, cx, |_, _| CollapsibleMotionState::new(open));
    let close_epoch = state.update(cx, |state, _| state.update_open(open));
    let duration = cx.theme().style.motion.slow();
    let effective_duration = effective_motion_duration(duration, cx);
    if let Some(close_epoch) = close_epoch {
        cx.spawn({
            let state = state.clone();
            async move |cx| {
                cx.background_executor().timer(effective_duration).await;
                _ = state.update(cx, |state, cx| {
                    if state.finish_close(close_epoch) {
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    let snapshot = *state.read(cx);
    let move_easing = cx.theme().style.motion.move_easing;
    let motion_id = format!("{id}-content-{index}-height-{}", snapshot.epoch);
    let state_for_measure = state.clone();
    let content = v_flex()
        .w_full()
        .when(snapshot.render_content, |this| this.child(content))
        .on_prepaint(move |bounds, _, cx| {
            _ = state_for_measure.update(cx, |state, cx| {
                if state.measure(bounds.size.height) {
                    cx.notify();
                }
            });
        });

    let wrapper = v_flex().w_full().overflow_hidden().child(content);

    Transition::new(duration)
        .ease_token(move_easing)
        .height(snapshot.from_height, snapshot.target_height)
        .apply(wrapper, motion_id)
        .into_any_element()
}

impl Collapsible {
    /// Creates a new `Collapsible` instance.
    pub fn new() -> Self {
        Self {
            id: None,
            style: StyleRefinement::default(),
            open: false,
            children: vec![],
        }
    }

    /// Sets the stable identity required for measured enter and exit motion.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets whether the collapsible is open. default is false.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets the content of the collapsible.
    ///
    /// If `open` is false, content will be hidden.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.children
            .push(CollapsibleChild::Content(content.into_any_element()));
        self
    }
}

impl Styled for Collapsible {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(|el| CollapsibleChild::Element(el)));
    }
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(id) = self.id else {
            return v_flex()
                .refine_style(&self.style)
                .children(self.children.into_iter().filter_map(|child| {
                    if child.is_content() && !self.open {
                        None
                    } else {
                        match child {
                            CollapsibleChild::Element(el) | CollapsibleChild::Content(el) => {
                                Some(el)
                            }
                        }
                    }
                }))
                .into_any_element();
        };

        let children = self
            .children
            .into_iter()
            .enumerate()
            .map(|(index, child)| match child {
                CollapsibleChild::Element(element) => element,
                CollapsibleChild::Content(content) => {
                    render_animated_content(&id, index, content, self.open, window, cx)
                }
            })
            .collect::<Vec<_>>();

        v_flex()
            .id(id)
            .refine_style(&self.style)
            .children(children)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_completion_ignores_an_interrupted_reopen() {
        let mut state = CollapsibleMotionState::new(true);
        assert!(state.measure(px(120.)));
        let close_epoch = state.update_open(false).unwrap();

        assert_eq!(state.update_open(true), None);
        assert!(!state.finish_close(close_epoch));
        assert!(state.render_content);
    }

    #[test]
    fn dynamic_open_content_retargets_from_the_previous_height() {
        let mut state = CollapsibleMotionState::new(true);
        assert!(state.measure(px(80.)));
        assert!(state.measure(px(132.)));

        assert_eq!(state.from_height, px(80.));
        assert_eq!(state.target_height, px(132.));
    }

    #[test]
    fn matching_close_completion_unmounts_hidden_content() {
        let mut state = CollapsibleMotionState::new(true);
        assert!(state.measure(px(96.)));
        let close_epoch = state.update_open(false).unwrap();

        assert!(state.finish_close(close_epoch));
        assert!(!state.render_content);
    }
}
