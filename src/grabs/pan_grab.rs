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

/// Pointer grab that pans the viewport camera.
/// Triggered by Super+right-click or left-click on empty canvas.
pub struct PanGrab {
    pub start_data: GrabStartData<DriftWm>,
    pub initial_camera: Point<f64, Logical>,
    /// Screen-local position at grab start (canvas_pos - camera at grab time).
    /// We track screen-relative delta to avoid the feedback loop where the
    /// changing camera feeds back into event.location → delta → camera.
    pub start_screen_pos: Point<f64, Logical>,
}

impl PointerGrab<DriftWm> for PanGrab {
    fn motion(
        &mut self,
        data: &mut DriftWm,
        handle: &mut PointerInnerHandle<'_, DriftWm>,
        _focus: Option<(<DriftWm as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        // event.location is in canvas coords (screen + current_camera).
        // Recover screen-local pos by subtracting current camera.
        let current_screen_pos = canvas_to_screen(CanvasPos(event.location), data.camera).0;
        let screen_delta = current_screen_pos - self.start_screen_pos;
        data.camera = self.initial_camera - screen_delta;
        data.update_output_from_camera();
        handle.motion(data, None, event);
    }

    fn button(
        &mut self,
        data: &mut DriftWm,
        handle: &mut PointerInnerHandle<'_, DriftWm>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn unset(&mut self, _data: &mut DriftWm) {}

    crate::grabs::forward_pointer_grab_methods!();
}
