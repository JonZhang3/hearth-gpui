// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `empty`, `for_focus`.
// - Added or exposed behavior through `empty`, `for_focus`, `case_insensitive_ranges`, `extend`,
//   `disabled`, `unicode_case_folding_keeps_original_byte_boundaries`,
//   `masked_labels_do_not_apply_ranges_from_unmasked_text`, `supports_composed_and_disabled_labels`
//   and 1 more.
// - Reworked Label around focus-visible and focus restoration behavior.
use std::ops::Range;

use gpui::{
    AnyElement, App, FocusHandle, HighlightStyle, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, StyledText,
    Window, div, prelude::FluentBuilder, relative,
};

use crate::{ActiveTheme, Disableable, StyledExt};

const MASKED: &str = "•";

/// Represents the type of match for highlighting text in a label.
#[derive(Clone)]
pub enum HighlightsMatch {
    Prefix(SharedString),
    Full(SharedString),
}

impl HighlightsMatch {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Prefix(s) => s.as_str(),
            Self::Full(s) => s.as_str(),
        }
    }

    #[inline]
    pub fn is_prefix(&self) -> bool {
        matches!(self, Self::Prefix(_))
    }
}

impl From<&str> for HighlightsMatch {
    fn from(value: &str) -> Self {
        Self::Full(value.to_string().into())
    }
}

impl From<String> for HighlightsMatch {
    fn from(value: String) -> Self {
        Self::Full(value.into())
    }
}

impl From<SharedString> for HighlightsMatch {
    fn from(value: SharedString) -> Self {
        Self::Full(value)
    }
}

/// A text label element with optional secondary text, masking, and highlighting capabilities.
#[derive(IntoElement)]
pub struct Label {
    style: StyleRefinement,
    label: SharedString,
    secondary: Option<SharedString>,
    children: Vec<AnyElement>,
    focus_target: Option<FocusHandle>,
    disabled: bool,
    masked: bool,
    highlights_text: Option<HighlightsMatch>,
}

impl Label {
    /// Create a new label with the main label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        Self {
            style: Default::default(),
            label,
            secondary: None,
            children: Vec::new(),
            focus_target: None,
            disabled: false,
            masked: false,
            highlights_text: None,
        }
    }

    /// Creates a label without predefined text for fully composed content.
    pub fn empty() -> Self {
        Self::new(SharedString::default())
    }

    /// Focuses the associated control when this label receives a primary mouse press.
    ///
    /// This provides native desktop pointer behavior but does not create an
    /// AccessKit `labelled_by` relation. The target control must still expose
    /// its own accessible name.
    pub fn for_focus(mut self, focus_target: &FocusHandle) -> Self {
        self.focus_target = Some(focus_target.clone());
        self
    }

    /// Set the secondary text for the label,
    /// the secondary text will be displayed after the label text with `muted` color.
    pub fn secondary(mut self, secondary: impl Into<SharedString>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    /// Set whether to mask the label text.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Set for matching text to highlight in the label.
    pub fn highlights(mut self, text: impl Into<HighlightsMatch>) -> Self {
        self.highlights_text = Some(text.into());
        self
    }

    fn full_text(&self) -> SharedString {
        match &self.secondary {
            Some(secondary) => format!("{} {}", self.label, secondary).into(),
            None => self.label.clone(),
        }
    }

    /// Returns original-text byte ranges for a Unicode-safe case-insensitive match.
    fn case_insensitive_ranges(text: &str, needle: &str, prefix_only: bool) -> Vec<Range<usize>> {
        let needle_lower = needle.to_lowercase();
        if needle_lower.is_empty() {
            return Vec::new();
        }

        let mut ranges = Vec::new();
        for (start, _) in text.char_indices() {
            if prefix_only && start != 0 {
                break;
            }

            // Grow a candidate along original character boundaries so every
            // resulting byte range remains valid even when lowercasing expands.
            let mut candidate_lower = String::new();
            for (relative_start, character) in text[start..].char_indices() {
                candidate_lower.extend(character.to_lowercase());
                let end = start + relative_start + character.len_utf8();

                if candidate_lower == needle_lower {
                    ranges.push(start..end);
                    break;
                }

                if !needle_lower.starts_with(&candidate_lower) {
                    break;
                }
            }

            if prefix_only {
                break;
            }
        }

        ranges
    }

    fn highlight_ranges(&self, _total_length: usize) -> Vec<Range<usize>> {
        if self.masked {
            return Vec::new();
        }

        let mut ranges = Vec::new();
        let full_text = self.full_text();

        if self.secondary.is_some() {
            ranges.push(0..self.label.len());
            ranges.push(self.label.len()..full_text.len());
        }

        if let Some(matched) = &self.highlights_text {
            let matched_str = matched.as_str();
            if !matched_str.is_empty() {
                ranges.extend(Self::case_insensitive_ranges(
                    &full_text,
                    matched_str,
                    matched.is_prefix(),
                ));
            }
        }

        ranges
    }

    fn measure_highlights(
        &self,
        length: usize,
        cx: &mut App,
    ) -> Option<Vec<(Range<usize>, HighlightStyle)>> {
        let ranges = self.highlight_ranges(length);
        if ranges.is_empty() {
            return None;
        }

        let mut highlights = Vec::new();
        let mut highlight_ranges_added = 0;

        if self.secondary.is_some() {
            highlights.push((ranges[0].clone(), HighlightStyle::default()));
            highlights.push((
                ranges[1].clone(),
                HighlightStyle {
                    color: Some(cx.theme().muted_foreground),
                    ..Default::default()
                },
            ));
            highlight_ranges_added = 2;
        }

        for range in ranges.iter().skip(highlight_ranges_added) {
            highlights.push((
                range.clone(),
                HighlightStyle {
                    color: Some(cx.theme().blue),
                    ..Default::default()
                },
            ));
        }

        Some(gpui::combine_highlights(vec![], highlights).collect())
    }
}

impl ParentElement for Label {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for Label {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut text = self.full_text();
        let chars_count = text.chars().count();
        let disabled = self.disabled;

        if self.masked {
            text = SharedString::from(MASKED.repeat(chars_count))
        };

        let highlights = self.measure_highlights(text.len(), cx);
        let focus_target = self.focus_target;

        div()
            .h_flex()
            .items_center()
            .gap_2()
            .text_sm()
            .font_medium()
            .line_height(relative(1.))
            .text_color(cx.theme().foreground)
            .when(disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .child(
                StyledText::new(&text).when_some(highlights, |this, hl| this.with_highlights(hl)),
            )
            .children(self.children)
            .when_some(
                if disabled { None } else { focus_target },
                |this, focus_target| {
                    this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        focus_target.focus(window, cx);
                    })
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn test_highlight_ranges() {
        // Basic functionality

        // No highlights
        let label = Label::new("Hello World");
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result, Vec::<Range<usize>>::new());

        // Secondary text ranges only
        let label = Label::new("Hello").secondary("World");
        let total_length = "Hello World".len();
        let result = label.highlight_ranges(total_length);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0..5); // "Hello"
        assert_eq!(result[1], 5..11); // " World"

        // Text highlighting

        // Single match with case insensitive
        let label = Label::new("Hello World").highlights("WORLD");
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 6..11); // "World"

        // Multiple matches
        let label = Label::new("Hello Hello Hello").highlights("Hello");
        let result = label.highlight_ranges("Hello Hello Hello".len());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0..5); // First "Hello"
        assert_eq!(result[1], 6..11); // Second "Hello"
        assert_eq!(result[2], 12..17); // Third "Hello"

        // No match and empty search
        let label = Label::new("Hello World").highlights("xyz");
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result, Vec::<Range<usize>>::new());

        let label = Label::new("Hello World").highlights("");
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result, Vec::<Range<usize>>::new());

        // Combined functionality

        // Secondary + highlights in main text
        let label = Label::new("Hello").secondary("World").highlights("llo");
        let total_length = "Hello World".len();
        let result = label.highlight_ranges(total_length);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0..5); // Main text range
        assert_eq!(result[1], 5..11); // Secondary text range
        assert_eq!(result[2], 2..5); // "llo" in main text

        // Highlight in secondary text
        let label = Label::new("Hello").secondary("World").highlights("World");
        let total_length = "Hello World".len();
        let result = label.highlight_ranges(total_length);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0..5); // Main text range
        assert_eq!(result[1], 5..11); // Secondary text range
        assert_eq!(result[2], 6..11); // "World" in secondary text

        // Cross-boundary highlight
        let label = Label::new("Hello").secondary("World").highlights("o W");
        let total_length = "Hello World".len();
        let result = label.highlight_ranges(total_length);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0..5); // Main text range
        assert_eq!(result[1], 5..11); // Secondary text range
        assert_eq!(result[2], 4..7); // "o W" across boundary

        // Edge cases

        // Overlapping matches
        let label = Label::new("aaaa").highlights("aa");
        let result = label.highlight_ranges("aaaa".len());
        assert!(result.len() >= 2);
        assert_eq!(result[0], 0..2); // First "aa"
        assert_eq!(result[1], 1..3); // Overlapping "aa"

        // Unicode text
        let label = Label::new("你好世界，Hello World").highlights("世界");
        let result = label.highlight_ranges("你好世界，Hello World".len());
        assert_eq!(result.len(), 1);
        let text = "你好世界，Hello World";
        let start = text.find("世界").unwrap();
        let end = start + "世界".len();
        assert_eq!(result[0], start..end);
    }

    #[test]
    fn test_highlight_ranges_prefix() {
        // Test prefix match - should only match the first occurrence
        let label = Label::new("aaaa").highlights(HighlightsMatch::Prefix("aa".into()));
        let result = label.highlight_ranges("aaaa".len());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0..2); // Only first "aa"

        // Test prefix vs full match behavior
        let label_full =
            Label::new("Hello Hello").highlights(HighlightsMatch::Full("Hello".into()));
        let result_full = label_full.highlight_ranges("Hello Hello".len());
        assert_eq!(result_full.len(), 2); // Both "Hello" matches

        let label_prefix =
            Label::new("Hello Hello").highlights(HighlightsMatch::Prefix("Hello".into()));
        let result_prefix = label_prefix.highlight_ranges("Hello Hello".len());
        assert_eq!(result_prefix.len(), 1); // Only first "Hello"
        assert_eq!(result_prefix[0], 0..5);

        // Test prefix with case insensitive matching
        let label =
            Label::new("Hello hello HELLO").highlights(HighlightsMatch::Prefix("hello".into()));
        let result = label.highlight_ranges("Hello hello HELLO".len());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0..5); // First "Hello" (case insensitive)

        // Test prefix with no match
        let label = Label::new("Hello World").highlights(HighlightsMatch::Prefix("xyz".into()));
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result.len(), 0);

        // Test prefix with empty string
        let label = Label::new("Hello World").highlights(HighlightsMatch::Prefix("".into()));
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result.len(), 0);

        // Test prefix with secondary text - match in main text
        let label = Label::new("Hello")
            .secondary("Hello World")
            .highlights(HighlightsMatch::Prefix("Hello".into()));
        let total_length = "Hello Hello World".len();
        let result = label.highlight_ranges(total_length);
        assert_eq!(result.len(), 3); // 2 for secondary + 1 for prefix match
        assert_eq!(result[0], 0..5); // Main text range
        assert_eq!(result[1], 5..17); // Secondary text range
        assert_eq!(result[2], 0..5); // First "Hello" prefix match in main text

        // Test prefix with secondary text - match spans boundary (now no match since "abc" is not at start of full text)
        let label = Label::new("abc")
            .secondary("def abc def")
            .highlights(HighlightsMatch::Prefix("abc".into()));
        let total_length = "abc def abc def".len();
        let result = label.highlight_ranges(total_length);
        assert_eq!(result.len(), 3); // 2 for secondary + 1 for prefix match
        assert_eq!(result[0], 0..3); // Main text range
        assert_eq!(result[1], 3..15); // Secondary text range
        assert_eq!(result[2], 0..3); // "abc" matches at start of full text

        // Test prefix with Unicode characters
        let label = Label::new("你好世界你好").highlights(HighlightsMatch::Prefix("你好".into()));
        let result = label.highlight_ranges("你好世界你好".len());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0..6); // First "你好" (6 bytes in UTF-8)

        // Test prefix with overlapping pattern
        let label = Label::new("abababab").highlights(HighlightsMatch::Prefix("abab".into()));
        let result = label.highlight_ranges("abababab".len());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0..4); // First "abab" only

        // Test prefix match at different positions (now no match since "Hello" is not at start)
        let label =
            Label::new("xyz Hello abc Hello").highlights(HighlightsMatch::Prefix("Hello".into()));
        let result = label.highlight_ranges("xyz Hello abc Hello".len());
        assert_eq!(result.len(), 0); // No match since "Hello" is not at the beginning

        // Test is_prefix method
        let prefix_match = HighlightsMatch::Prefix("test".into());
        let full_match = HighlightsMatch::Full("test".into());
        assert!(prefix_match.is_prefix());
        assert!(!full_match.is_prefix());

        // Test as_str method for prefix
        let prefix_match = HighlightsMatch::Prefix("test".into());
        assert_eq!(prefix_match.as_str(), "test");
    }

    #[test]
    fn unicode_case_folding_keeps_original_byte_boundaries() {
        let text = "İX";
        let label = Label::new(text).highlights("x");
        let ranges = label.highlight_ranges(text.len());

        assert_eq!(ranges, vec![2..3]);
        assert!(ranges
            .iter()
            .all(|range| text.is_char_boundary(range.start) && text.is_char_boundary(range.end)));
    }

    #[test]
    fn masked_labels_do_not_apply_ranges_from_unmasked_text() {
        let label = Label::new("秘密 Secret")
            .secondary("optional")
            .highlights("secret")
            .masked(true);

        assert!(
            label
                .highlight_ranges("秘密 Secret optional".len())
                .is_empty()
        );
    }

    #[test]
    fn supports_composed_and_disabled_labels() {
        let label = Label::empty().disabled(true).child("Composed label");

        assert!(label.label.is_empty());
        assert!(label.disabled);
        assert_eq!(label.children.len(), 1);
    }

    #[gpui::test]
    fn focus_target_builder_preserves_the_association(cx: &mut TestAppContext) {
        let target = cx.update(|cx| cx.focus_handle());
        let label = Label::new("Username").for_focus(&target);

        assert_eq!(label.focus_target, Some(target));
    }
}
