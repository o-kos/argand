//! Client-side frame geometry and its resize regions.

use gpui::{
    App, Bounds, BoxShadow, Corners, CursorStyle, Decorations, Edges, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, ResizeEdge, Size, Styled, Tiling, Window, div,
    point, prelude::FluentBuilder, px, size,
};
use gpui_component::ActiveTheme;

const SHADOW: Pixels = px(12.0);
const RESIZE_GRIP: Pixels = px(6.0);
const RADIUS: Pixels = px(8.0);
pub const WAVEFORM_HEIGHT: Pixels = px(64.0);

pub struct Frame {
    padding: Edges<Pixels>,
    pub corners: Corners<Pixels>,
    regions: Vec<(ResizeEdge, Bounds<Pixels>)>,
    shadow: bool,
}

impl Frame {
    pub fn for_window(window: &mut Window) -> Self {
        let decorated = cfg!(target_os = "linux")
            && matches!(window.window_decorations(), Decorations::Client { .. });
        let tiling = match window.window_decorations() {
            Decorations::Client { tiling } if decorated => Some(tiling),
            _ => None,
        };
        window.set_client_inset(if decorated { SHADOW } else { px(0.0) });
        Self::new(
            window.viewport_size(),
            tiling,
            window.is_maximized() || window.is_fullscreen(),
        )
    }

    fn new(viewport: Size<Pixels>, tiling: Option<Tiling>, expanded: bool) -> Self {
        let Some(tiling) = tiling.filter(|_| !expanded) else {
            return Self {
                padding: Edges::default(),
                corners: Corners::default(),
                regions: Vec::new(),
                shadow: false,
            };
        };
        let inset = |tiled| if tiled { px(0.0) } else { SHADOW };
        let radius = |joined| if joined { px(0.0) } else { RADIUS };
        let padding = Edges {
            top: inset(tiling.top),
            right: inset(tiling.right),
            bottom: inset(tiling.bottom),
            left: inset(tiling.left),
        };
        Self {
            padding,
            corners: Corners {
                top_left: radius(tiling.top || tiling.left),
                top_right: radius(tiling.top || tiling.right),
                bottom_left: radius(tiling.bottom || tiling.left),
                bottom_right: radius(tiling.bottom || tiling.right),
            },
            // Some compositors exclude shadows from pointer input. Extend
            // each grip into the visible edge instead of relying on the shadow.
            regions: resize_regions(
                viewport,
                padding.map(|&inset| {
                    if inset > px(0.0) {
                        inset + RESIZE_GRIP
                    } else {
                        inset
                    }
                }),
            ),
            shadow: !tiling.is_tiled(),
        }
    }

    pub fn render(self, content: impl IntoElement, cx: &App) -> impl IntoElement {
        let border = |inset: Pixels| if inset > px(0.0) { px(1.0) } else { px(0.0) };
        div()
            .id("window-frame")
            .relative()
            .size_full()
            .cursor(CursorStyle::Arrow)
            .pt(self.padding.top)
            .pr(self.padding.right)
            .pb(self.padding.bottom)
            .pl(self.padding.left)
            .child(
                div()
                    .size_full()
                    .rounded_tl(self.corners.top_left)
                    .rounded_tr(self.corners.top_right)
                    .rounded_bl(self.corners.bottom_left)
                    .rounded_br(self.corners.bottom_right)
                    .border_t(border(self.padding.top))
                    .border_r(border(self.padding.right))
                    .border_b(border(self.padding.bottom))
                    .border_l(border(self.padding.left))
                    .border_color(cx.theme().window_border)
                    .bg(cx.theme().background)
                    .when(self.shadow, |frame| {
                        frame.shadow(vec![BoxShadow {
                            color: gpui::black().opacity(0.3),
                            blur_radius: SHADOW / 2.0,
                            spread_radius: px(0.0),
                            offset: point(px(0.0), px(0.0)),
                        }])
                    })
                    .child(content),
            )
            .children(self.regions.into_iter().map(|(edge, bounds)| {
                div()
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .cursor(resize_cursor(edge))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                        window.start_window_resize(edge);
                    })
            }))
    }
}

fn resize_regions(
    viewport: Size<Pixels>,
    padding: Edges<Pixels>,
) -> Vec<(ResizeEdge, Bounds<Pixels>)> {
    let columns = [
        (px(0.0), padding.left),
        (padding.left, viewport.width - padding.left - padding.right),
        (viewport.width - padding.right, padding.right),
    ];
    let rows = [
        (px(0.0), padding.top),
        (padding.top, viewport.height - padding.top - padding.bottom),
        (viewport.height - padding.bottom, padding.bottom),
    ];
    let mut regions = Vec::with_capacity(8);
    for (row, (y, height)) in rows.into_iter().enumerate() {
        for (column, (x, width)) in columns.into_iter().enumerate() {
            let edge = match (row, column) {
                (0, 0) => ResizeEdge::TopLeft,
                (0, 1) => ResizeEdge::Top,
                (0, 2) => ResizeEdge::TopRight,
                (1, 0) => ResizeEdge::Left,
                (1, 2) => ResizeEdge::Right,
                (2, 0) => ResizeEdge::BottomLeft,
                (2, 1) => ResizeEdge::Bottom,
                (2, 2) => ResizeEdge::BottomRight,
                _ => continue,
            };
            if width > px(0.0) && height > px(0.0) {
                regions.push((edge, Bounds::new(point(x, y), size(width, height))));
            }
        }
    }
    regions
}

fn resize_cursor(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

#[cfg(test)]
mod tests {
    include!("chrome_tests.rs");
}
