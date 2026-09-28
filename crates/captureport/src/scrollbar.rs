use gpui::{
    AnyElement, Bounds, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Rgba, ScrollHandle, ScrollWheelEvent, Styled, UniformListScrollHandle, canvas,
    fill, point, px, size,
};
use std::{cell::Cell, rc::Rc};

const GUTTER: f32 = 14.;
const INSET: f32 = 3.;
const MIN_THUMB: f32 = 28.;

#[derive(Default)]
pub struct Scrollbars {
    pub sidebar: Scrollbar,
    pub bundle: Scrollbar,
    pub history: Scrollbar,
    pub recovery: Scrollbar,
    pub settings: Scrollbar,
    pub timezone: Scrollbar,
    pub media: ListScrollbar,
    pub preview: ListScrollbar,
}

#[derive(Default)]
pub struct ListScrollbar {
    pub list: UniformListScrollHandle,
    bar: Scrollbar,
}

impl ListScrollbar {
    pub fn element(&self, track: Rgba, thumb: Rgba, active: Rgba) -> AnyElement {
        self.bar.element_for(
            self.list.0.borrow().base_handle.clone(),
            track,
            thumb,
            active,
        )
    }
}

#[derive(Default)]
pub struct Scrollbar {
    pub handle: ScrollHandle,
    drag: Rc<Cell<Option<f32>>>,
    hovered: Rc<Cell<bool>>,
}

#[derive(Clone, Copy, Debug)]
struct Geometry {
    track_height: f32,
    thumb_height: f32,
    thumb_top: f32,
    max_offset: f32,
}

impl Geometry {
    fn new(height: f32, viewport: f32, max_offset: f32, offset: f32) -> Option<Self> {
        let track_height = (height - INSET * 2.).max(0.);
        if track_height <= 0. || viewport <= 0. || max_offset <= 0. {
            return None;
        }
        let thumb_height = (track_height * viewport / (viewport + max_offset))
            .max(MIN_THUMB)
            .min(track_height);
        let thumb_top = (track_height - thumb_height) * (-offset / max_offset).clamp(0., 1.);
        Some(Self {
            track_height,
            thumb_height,
            thumb_top,
            max_offset,
        })
    }

    fn offset_at(self, thumb_top: f32) -> f32 {
        let travel = self.track_height - self.thumb_height;
        if travel <= 0. {
            return 0.;
        }
        -self.max_offset * (thumb_top / travel).clamp(0., 1.)
    }
}

impl Scrollbar {
    pub fn element(&self, track: Rgba, thumb: Rgba, active: Rgba) -> AnyElement {
        self.element_for(self.handle.clone(), track, thumb, active)
    }

    fn element_for(
        &self,
        handle: ScrollHandle,
        track: Rgba,
        thumb: Rgba,
        active: Rgba,
    ) -> AnyElement {
        let layout_handle = handle.clone();
        let drag = self.drag.clone();
        let hovered = self.hovered.clone();
        canvas(
            move |bounds, window, _| {
                // Read after the sibling scroll surface has laid out, including
                // its first frame and changes to content or window size.
                Geometry::new(
                    bounds.size.height.into(),
                    layout_handle.bounds().size.height.into(),
                    layout_handle.max_offset().height.into(),
                    layout_handle.offset().y.into(),
                )
                .map(|geometry| {
                    (
                        geometry,
                        window.insert_hitbox(bounds, HitboxBehavior::Normal),
                    )
                })
            },
            move |bounds, state, window, _| {
                let Some((geometry, hitbox)) = state else {
                    drag.set(None);
                    return;
                };
                let track_bounds = Bounds::new(
                    bounds.origin + point(px(INSET), px(INSET)),
                    size(px(GUTTER - INSET * 2.), px(geometry.track_height)),
                );
                let thumb_bounds = Bounds::new(
                    track_bounds.origin + point(px(0.), px(geometry.thumb_top)),
                    size(track_bounds.size.width, px(geometry.thumb_height)),
                );
                window.paint_quad(fill(track_bounds, track).corner_radii(px(4.)));
                window.paint_quad(
                    fill(
                        thumb_bounds,
                        if drag.get().is_some() || hitbox.is_hovered(window) {
                            active
                        } else {
                            thumb
                        },
                    )
                    .corner_radii(px(4.)),
                );

                window.on_mouse_event({
                    let drag = drag.clone();
                    let handle = handle.clone();
                    let hitbox = hitbox.clone();
                    move |event: &MouseDownEvent, phase, window, cx| {
                        if !phase.capture()
                            || event.button != MouseButton::Left
                            || !hitbox.is_hovered(window)
                        {
                            return;
                        }
                        let pointer = f32::from(event.position.y - track_bounds.origin.y);
                        let grab = if thumb_bounds.contains(&event.position) {
                            pointer - geometry.thumb_top
                        } else {
                            geometry.thumb_height / 2.
                        };
                        drag.set(Some(grab));
                        let mut offset = handle.offset();
                        offset.y = px(geometry.offset_at(pointer - grab));
                        handle.set_offset(offset);
                        window.refresh();
                        cx.stop_propagation();
                    }
                });
                window.on_mouse_event({
                    let drag = drag.clone();
                    let handle = handle.clone();
                    let hitbox = hitbox.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if !phase.capture() {
                            return;
                        }
                        if let Some(grab) = drag.get() {
                            if !event.dragging() {
                                drag.set(None);
                            } else {
                                let pointer = f32::from(event.position.y - track_bounds.origin.y);
                                let mut offset = handle.offset();
                                offset.y = px(geometry.offset_at(pointer - grab));
                                handle.set_offset(offset);
                                cx.stop_propagation();
                            }
                            window.refresh();
                        }
                        if hovered.replace(hitbox.is_hovered(window)) != hitbox.is_hovered(window) {
                            window.refresh();
                        }
                    }
                });
                window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                    if phase.capture() && hitbox.is_hovered(window) {
                        let mut offset = handle.offset();
                        offset.y = (offset.y + event.delta.pixel_delta(window.line_height()).y)
                            .clamp(-px(geometry.max_offset), px(0.));
                        handle.set_offset(offset);
                        window.refresh();
                        cx.stop_propagation();
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase.capture() && event.button == MouseButton::Left && drag.take().is_some()
                    {
                        window.refresh();
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .w(px(GUTTER))
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_tracks_scroll_range_and_drag_reaches_both_ends() {
        let top = Geometry::new(206., 200., 800., 0.).unwrap();
        assert_eq!(top.thumb_height, 40.);
        assert_eq!(top.thumb_top, 0.);
        let middle = Geometry::new(206., 200., 800., -400.).unwrap();
        assert_eq!(middle.thumb_top, 80.);
        assert_eq!(middle.offset_at(middle.thumb_top), -400.);
        assert_eq!(middle.offset_at(-100.), 0.);
        assert_eq!(middle.offset_at(1000.), -800.);
    }

    #[test]
    fn no_overflow_and_tiny_viewports_have_safe_geometry() {
        assert!(Geometry::new(206., 200., 0., 0.).is_none());
        assert!(Geometry::new(0., 0., 100., 0.).is_none());
        let large = Geometry::new(206., 200., 100_000., 0.).unwrap();
        assert_eq!(large.thumb_height, MIN_THUMB);
        let tiny = Geometry::new(16., 10., 100., 0.).unwrap();
        assert_eq!(tiny.thumb_height, 10.);
        assert_eq!(tiny.offset_at(100.), 0.);
    }
}
