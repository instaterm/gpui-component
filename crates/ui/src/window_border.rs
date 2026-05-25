// From:
// https://github.com/zed-industries/zed/blob/56daba28d40301ee4c05546fadb691d070b7b2b6/crates/gpui/examples/window_shadow.rs
use gpui::{
    AnyElement, App, Bounds, CursorStyle, Decorations, Div, Edges, HitboxBehavior, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    ResizeEdge, Size, Styled as _, Tiling, Window, canvas, div, point, prelude::FluentBuilder as _,
    px,
};

use crate::ActiveTheme;

#[cfg(not(target_os = "linux"))]
pub(crate) const SHADOW_SIZE: Pixels = px(0.0);
#[cfg(target_os = "linux")]
pub(crate) const SHADOW_SIZE: Pixels = px(12.0);
const BORDER_SIZE: Pixels = px(1.0);
pub(crate) const BORDER_RADIUS: Pixels = px(0.0);

/// Pixels by which the resize hit region extends inward from the chrome edge.
///
/// Together with [`SHADOW_SIZE`], the resize zone straddles the visible
/// chrome edge — `GRIP_SIZE` inside chrome + `SHADOW_SIZE` outside in the
/// shadow padding — so users can grab the visual edge directly instead of
/// having to reach into the soft shadow strip.
#[cfg(not(target_os = "linux"))]
pub(crate) const GRIP_SIZE: Pixels = px(0.0);
#[cfg(target_os = "linux")]
pub(crate) const GRIP_SIZE: Pixels = px(4.0);

/// Pixels by which the corner resize hit region extends inward from the
/// chrome corner along each axis. Made larger than [`GRIP_SIZE`] so the
/// diagonal-cursor "sweet spot" stays comfortable to land on; combined with
/// the [`SHADOW_SIZE`] × [`SHADOW_SIZE`] shadow-zone corner this yields
/// roughly a 20 px wide diagonal zone straddling each chrome corner. Tuned
/// to leave the Linux CSD close/maximize/minimize buttons' hit regions
/// untouched (they sit at least 8 px from the chrome right edge and 4 px
/// from the top).
#[cfg(not(target_os = "linux"))]
pub(crate) const CORNER_GRIP_SIZE: Pixels = px(0.0);
#[cfg(target_os = "linux")]
pub(crate) const CORNER_GRIP_SIZE: Pixels = px(8.0);

/// Create a new window border.
pub fn window_border() -> WindowBorder {
    WindowBorder::new()
}

/// Window border use to render a custom window border and shadow for Linux.
#[derive(IntoElement)]
pub struct WindowBorder {
    shadow_size: Pixels,
    grip_size: Pixels,
    children: Vec<AnyElement>,
}

impl Default for WindowBorder {
    fn default() -> Self {
        Self {
            shadow_size: SHADOW_SIZE,
            grip_size: GRIP_SIZE,
            children: Vec::new(),
        }
    }
}

impl WindowBorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the shadow size for typical Linux client-side decorations.
    ///
    /// Default: [`SHADOW_SIZE`]
    pub fn shadow_size(mut self, size: impl Into<Pixels>) -> Self {
        self.shadow_size = size.into();
        self
    }

    /// Set the grip size — pixels by which the resize hit region extends
    /// inward from the chrome edge. Together with the shadow padding this
    /// produces a resize zone that straddles the visible edge.
    ///
    /// Default: [`GRIP_SIZE`]
    pub fn grip_size(mut self, size: impl Into<Pixels>) -> Self {
        self.grip_size = size.into();
        self
    }
}

/// Get the window paddings.
pub fn window_paddings(window: &Window) -> Edges<Pixels> {
    let shadow_size = window.client_inset().unwrap_or(SHADOW_SIZE);
    match window.window_decorations() {
        Decorations::Server => Edges::all(px(0.0)),
        Decorations::Client { tiling } => {
            let mut paddings = Edges::all(shadow_size);
            if tiling.top {
                paddings.top = px(0.0);
            }
            if tiling.bottom {
                paddings.bottom = px(0.0);
            }
            if tiling.left {
                paddings.left = px(0.0);
            }
            if tiling.right {
                paddings.right = px(0.0);
            }
            paddings
        }
    }
}

impl ParentElement for WindowBorder {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for WindowBorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let decorations = window.window_decorations();
        let shadow_size = self.shadow_size;
        let grip_size = self.grip_size;
        window.set_client_inset(shadow_size);

        div()
            .id("window-backdrop")
            .bg(gpui::transparent_black())
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling, .. } => div
                    .bg(gpui::transparent_black())
                    .child(
                        canvas(
                            |_bounds, window, _| {
                                window.insert_hitbox(
                                    Bounds::new(
                                        point(px(0.0), px(0.0)),
                                        window.window_bounds().get_bounds().size,
                                    ),
                                    HitboxBehavior::Normal,
                                )
                            },
                            move |_bounds, hitbox, window, _| {
                                let mouse = window.mouse_position();
                                let size = window.window_bounds().get_bounds().size;
                                let Some(edge) = resize_edge(mouse, shadow_size, size) else {
                                    return;
                                };
                                window.set_cursor_style(
                                    match edge {
                                        ResizeEdge::Top | ResizeEdge::Bottom => {
                                            CursorStyle::ResizeUpDown
                                        }
                                        ResizeEdge::Left | ResizeEdge::Right => {
                                            CursorStyle::ResizeLeftRight
                                        }
                                        ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                                            CursorStyle::ResizeUpLeftDownRight
                                        }
                                        ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                                            CursorStyle::ResizeUpRightDownLeft
                                        }
                                    },
                                    &hitbox,
                                );
                            },
                        )
                        .size_full()
                        .absolute(),
                    )
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(BORDER_RADIUS)
                    })
                    .when(!(tiling.top || tiling.left), |div| {
                        div.rounded_tl(BORDER_RADIUS)
                    })
                    .when(!tiling.top, |div| div.pt(shadow_size))
                    .when(!tiling.bottom, |div| div.pb(shadow_size))
                    .when(!tiling.left, |div| div.pl(shadow_size))
                    .when(!tiling.right, |div| div.pr(shadow_size))
                    .on_mouse_down(MouseButton::Left, move |_, window, _| {
                        let size = window.window_bounds().get_bounds().size;
                        let pos = window.mouse_position();

                        match resize_edge(pos, shadow_size, size) {
                            Some(edge) => window.start_window_resize(edge),
                            None => {}
                        };
                    }),
            })
            .size_full()
            .child(
                div()
                    .cursor(CursorStyle::default())
                    .map(|div| match decorations {
                        Decorations::Server => div,
                        Decorations::Client { tiling } => div
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(BORDER_RADIUS)
                            })
                            .when(!(tiling.top || tiling.left), |div| {
                                div.rounded_tl(BORDER_RADIUS)
                            })
                            .border_color(cx.theme().window_border)
                            .when(!tiling.top, |div| div.border_t(BORDER_SIZE))
                            .when(!tiling.bottom, |div| div.border_b(BORDER_SIZE))
                            .when(!tiling.left, |div| div.border_l(BORDER_SIZE))
                            .when(!tiling.right, |div| div.border_r(BORDER_SIZE))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(vec![gpui::BoxShadow {
                                    color: Hsla {
                                        h: 0.,
                                        s: 0.,
                                        l: 0.,
                                        a: 0.3,
                                    },
                                    blur_radius: shadow_size / 2.,
                                    spread_radius: px(0.),
                                    offset: point(px(0.0), px(0.0)),
                                }])
                            }),
                    })
                    .on_mouse_move(|_e, _, cx| {
                        cx.stop_propagation();
                    })
                    .bg(gpui::transparent_black())
                    .size_full()
                    .children(self.children)
                    // Edge-grip overlay: 4 strips + 4 corners along the chrome
                    // edges, each claiming hit events via `.occlude()`
                    // (BlockMouse) so resize wins over inner UI in the
                    // outermost `grip_size` pixels. Painted last → topmost
                    // hitbox at those positions. Dead code on macOS/Windows
                    // (always `Decorations::Server`) and on Linux without a
                    // compositor (auto-downgrades to Server).
                    .map(|d| match decorations {
                        Decorations::Server => d,
                        Decorations::Client { tiling } => {
                            d.child(edge_grips(tiling, grip_size, CORNER_GRIP_SIZE))
                        }
                    }),
            )
    }
}

fn resize_edge(pos: Point<Pixels>, shadow_size: Pixels, size: Size<Pixels>) -> Option<ResizeEdge> {
    let edge = if pos.y < shadow_size && pos.x < shadow_size {
        ResizeEdge::TopLeft
    } else if pos.y < shadow_size && pos.x > size.width - shadow_size {
        ResizeEdge::TopRight
    } else if pos.y < shadow_size {
        ResizeEdge::Top
    } else if pos.y > size.height - shadow_size && pos.x < shadow_size {
        ResizeEdge::BottomLeft
    } else if pos.y > size.height - shadow_size && pos.x > size.width - shadow_size {
        ResizeEdge::BottomRight
    } else if pos.y > size.height - shadow_size {
        ResizeEdge::Bottom
    } else if pos.x < shadow_size {
        ResizeEdge::Left
    } else if pos.x > size.width - shadow_size {
        ResizeEdge::Right
    } else {
        return None;
    };
    Some(edge)
}

/// Inside-chrome resize grip overlay: 4 edge strips (`grip_size` thick) +
/// 4 square corners (`corner_size` per side) along the inside of the visible
/// chrome. Each grip uses `.occlude()` so its `BlockMouse` hitbox wins over
/// inner UI clicks. Edges/corners whose direction is tiled (e.g. when
/// maximized or half-tiled) are skipped so resize affordances disappear
/// where they would be inert. Edge strips are inset from corners by
/// `corner_size` so the corner zones own those squares exclusively.
fn edge_grips(tiling: Tiling, grip_size: Pixels, corner_size: Pixels) -> impl IntoElement {
    // When a perpendicular edge is tiled, the corner in that direction is
    // skipped — extend the orthogonal edge strip into that corner so we don't
    // leave a `corner_size × grip_size` dead zone where neither corner nor
    // strip covers. Without this, half-tiled windows have a small unreachable
    // patch at the tiled-side end of each remaining resizable edge.
    let h_left_inset = if tiling.left { px(0.0) } else { corner_size };
    let h_right_inset = if tiling.right { px(0.0) } else { corner_size };
    let v_top_inset = if tiling.top { px(0.0) } else { corner_size };
    let v_bottom_inset = if tiling.bottom { px(0.0) } else { corner_size };

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .when(!tiling.top, |d| {
            d.child(edge_grip(
                "window-grip-top",
                ResizeEdge::Top,
                CursorStyle::ResizeUpDown,
                move |g| {
                    g.top_0()
                        .left(h_left_inset)
                        .right(h_right_inset)
                        .h(grip_size)
                },
            ))
        })
        .when(!tiling.bottom, |d| {
            d.child(edge_grip(
                "window-grip-bottom",
                ResizeEdge::Bottom,
                CursorStyle::ResizeUpDown,
                move |g| {
                    g.bottom_0()
                        .left(h_left_inset)
                        .right(h_right_inset)
                        .h(grip_size)
                },
            ))
        })
        .when(!tiling.left, |d| {
            d.child(edge_grip(
                "window-grip-left",
                ResizeEdge::Left,
                CursorStyle::ResizeLeftRight,
                move |g| {
                    g.left_0()
                        .top(v_top_inset)
                        .bottom(v_bottom_inset)
                        .w(grip_size)
                },
            ))
        })
        .when(!tiling.right, |d| {
            d.child(edge_grip(
                "window-grip-right",
                ResizeEdge::Right,
                CursorStyle::ResizeLeftRight,
                move |g| {
                    g.right_0()
                        .top(v_top_inset)
                        .bottom(v_bottom_inset)
                        .w(grip_size)
                },
            ))
        })
        .when(!(tiling.top || tiling.left), |d| {
            d.child(edge_grip(
                "window-grip-top-left",
                ResizeEdge::TopLeft,
                CursorStyle::ResizeUpLeftDownRight,
                |g| g.top_0().left_0().w(corner_size).h(corner_size),
            ))
        })
        .when(!(tiling.top || tiling.right), |d| {
            d.child(edge_grip(
                "window-grip-top-right",
                ResizeEdge::TopRight,
                CursorStyle::ResizeUpRightDownLeft,
                |g| g.top_0().right_0().w(corner_size).h(corner_size),
            ))
        })
        .when(!(tiling.bottom || tiling.left), |d| {
            d.child(edge_grip(
                "window-grip-bottom-left",
                ResizeEdge::BottomLeft,
                CursorStyle::ResizeUpRightDownLeft,
                |g| g.bottom_0().left_0().w(corner_size).h(corner_size),
            ))
        })
        .when(!(tiling.bottom || tiling.right), |d| {
            d.child(edge_grip(
                "window-grip-bottom-right",
                ResizeEdge::BottomRight,
                CursorStyle::ResizeUpLeftDownRight,
                |g| g.bottom_0().right_0().w(corner_size).h(corner_size),
            ))
        })
}

fn edge_grip(
    id: &'static str,
    edge: ResizeEdge,
    cursor: CursorStyle,
    position: impl FnOnce(Div) -> Div,
) -> impl IntoElement {
    position(div().absolute())
        .id(id)
        .occlude()
        .cursor(cursor)
        .on_mouse_down(MouseButton::Left, move |_, window, _| {
            window.start_window_resize(edge);
        })
}
