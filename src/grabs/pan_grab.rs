use smithay::{
    input::{
        pointer::{
            ButtonEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
        },
        SeatHandler,
    },
    utils::{Logical, Point},
};

use driftwm::canvas::{CanvasPos, canvas_to_screen};
use crate::state::DriftWm;

/// Pointer grab that pans the viewport camera with momentum.
/// Triggered by Super+left-click or left-click on empty canvas.
/// Accumulates momentum during drag so the viewport coasts on release.
pub struct PanGrab {
    pub start_data: GrabStartData<DriftWm>,
    /// Screen-local position of the pointer last frame.
    /// Delta between consecutive screen positions drives the pan.
    pub last_screen_pos: Point<f64, Logical>,
}

impl PointerGrab<DriftWm> for PanGrab {
    fn motion(
        &mut self,
        data: &mut DriftWm,
        handle: &mut PointerInnerHandle<'_, DriftWm>,
        _focus: Option<(<DriftWm as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        // Recover screen position from canvas coords
        let current_screen_pos = canvas_to_screen(CanvasPos(event.location), data.camera).0;
        let screen_delta = current_screen_pos - self.last_screen_pos;

        // Dragging right → camera decreases → negate
        let camera_delta = Point::from((-screen_delta.x, -screen_delta.y));
        data.drift_pan(camera_delta);
        self.last_screen_pos = current_screen_pos;

        // Shift pointer canvas position so cursor stays at the same screen spot
        let adjusted = MotionEvent {
            location: event.location + camera_delta,
            serial: event.serial,
            time: event.time,
        };
        handle.motion(data, None, &adjusted);
    }

    fn button(
        &mut self,
        data: &mut DriftWm,
        handle: &mut PointerInnerHandle<'_, DriftWm>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            // Momentum is already primed from accumulated deltas — friction handles the coast
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn unset(&mut self, _data: &mut DriftWm) {}

    crate::grabs::forward_pointer_grab_methods!();
}
