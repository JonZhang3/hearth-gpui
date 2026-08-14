use std::{collections::HashMap, rc::Rc, time::Duration};

use anyhow::{Result, anyhow, bail};
use gpui::{App, Global, Pixels, Point, SharedString, point, px};
pub use hearth_gpui_motion::MotionEasing;

use crate::Size;

/// The semantic density of a style preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    Compact,
    Standard,
    Comfortable,
}

/// Corner radii shared by controls and surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusMetrics {
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
}

/// Resolved geometry for one control size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlSizeMetrics {
    pub height: Pixels,
    pub padding_x: Pixels,
    pub icon_edge_padding: Pixels,
    pub gap: Pixels,
    pub icon_size: Pixels,
}

/// Geometry shared by form controls and buttons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    pub xs: ControlSizeMetrics,
    pub sm: ControlSizeMetrics,
    pub md: ControlSizeMetrics,
    pub lg: ControlSizeMetrics,
}

/// Resolved geometry for one Avatar size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvatarSizeMetrics {
    pub diameter: Pixels,
    pub fallback_text_size: Pixels,
    pub fallback_icon_size: Pixels,
    pub badge_size: Pixels,
    pub badge_icon_size: Option<Pixels>,
    pub count_icon_size: Pixels,
}

/// Geometry shared by Avatar, AvatarBadge, AvatarGroup, and AvatarGroupCount.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvatarMetrics {
    pub xs: AvatarSizeMetrics,
    pub sm: AvatarSizeMetrics,
    pub md: AvatarSizeMetrics,
    pub lg: AvatarSizeMetrics,
    pub outline_width: Pixels,
    pub group_overlap: Pixels,
    pub group_ring_width: Pixels,
}

impl AvatarMetrics {
    /// Returns the resolved geometry for a semantic or custom Avatar size.
    pub fn for_size(&self, size: Size) -> AvatarSizeMetrics {
        match size {
            Size::XSmall => self.xs,
            Size::Small => self.sm,
            Size::Medium => self.md,
            Size::Large => self.lg,
            Size::Size(diameter) => {
                let ratio = diameter.as_f32() / self.md.diameter.as_f32();
                AvatarSizeMetrics {
                    diameter,
                    fallback_text_size: self.md.fallback_text_size * ratio,
                    fallback_icon_size: self.md.fallback_icon_size * ratio,
                    badge_size: self.md.badge_size * ratio,
                    badge_icon_size: self.md.badge_icon_size.map(|size| size * ratio),
                    count_icon_size: self.md.count_icon_size * ratio,
                }
            }
        }
    }
}

impl ControlMetrics {
    /// Returns the resolved metrics for a semantic size.
    pub fn for_size(&self, size: Size) -> ControlSizeMetrics {
        match size {
            Size::XSmall => self.xs,
            Size::Small => self.sm,
            Size::Medium => self.md,
            Size::Large => self.lg,
            Size::Size(height) => ControlSizeMetrics { height, ..self.md },
        }
    }
}

/// Geometry shared by floating surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayMetrics {
    pub padding: Pixels,
    pub gap: Pixels,
    pub side_offset: Pixels,
    pub enter_scale: f32,
}

/// Geometry and presentation used by modal confirmation surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModalMetrics {
    pub default_width: Pixels,
    pub small_width: Pixels,
    pub padding: Pixels,
    pub gap: Pixels,
    pub header_gap: Pixels,
    pub title_font_size: Pixels,
    pub media_size: Pixels,
    pub media_icon_size: Pixels,
    pub overlay_opacity: f32,
    pub ring_opacity: f32,
    pub footer_padding: Pixels,
    pub footer_separated: bool,
    pub footer_tinted: bool,
    pub media_round: bool,
}

impl OverlayMetrics {
    /// Returns the initial translation for an overlay entering from a side.
    ///
    /// The offset points toward the trigger so the surface settles away from
    /// it without changing layout geometry.
    pub fn enter_offset(&self, placement: OverlayPlacement) -> Point<Pixels> {
        match placement {
            OverlayPlacement::Top => point(px(0.), self.side_offset),
            OverlayPlacement::Right => point(-self.side_offset, px(0.)),
            OverlayPlacement::Bottom => point(px(0.), -self.side_offset),
            OverlayPlacement::Left => point(self.side_offset, px(0.)),
        }
    }
}

/// Physical side used to resolve placement-aware overlay motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPlacement {
    Top,
    Right,
    Bottom,
    Left,
}

/// Focus ring geometry. Color remains owned by the Color Theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusMetrics {
    pub ring_width: Pixels,
    pub ring_offset: Pixels,
}

/// Geometry and appearance shared by disclosure components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisclosureMetrics {
    pub trigger_padding_x: Pixels,
    pub trigger_padding_y: Pixels,
    pub content_padding_x: Pixels,
    pub content_padding_bottom: Pixels,
    pub title_gap: Pixels,
    pub indicator_size: Pixels,
    pub trigger_radius: Pixels,
    pub frame_radius: Pixels,
    pub framed_by_default: bool,
    pub open_tint: bool,
}

/// Shared elevation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElevationMetrics {
    pub enabled: bool,
}

/// Named motion durations and easing curves used by component transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionMetrics {
    pub fast: Duration,
    pub normal: Duration,
    pub slow: Duration,
    pub emphasis: Duration,
    pub loading: Duration,
    pub enter_easing: MotionEasing,
    pub exit_easing: MotionEasing,
    pub move_easing: MotionEasing,
}

impl MotionMetrics {
    /// Returns the duration for immediate overlay and feedback transitions.
    pub fn fast(&self) -> Duration {
        self.fast
    }

    /// Returns the default component transition duration.
    pub fn normal(&self) -> Duration {
        self.normal
    }

    /// Returns the duration for larger disclosure transitions.
    pub fn slow(&self) -> Duration {
        self.slow
    }

    /// Returns the duration for deliberate emphasis transitions.
    pub fn emphasis(&self) -> Duration {
        self.emphasis
    }

    /// Returns the duration for continuous loading indicators.
    pub fn loading(&self) -> Duration {
        self.loading
    }
}

/// Table geometry retained by GPUI-native data components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataMetrics {
    pub row_heights: [Pixels; 4],
    pub cell_padding_x: [Pixels; 4],
    pub cell_padding_y: [Pixels; 4],
}

/// A complete geometry and motion preset independent from Color Themes.
#[derive(Debug, Clone, PartialEq)]
pub struct StylePreset {
    pub id: SharedString,
    pub name: SharedString,
    pub density: Density,
    pub radii: RadiusMetrics,
    pub controls: ControlMetrics,
    pub avatars: AvatarMetrics,
    pub overlays: OverlayMetrics,
    pub modals: ModalMetrics,
    pub focus: FocusMetrics,
    pub disclosure: DisclosureMetrics,
    pub elevation: ElevationMetrics,
    pub motion: MotionMetrics,
    pub data: DataMetrics,
}

impl StylePreset {
    /// Validates invariants before a preset becomes visible to components.
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("style preset id cannot be empty");
        }
        if self.name.trim().is_empty() {
            bail!("style preset name cannot be empty");
        }

        let sizes = [
            self.controls.xs,
            self.controls.sm,
            self.controls.md,
            self.controls.lg,
        ];
        if sizes.iter().any(|metrics| {
            !is_positive(metrics.height)
                || !is_non_negative(metrics.padding_x)
                || !is_non_negative(metrics.icon_edge_padding)
                || !is_non_negative(metrics.gap)
                || !is_positive(metrics.icon_size)
        }) {
            bail!("style preset control metrics must be finite and non-negative");
        }
        if sizes.windows(2).any(|pair| pair[0].height > pair[1].height) {
            bail!("style preset control heights must be ordered from xs to lg");
        }
        let avatar_sizes = [
            self.avatars.xs,
            self.avatars.sm,
            self.avatars.md,
            self.avatars.lg,
        ];
        if avatar_sizes.iter().any(|metrics| {
            !is_positive(metrics.diameter)
                || !is_positive(metrics.fallback_text_size)
                || !is_positive(metrics.fallback_icon_size)
                || !is_positive(metrics.badge_size)
                || metrics
                    .badge_icon_size
                    .is_some_and(|size| !is_positive(size))
                || !is_positive(metrics.count_icon_size)
        }) {
            bail!("style preset avatar metrics must be finite and positive");
        }
        if avatar_sizes
            .windows(2)
            .any(|pair| pair[0].diameter > pair[1].diameter)
        {
            bail!("style preset avatar diameters must be ordered from xs to lg");
        }
        if !is_positive(self.avatars.outline_width)
            || !is_positive(self.avatars.group_overlap)
            || !is_positive(self.avatars.group_ring_width)
        {
            bail!("style preset avatar outline and group metrics must be positive");
        }
        let radii = [self.radii.sm, self.radii.md, self.radii.lg, self.radii.xl];
        if radii.windows(2).any(|pair| pair[0] > pair[1]) {
            bail!("style preset radii must be ordered from sm to xl");
        }
        if !self.overlays.enter_scale.is_finite()
            || !(0.0..=1.0).contains(&self.overlays.enter_scale)
        {
            bail!("style preset overlay enter scale must be between 0 and 1");
        }
        if !self.modals.overlay_opacity.is_finite()
            || !(0.0..=1.0).contains(&self.modals.overlay_opacity)
            || !self.modals.ring_opacity.is_finite()
            || !(0.0..=1.0).contains(&self.modals.ring_opacity)
        {
            bail!("style preset modal opacity must be between 0 and 1");
        }
        if [
            self.modals.default_width,
            self.modals.small_width,
            self.modals.title_font_size,
            self.modals.media_size,
            self.modals.media_icon_size,
        ]
        .iter()
        .any(|value| !is_positive(*value))
        {
            bail!("style preset modal sizes must be finite and positive");
        }
        if [
            self.radii.sm,
            self.radii.md,
            self.radii.lg,
            self.radii.xl,
            self.overlays.padding,
            self.overlays.gap,
            self.overlays.side_offset,
            self.modals.padding,
            self.modals.gap,
            self.modals.header_gap,
            self.modals.footer_padding,
            self.focus.ring_width,
            self.focus.ring_offset,
            self.disclosure.trigger_padding_x,
            self.disclosure.trigger_padding_y,
            self.disclosure.content_padding_x,
            self.disclosure.content_padding_bottom,
            self.disclosure.title_gap,
            self.disclosure.trigger_radius,
            self.disclosure.frame_radius,
        ]
        .iter()
        .any(|value| !is_non_negative(*value))
        {
            bail!("style preset surface metrics must be finite and non-negative");
        }
        if !is_positive(self.disclosure.indicator_size) {
            bail!("style preset disclosure indicator size must be finite and positive");
        }
        if self
            .data
            .row_heights
            .iter()
            .any(|value| !is_positive(*value))
            || self
                .data
                .cell_padding_x
                .iter()
                .chain(self.data.cell_padding_y.iter())
                .any(|value| !is_non_negative(*value))
        {
            bail!("style preset data metrics must be finite and non-negative");
        }
        if self
            .data
            .row_heights
            .windows(2)
            .any(|pair| pair[0] > pair[1])
        {
            bail!("style preset data row heights must be ordered from xs to lg");
        }
        if [
            self.motion.fast,
            self.motion.normal,
            self.motion.slow,
            self.motion.emphasis,
            self.motion.loading,
        ]
        .contains(&Duration::ZERO)
        {
            bail!("style preset motion durations must be positive");
        }
        if self.motion.fast > self.motion.normal
            || self.motion.normal > self.motion.slow
            || self.motion.slow > self.motion.emphasis
        {
            bail!("style preset motion durations must be ordered from fast to emphasis");
        }
        Ok(())
    }

    /// Returns the default shadcn Vega preset.
    pub fn vega() -> Self {
        Self {
            id: "vega".into(),
            name: "Vega".into(),
            density: Density::Standard,
            radii: radii(6., 8., 10., 14.),
            controls: controls(
                control(24., 8., 6., 4., 12.),
                control(32., 10., 6., 4., 16.),
                control(36., 10., 8., 6., 16.),
                control(40., 10., 8., 6., 16.),
            ),
            avatars: avatars(),
            overlays: overlay(8., 4., 4.),
            modals: modal(
                512., 320., 24., 24., 6., 18., 64., 32., 0.1, 0.1, 0., false, false, false,
            ),
            focus: focus(),
            disclosure: disclosure(0., 12., 0., 16., 8., 16., 8., 14., false, false),
            elevation: ElevationMetrics { enabled: true },
            motion: motion(),
            data: data([26., 30., 32., 40.], [4., 6., 8., 12.], [2., 3., 4., 8.]),
        }
    }

    /// Returns the compact shadcn Nova preset.
    pub fn nova() -> Self {
        Self {
            id: "nova".into(),
            name: "Nova".into(),
            density: Density::Compact,
            radii: radii(4., 6., 8., 10.),
            controls: controls(
                control(24., 8., 6., 4., 12.),
                control(28., 10., 6., 4., 14.),
                control(32., 10., 8., 6., 16.),
                control(36., 10., 8., 6., 16.),
            ),
            avatars: avatars(),
            overlays: overlay(6., 4., 4.),
            modals: modal(
                384., 320., 16., 16., 6., 16., 40., 24., 0.1, 0.1, 16., true, true, false,
            ),
            focus: focus(),
            disclosure: disclosure(0., 10., 0., 10., 8., 16., 8., 10., false, false),
            elevation: ElevationMetrics { enabled: true },
            motion: motion(),
            data: data([24., 28., 30., 36.], [4., 6., 8., 10.], [2., 2., 3., 6.]),
        }
    }

    /// Returns the comfortable shadcn Maia preset.
    pub fn maia() -> Self {
        Self {
            id: "maia".into(),
            name: "Maia".into(),
            density: Density::Comfortable,
            radii: radii(8., 10., 14., 18.),
            controls: controls(
                control(24., 10., 8., 4., 12.),
                control(32., 12., 8., 4., 16.),
                control(36., 12., 10., 6., 16.),
                control(40., 16., 12., 6., 16.),
            ),
            avatars: avatars(),
            overlays: overlay(12., 6., 6.),
            modals: modal(
                448., 320., 24., 24., 6., 18., 64., 32., 0.8, 0.05, 0., false, false, true,
            ),
            focus: focus(),
            disclosure: disclosure(16., 12., 16., 16., 8., 16., 0., 18., true, true),
            elevation: ElevationMetrics { enabled: true },
            motion: motion(),
            data: data([28., 32., 36., 44.], [6., 8., 10., 14.], [3., 4., 6., 9.]),
        }
    }
}

impl Default for StylePreset {
    fn default() -> Self {
        Self::vega()
    }
}

/// Registry for built-in and application-provided Style Presets.
#[derive(Debug, Default)]
pub struct StyleRegistry {
    presets: HashMap<SharedString, Rc<StylePreset>>,
}

impl Global for StyleRegistry {}

impl StyleRegistry {
    /// Returns the global Style Registry.
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns the mutable global Style Registry.
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Registers a fully validated preset under its stable id.
    pub fn register(preset: StylePreset, cx: &mut App) -> Result<Rc<StylePreset>> {
        preset.validate()?;
        let registry = Self::global_mut(cx);
        if registry.presets.contains_key(&preset.id) {
            bail!("style preset '{}' is already registered", preset.id);
        }
        let preset = Rc::new(preset);
        registry.presets.insert(preset.id.clone(), preset.clone());
        Ok(preset)
    }

    /// Returns a preset by stable id.
    pub fn get(id: &str, cx: &App) -> Option<Rc<StylePreset>> {
        Self::global(cx).presets.get(id).cloned()
    }

    /// Returns presets sorted by display name.
    pub fn sorted_styles(cx: &App) -> Vec<Rc<StylePreset>> {
        let mut presets = Self::global(cx)
            .presets
            .values()
            .cloned()
            .collect::<Vec<_>>();
        presets.sort_by_key(|preset| preset.name.to_lowercase());
        presets
    }
}

pub(super) fn init(cx: &mut App) {
    cx.set_global(StyleRegistry::default());
    for preset in [
        StylePreset::vega(),
        StylePreset::nova(),
        StylePreset::maia(),
    ] {
        StyleRegistry::register(preset, cx).expect("built-in style presets must be valid");
    }
}

fn control(
    height: f32,
    padding_x: f32,
    icon_edge_padding: f32,
    gap: f32,
    icon_size: f32,
) -> ControlSizeMetrics {
    ControlSizeMetrics {
        height: px(height),
        padding_x: px(padding_x),
        icon_edge_padding: px(icon_edge_padding),
        gap: px(gap),
        icon_size: px(icon_size),
    }
}

fn controls(
    xs: ControlSizeMetrics,
    sm: ControlSizeMetrics,
    md: ControlSizeMetrics,
    lg: ControlSizeMetrics,
) -> ControlMetrics {
    ControlMetrics { xs, sm, md, lg }
}

fn avatar_size(
    diameter: f32,
    fallback_text_size: f32,
    fallback_icon_size: f32,
    badge_size: f32,
    badge_icon_size: Option<f32>,
    count_icon_size: f32,
) -> AvatarSizeMetrics {
    AvatarSizeMetrics {
        diameter: px(diameter),
        fallback_text_size: px(fallback_text_size),
        fallback_icon_size: px(fallback_icon_size),
        badge_size: px(badge_size),
        badge_icon_size: badge_icon_size.map(px),
        count_icon_size: px(count_icon_size),
    }
}

fn avatars() -> AvatarMetrics {
    AvatarMetrics {
        xs: avatar_size(16., 10., 10., 6., None, 10.),
        sm: avatar_size(24., 12., 12., 8., None, 12.),
        md: avatar_size(32., 14., 16., 10., Some(8.), 16.),
        lg: avatar_size(40., 14., 20., 12., Some(8.), 20.),
        outline_width: px(1.),
        group_overlap: px(8.),
        group_ring_width: px(2.),
    }
}

fn radii(sm: f32, md: f32, lg: f32, xl: f32) -> RadiusMetrics {
    RadiusMetrics {
        sm: px(sm),
        md: px(md),
        lg: px(lg),
        xl: px(xl),
    }
}

fn overlay(padding: f32, gap: f32, side_offset: f32) -> OverlayMetrics {
    OverlayMetrics {
        padding: px(padding),
        gap: px(gap),
        side_offset: px(side_offset),
        enter_scale: 0.95,
    }
}

#[allow(clippy::too_many_arguments)]
fn modal(
    default_width: f32,
    small_width: f32,
    padding: f32,
    gap: f32,
    header_gap: f32,
    title_font_size: f32,
    media_size: f32,
    media_icon_size: f32,
    overlay_opacity: f32,
    ring_opacity: f32,
    footer_padding: f32,
    footer_separated: bool,
    footer_tinted: bool,
    media_round: bool,
) -> ModalMetrics {
    ModalMetrics {
        default_width: px(default_width),
        small_width: px(small_width),
        padding: px(padding),
        gap: px(gap),
        header_gap: px(header_gap),
        title_font_size: px(title_font_size),
        media_size: px(media_size),
        media_icon_size: px(media_icon_size),
        overlay_opacity,
        ring_opacity,
        footer_padding: px(footer_padding),
        footer_separated,
        footer_tinted,
        media_round,
    }
}

fn focus() -> FocusMetrics {
    FocusMetrics {
        ring_width: px(3.),
        ring_offset: px(0.),
    }
}

#[allow(clippy::too_many_arguments)]
fn disclosure(
    trigger_padding_x: f32,
    trigger_padding_y: f32,
    content_padding_x: f32,
    content_padding_bottom: f32,
    title_gap: f32,
    indicator_size: f32,
    trigger_radius: f32,
    frame_radius: f32,
    framed_by_default: bool,
    open_tint: bool,
) -> DisclosureMetrics {
    DisclosureMetrics {
        trigger_padding_x: px(trigger_padding_x),
        trigger_padding_y: px(trigger_padding_y),
        content_padding_x: px(content_padding_x),
        content_padding_bottom: px(content_padding_bottom),
        title_gap: px(title_gap),
        indicator_size: px(indicator_size),
        trigger_radius: px(trigger_radius),
        frame_radius: px(frame_radius),
        framed_by_default,
        open_tint,
    }
}

fn motion() -> MotionMetrics {
    MotionMetrics {
        fast: Duration::from_millis(100),
        normal: Duration::from_millis(150),
        slow: Duration::from_millis(200),
        emphasis: Duration::from_millis(250),
        loading: Duration::from_secs(1),
        enter_easing: MotionEasing::EaseOutCubic,
        exit_easing: MotionEasing::EaseInCubic,
        move_easing: MotionEasing::EaseInOutCubic,
    }
}

fn data(row_heights: [f32; 4], cell_padding_x: [f32; 4], cell_padding_y: [f32; 4]) -> DataMetrics {
    DataMetrics {
        row_heights: row_heights.map(px),
        cell_padding_x: cell_padding_x.map(px),
        cell_padding_y: cell_padding_y.map(px),
    }
}

fn is_positive(value: Pixels) -> bool {
    value.as_f32().is_finite() && value > px(0.)
}

fn is_non_negative(value: Pixels) -> bool {
    value.as_f32().is_finite() && value >= px(0.)
}

/// Resolves a preset and returns an actionable error for unknown ids.
pub(crate) fn resolve(id: &str, cx: &App) -> Result<Rc<StylePreset>> {
    StyleRegistry::get(id, cx).ok_or_else(|| anyhow!("unknown style preset '{id}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn built_in_styles_have_distinct_medium_control_geometry() {
        let vega = StylePreset::vega();
        let nova = StylePreset::nova();
        let maia = StylePreset::maia();

        assert_ne!(vega.controls.md.height, nova.controls.md.height);
        assert_ne!(vega.controls.md.padding_x, maia.controls.md.padding_x);
        assert!(vega.validate().is_ok());
        assert!(nova.validate().is_ok());
        assert!(maia.validate().is_ok());
    }

    #[test]
    fn built_in_styles_define_distinct_disclosure_geometry() {
        let vega = StylePreset::vega();
        let nova = StylePreset::nova();
        let maia = StylePreset::maia();

        assert_eq!(vega.disclosure.trigger_padding_y, px(12.));
        assert_eq!(nova.disclosure.trigger_padding_y, px(10.));
        assert_eq!(maia.disclosure.trigger_padding_x, px(16.));
        assert!(!vega.disclosure.framed_by_default);
        assert!(!nova.disclosure.framed_by_default);
        assert!(maia.disclosure.framed_by_default);
        assert!(maia.disclosure.open_tint);
    }

    #[test]
    fn built_in_styles_define_shadcn_modal_geometry() {
        let vega = StylePreset::vega();
        let nova = StylePreset::nova();
        let maia = StylePreset::maia();

        assert_eq!(vega.modals.default_width, px(512.));
        assert_eq!(vega.modals.small_width, px(320.));
        assert_eq!(vega.modals.padding, px(24.));
        assert_eq!(vega.modals.title_font_size, px(18.));
        assert!(!vega.modals.footer_separated);

        assert_eq!(nova.modals.default_width, px(384.));
        assert_eq!(nova.modals.padding, px(16.));
        assert_eq!(nova.modals.title_font_size, px(16.));
        assert!(nova.modals.footer_separated);
        assert!(nova.modals.footer_tinted);

        assert_eq!(maia.modals.default_width, px(448.));
        assert_eq!(maia.modals.overlay_opacity, 0.8);
        assert!(maia.modals.media_round);
    }

    #[test]
    fn built_in_styles_define_shadcn_avatar_geometry() {
        for style in [
            StylePreset::vega(),
            StylePreset::nova(),
            StylePreset::maia(),
        ] {
            assert_eq!(style.avatars.sm.diameter, px(24.));
            assert_eq!(style.avatars.md.diameter, px(32.));
            assert_eq!(style.avatars.lg.diameter, px(40.));
            assert_eq!(style.avatars.md.badge_size, px(10.));
            assert_eq!(style.avatars.group_overlap, px(8.));
            assert_eq!(style.avatars.group_ring_width, px(2.));
        }
    }

    #[test]
    fn built_in_styles_cover_control_overlay_and_data_families() {
        let styles = [
            StylePreset::vega(),
            StylePreset::nova(),
            StylePreset::maia(),
        ];

        for style in &styles {
            assert!(style.validate().is_ok());
            assert!(style.controls.xs.height <= style.controls.sm.height);
            assert!(style.controls.sm.height <= style.controls.md.height);
            assert!(style.controls.md.height <= style.controls.lg.height);
            assert!(
                style
                    .data
                    .row_heights
                    .windows(2)
                    .all(|pair| pair[0] <= pair[1])
            );
            assert!(style.radii.sm <= style.radii.md);
            assert!(style.radii.md <= style.radii.lg);
            assert!(style.radii.lg <= style.radii.xl);
        }

        assert_ne!(styles[0].overlays.padding, styles[1].overlays.padding);
        assert_ne!(styles[1].overlays.padding, styles[2].overlays.padding);
        assert_ne!(
            styles[0].modals.default_width,
            styles[1].modals.default_width
        );
        assert_ne!(
            styles[1].modals.default_width,
            styles[2].modals.default_width
        );
        assert_ne!(styles[0].data.row_heights, styles[1].data.row_heights);
        assert_ne!(styles[1].data.row_heights, styles[2].data.row_heights);
    }

    #[test]
    fn motion_easing_clamps_and_preserves_endpoints() {
        for easing in [
            MotionEasing::Linear,
            MotionEasing::EaseInCubic,
            MotionEasing::EaseOutCubic,
            MotionEasing::EaseInOutCubic,
        ] {
            assert_eq!(easing.sample(-1.0), 0.0);
            assert_eq!(easing.sample(0.0), 0.0);
            assert_eq!(easing.sample(1.0), 1.0);
            assert_eq!(easing.sample(2.0), 1.0);
        }
    }

    #[test]
    fn overlay_offsets_follow_physical_placement() {
        let metrics = overlay(8., 4., 6.);

        assert_eq!(
            metrics.enter_offset(OverlayPlacement::Top),
            point(px(0.), px(6.))
        );
        assert_eq!(
            metrics.enter_offset(OverlayPlacement::Right),
            point(px(-6.), px(0.))
        );
        assert_eq!(
            metrics.enter_offset(OverlayPlacement::Bottom),
            point(px(0.), px(-6.))
        );
        assert_eq!(
            metrics.enter_offset(OverlayPlacement::Left),
            point(px(6.), px(0.))
        );
    }

    #[test]
    fn invalid_style_is_rejected_before_registration() {
        let mut preset = StylePreset::vega();
        preset.id = "".into();
        assert!(preset.validate().is_err());

        let mut preset = StylePreset::vega();
        preset.controls.md.height = px(f32::NAN);
        assert!(preset.validate().is_err());

        let mut preset = StylePreset::vega();
        preset.motion.fast = Duration::ZERO;
        assert!(preset.validate().is_err());

        let mut preset = StylePreset::vega();
        preset.radii.md = px(1.);
        assert!(preset.validate().is_err());

        let mut preset = StylePreset::vega();
        preset.data.row_heights[2] = px(1.);
        assert!(preset.validate().is_err());

        let mut preset = StylePreset::vega();
        preset.motion.fast = Duration::from_millis(500);
        assert!(preset.validate().is_err());

        let mut preset = StylePreset::vega();
        preset.disclosure.indicator_size = px(0.);
        assert!(preset.validate().is_err());
    }

    #[gpui::test]
    fn registry_rejects_duplicates(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            assert!(StyleRegistry::register(StylePreset::vega(), cx).is_err());
        });
    }

    #[gpui::test]
    fn style_selection_is_atomic_and_preserves_colors(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            cx.set_global(crate::Theme::default());
            let primary = crate::Theme::global(cx).colors.primary;

            crate::Theme::set_style("nova", cx).unwrap();
            assert_eq!(crate::Theme::global(cx).style.id, "nova");
            assert_eq!(crate::Theme::global(cx).colors.primary, primary);

            let color_theme = Rc::new(crate::ThemeConfig {
                name: "Test Light".into(),
                ..Default::default()
            });
            crate::Theme::set_color_theme(color_theme, cx);
            assert_eq!(crate::Theme::global(cx).style.id, "nova");
            let color_theme_primary = crate::Theme::global(cx).colors.primary;

            assert!(crate::Theme::set_style("missing", cx).is_err());
            assert_eq!(crate::Theme::global(cx).style.id, "nova");
            assert_eq!(crate::Theme::global(cx).colors.primary, color_theme_primary);
        });
    }
}
