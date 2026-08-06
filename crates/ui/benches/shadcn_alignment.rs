use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use gpui::{
    AppContext as _, BenchAppContext, Context, Entity, InteractiveElement as _, IntoElement,
    ListSizingBehavior, ParentElement as _, Render, ScrollStrategy, Styled as _,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder as _, px, uniform_list,
};

static TRACK_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATION_OPERATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static CONTROL_PROBE_PRINTED: AtomicBool = AtomicBool::new(false);
static LOADING_PROBE_PRINTED: AtomicBool = AtomicBool::new(false);
static OVERLAY_PROBE_PRINTED: AtomicBool = AtomicBool::new(false);
static SCROLL_PROBE_PRINTED: AtomicBool = AtomicBool::new(false);

struct TrackingAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: TrackingAllocator = TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_OPERATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() && TRACK_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_OPERATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() && TRACK_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_OPERATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        new_pointer
    }
}

#[derive(Clone, Copy)]
struct AllocationSnapshot {
    operations: u64,
    requested_bytes: u64,
}

/// Counts allocation requests during one synchronous render update after the
/// initial layout and resource caches have been warmed.
fn measure_render_allocations<V: 'static>(
    cx: &mut BenchAppContext,
    view: &Entity<V>,
    update: impl FnOnce(&mut V, &mut Context<V>),
) -> AllocationSnapshot {
    cx.run_until_idle();
    ALLOCATION_OPERATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    TRACK_ALLOCATIONS.store(true, Ordering::SeqCst);
    cx.update(|cx| view.update(cx, update));
    TRACK_ALLOCATIONS.store(false, Ordering::SeqCst);

    AllocationSnapshot {
        operations: ALLOCATION_OPERATIONS.load(Ordering::Relaxed),
        requested_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
    }
}

fn print_allocation_snapshot(name: &str, snapshot: AllocationSnapshot, printed: &AtomicBool) {
    if printed.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!(
        "GPUI allocation probe: {name}: {} operations, {} requested bytes",
        snapshot.operations, snapshot.requested_bytes
    );
}
use gpui_component::{
    Disableable as _, Sizable as _, button::Button, checkbox::Checkbox, h_flex, popover::Popover,
    progress::Progress, radio::Radio, skeleton::Skeleton, spinner::Spinner, switch::Switch, v_flex,
};

/// Representative render workload for high-frequency control state changes.
struct ControlMatrixBench {
    active: bool,
}

impl Render for ControlMatrixBench {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let variant_offset = usize::from(self.active);
        v_flex()
            .size_full()
            .gap_1()
            .children((0_usize..40).map(|index| {
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(("bench-button", index))
                            .small()
                            .when((index + variant_offset) % 4 == 0, |button| button)
                            .when((index + variant_offset) % 4 == 1, |button| button.outline())
                            .disabled(index % 11 == 0)
                            .label("Control"),
                    )
                    .child(
                        Checkbox::new(("bench-checkbox", index))
                            .checked(index % 2 == 0)
                            .label("Checkbox"),
                    )
                    .child(
                        Radio::new(("bench-radio", index))
                            .checked(index % 3 == 0)
                            .label("Radio"),
                    )
                    .child(
                        Switch::new(("bench-switch", index))
                            .checked(index % 2 == 0)
                            .label("Switch"),
                    )
            }))
    }
}

/// Representative render workload for continuously repainting feedback surfaces.
struct LoadingSurfaceBench {
    frame: usize,
}

/// Exercises interrupted overlay ownership by reversing a controlled Popover
/// before prior open or close completion tasks necessarily finish.
struct OverlayToggleBench {
    open: bool,
}

impl Render for OverlayToggleBench {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_8().child(
            Popover::new("bench-popover")
                .open(self.open)
                .trigger(Button::new("bench-popover-trigger").label("Toggle overlay"))
                .content(|_, _, _| div().w(px(280.)).p_4().child("Overlay content")),
        )
    }
}

/// Scrolls between distant positions in a 1,000-row uniform virtual list.
struct VirtualScrollBench {
    target: usize,
    scroll_handle: UniformListScrollHandle,
}

impl Render for VirtualScrollBench {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        uniform_list("bench-virtual-list", 1_000, |visible_range, _, _| {
            visible_range
                .map(|index| {
                    div()
                        .id(("bench-row", index))
                        .h(px(32.))
                        .px_3()
                        .flex()
                        .items_center()
                        .child("Virtual row")
                })
                .collect::<Vec<_>>()
        })
        .size_full()
        .with_sizing_behavior(ListSizingBehavior::Infer)
        .track_scroll(&self.scroll_handle)
    }
}

/// Representative first-surface workload used by the repeated mount benchmark.
struct StartupSurfaceBench;

impl Render for StartupSurfaceBench {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .children((0_usize..24).map(|index| {
                h_flex()
                    .gap_2()
                    .child(Button::new(("startup-button", index)).label("Action"))
                    .child(Checkbox::new(("startup-check", index)).label("Option"))
            }))
    }
}

impl Render for LoadingSurfaceBench {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_1()
            .children((0_usize..48).map(|index| {
                h_flex()
                    .gap_2()
                    .child(Spinner::new().small())
                    .child(Skeleton::new().w_32().h_3())
                    .child(
                        Progress::new(("bench-progress", index))
                            .w_32()
                            .value((self.frame.wrapping_add(index) % 100) as f32)
                            .loading(index % 4 == 0),
                    )
            }))
    }
}

/// Installs gpui-component globals and mounts a benchmark view directly in the
/// headless window. Root is intentionally excluded because its macOS native
/// accessibility bridge requires an application-bundle window.
fn mount_bench_view<V: Render>(
    cx: &mut BenchAppContext,
    reduce_motion: bool,
    build: impl FnOnce() -> V,
) -> gpui::Entity<V> {
    cx.update(|cx| {
        gpui_component::init(cx);
        cx.set_reduce_motion(reduce_motion);
    });
    let mut window = cx.add_empty_window();
    window.update(|window, cx| window.replace_root(cx, |_, _| build()))
}

#[gpui::bench]
fn control_state_matrix_render(cx: &mut BenchAppContext) {
    // State-matrix timing measures final-state rendering. Zero-duration motion also
    // prevents benchmark teardown from retaining component transition tasks.
    let view = mount_bench_view(cx, true, || ControlMatrixBench { active: false });
    let allocations = measure_render_allocations(cx, &view, |view, cx| {
        view.active = !view.active;
        cx.notify();
    });
    print_allocation_snapshot(
        "control_state_matrix_render",
        allocations,
        &CONTROL_PROBE_PRINTED,
    );
    cx.bench_renderer(view, |view, _, cx| {
        view.active = !view.active;
        cx.notify();
    });
    cx.run_until_idle();
}

#[gpui::bench]
fn loading_surface_render(cx: &mut BenchAppContext) {
    let view = mount_bench_view(cx, false, || LoadingSurfaceBench { frame: 0 });
    let allocations = measure_render_allocations(cx, &view, |view, cx| {
        // Keep determinate targets stable; this benchmark isolates continuous
        // loading repaint cost rather than spawning value-transition tasks.
        view.frame = view.frame.wrapping_add(100);
        cx.notify();
    });
    print_allocation_snapshot(
        "loading_surface_render",
        allocations,
        &LOADING_PROBE_PRINTED,
    );
    cx.bench_renderer(view, |view, _, cx| {
        view.frame = view.frame.wrapping_add(100);
        cx.notify();
    });
}

#[gpui::bench]
fn rapid_overlay_toggle_render(cx: &mut BenchAppContext) {
    let view = mount_bench_view(cx, false, || OverlayToggleBench { open: false });
    let allocations = measure_render_allocations(cx, &view, |view, cx| {
        view.open = !view.open;
        cx.notify();
    });
    print_allocation_snapshot(
        "rapid_overlay_toggle_render",
        allocations,
        &OVERLAY_PROBE_PRINTED,
    );
    cx.bench_renderer(view, |view, _, cx| {
        view.open = !view.open;
        cx.notify();
    });
}

#[gpui::bench]
fn virtual_scroll_1000_rows_render(cx: &mut BenchAppContext) {
    let view = mount_bench_view(cx, true, || VirtualScrollBench {
        target: 0,
        scroll_handle: UniformListScrollHandle::new(),
    });
    let allocations = measure_render_allocations(cx, &view, |view, cx| {
        view.target = 999;
        view.scroll_handle
            .scroll_to_item(view.target, ScrollStrategy::Center);
        cx.notify();
    });
    print_allocation_snapshot(
        "virtual_scroll_1000_rows_render",
        allocations,
        &SCROLL_PROBE_PRINTED,
    );
    cx.bench_renderer(view, |view, _, cx| {
        view.target = if view.target == 0 { 999 } else { 0 };
        view.scroll_handle
            .scroll_to_item(view.target, ScrollStrategy::Center);
        cx.notify();
    });
}

#[gpui::bench]
fn startup_surface_mount(cx: &mut BenchAppContext) {
    cx.update(gpui_component::init);
    let window = cx.add_empty_window().window_handle();
    cx.bench_iter(|cx| {
        cx.update_window(window, |_, window, cx| {
            window.replace_root(cx, |_, _| StartupSurfaceBench)
        })
        .expect("benchmark window should remain open");
    });
}

#[gpui::bench]
fn idle_queue_drain(cx: &mut BenchAppContext) {
    let _view = mount_bench_view(cx, true, || StartupSurfaceBench);
    cx.bench_iter(|cx| cx.run_until_idle());
}

gpui::bench_group!(
    benches,
    control_state_matrix_render,
    loading_surface_render,
    rapid_overlay_toggle_render,
    virtual_scroll_1000_rows_render,
    startup_surface_mount,
    idle_queue_drain,
);
gpui::bench_main!(benches);
