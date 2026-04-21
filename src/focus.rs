//! FocusTarget newtype — the SeatHandler focus type for keyboard, pointer, and touch.
//!
//! Required because `PopupGrab` needs `KeyboardFocus: From<PopupKind>`, and we
//! can't impl `From<PopupKind> for WlSurface` (orphan rule). All input-target
//! Keyboard methods route through `X11Surface` when applicable (for ICCCM
//! `SetInputFocus` + `WM_TAKE_FOCUS`); pointer/touch delegate to `WlSurface`.

use std::borrow::Cow;

use smithay::{
    backend::input::KeyState,
    desktop::PopupKind,
    input::{
        Seat, SeatHandler,
        dnd::{DndFocus, Source},
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent,
            MotionEvent, PointerTarget, RelativeMotionEvent,
        },
        touch::{
            DownEvent as TouchDownEvent, MotionEvent as TouchMotionEvent, TouchTarget,
            UpEvent as TouchUpEvent,
        },
    },
    reexports::wayland_server::{DisplayHandle, protocol::wl_surface::WlSurface},
    utils::{IsAlive, Logical, Point, Serial},
    wayland::seat::WaylandFocus,
    xwayland::X11Surface,
};

use crate::state::Hordora;

// --- FocusTarget ---
// Newtype over WlSurface for use as SeatHandler focus types.
// Required because PopupGrab needs `KeyboardFocus: From<PopupKind>`,
// and we can't impl `From<PopupKind> for WlSurface` (orphan rule).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTarget(pub WlSurface);

impl From<PopupKind> for FocusTarget {
    fn from(popup: PopupKind) -> Self {
        FocusTarget(popup.wl_surface().clone())
    }
}

impl IsAlive for FocusTarget {
    fn alive(&self) -> bool {
        self.0.alive()
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        Some(Cow::Borrowed(&self.0))
    }
}

// Delegate all KeyboardTarget methods to the inner WlSurface using
// fully-qualified syntax to avoid clash with WlSurface::enter() protocol method.
impl KeyboardTarget<Hordora> for FocusTarget {
    fn enter(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Hordora>>::enter(&x11, seat, data, keys, serial);
        } else {
            <WlSurface as KeyboardTarget<Hordora>>::enter(&self.0, seat, data, keys, serial);
        }
    }

    fn leave(&self, seat: &Seat<Hordora>, data: &mut Hordora, serial: Serial) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Hordora>>::leave(&x11, seat, data, serial);
        } else {
            <WlSurface as KeyboardTarget<Hordora>>::leave(&self.0, seat, data, serial);
        }
    }

    fn key(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Hordora>>::key(&x11, seat, data, key, state, serial, time);
        } else {
            <WlSurface as KeyboardTarget<Hordora>>::key(&self.0, seat, data, key, state, serial, time);
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Hordora>>::modifiers(&x11, seat, data, modifiers, serial);
        } else {
            <WlSurface as KeyboardTarget<Hordora>>::modifiers(&self.0, seat, data, modifiers, serial);
        }
    }
}

impl PointerTarget<Hordora> for FocusTarget {
    fn enter(&self, seat: &Seat<Hordora>, data: &mut Hordora, event: &MotionEvent) {
        <WlSurface as PointerTarget<Hordora>>::enter(&self.0, seat, data, event);
    }

    fn motion(&self, seat: &Seat<Hordora>, data: &mut Hordora, event: &MotionEvent) {
        <WlSurface as PointerTarget<Hordora>>::motion(&self.0, seat, data, event);
    }

    fn relative_motion(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &RelativeMotionEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::relative_motion(&self.0, seat, data, event);
    }

    fn button(&self, seat: &Seat<Hordora>, data: &mut Hordora, event: &ButtonEvent) {
        <WlSurface as PointerTarget<Hordora>>::button(&self.0, seat, data, event);
    }

    fn axis(&self, seat: &Seat<Hordora>, data: &mut Hordora, frame: AxisFrame) {
        <WlSurface as PointerTarget<Hordora>>::axis(&self.0, seat, data, frame);
    }

    fn frame(&self, seat: &Seat<Hordora>, data: &mut Hordora) {
        <WlSurface as PointerTarget<Hordora>>::frame(&self.0, seat, data);
    }

    fn gesture_swipe_begin(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GestureSwipeBeginEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_swipe_begin(&self.0, seat, data, event);
    }

    fn gesture_swipe_update(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GestureSwipeUpdateEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_swipe_update(&self.0, seat, data, event);
    }

    fn gesture_swipe_end(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GestureSwipeEndEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_swipe_end(&self.0, seat, data, event);
    }

    fn gesture_pinch_begin(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GesturePinchBeginEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_pinch_begin(&self.0, seat, data, event);
    }

    fn gesture_pinch_update(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GesturePinchUpdateEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_pinch_update(&self.0, seat, data, event);
    }

    fn gesture_pinch_end(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GesturePinchEndEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_pinch_end(&self.0, seat, data, event);
    }

    fn gesture_hold_begin(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GestureHoldBeginEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_hold_begin(&self.0, seat, data, event);
    }

    fn gesture_hold_end(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &GestureHoldEndEvent,
    ) {
        <WlSurface as PointerTarget<Hordora>>::gesture_hold_end(&self.0, seat, data, event);
    }

    fn leave(&self, seat: &Seat<Hordora>, data: &mut Hordora, serial: Serial, time: u32) {
        <WlSurface as PointerTarget<Hordora>>::leave(&self.0, seat, data, serial, time);
    }
}

impl TouchTarget<Hordora> for FocusTarget {
    fn down(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &TouchDownEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Hordora>>::down(&self.0, seat, data, event, seq);
    }

    fn up(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &TouchUpEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Hordora>>::up(&self.0, seat, data, event, seq);
    }

    fn motion(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &TouchMotionEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Hordora>>::motion(&self.0, seat, data, event, seq);
    }

    fn frame(&self, seat: &Seat<Hordora>, data: &mut Hordora, seq: Serial) {
        <WlSurface as TouchTarget<Hordora>>::frame(&self.0, seat, data, seq);
    }

    fn cancel(&self, seat: &Seat<Hordora>, data: &mut Hordora, seq: Serial) {
        <WlSurface as TouchTarget<Hordora>>::cancel(&self.0, seat, data, seq);
    }

    fn shape(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &smithay::input::touch::ShapeEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Hordora>>::shape(&self.0, seat, data, event, seq);
    }

    fn orientation(
        &self,
        seat: &Seat<Hordora>,
        data: &mut Hordora,
        event: &smithay::input::touch::OrientationEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Hordora>>::orientation(&self.0, seat, data, event, seq);
    }
}

impl<D> DndFocus<D> for FocusTarget
where
    D: SeatHandler + smithay::wayland::selection::data_device::DataDeviceHandler + 'static,
    WlSurface: DndFocus<D>,
{
    type OfferData<S> = <WlSurface as DndFocus<D>>::OfferData<S> where S: Source;

    fn enter<S: Source>(
        &self,
        data: &mut D,
        dh: &DisplayHandle,
        source: std::sync::Arc<S>,
        seat: &Seat<D>,
        location: Point<f64, Logical>,
        serial: &Serial,
    ) -> Option<Self::OfferData<S>> {
        <WlSurface as DndFocus<D>>::enter(&self.0, data, dh, source, seat, location, serial)
    }

    fn motion<S: Source>(
        &self,
        data: &mut D,
        offer: Option<&mut Self::OfferData<S>>,
        seat: &Seat<D>,
        location: Point<f64, Logical>,
        time: u32,
    ) {
        <WlSurface as DndFocus<D>>::motion(&self.0, data, offer, seat, location, time)
    }

    fn leave<S: Source>(&self, data: &mut D, offer: Option<&mut Self::OfferData<S>>, seat: &Seat<D>) {
        <WlSurface as DndFocus<D>>::leave(&self.0, data, offer, seat)
    }

    fn drop<S: Source>(&self, data: &mut D, offer: Option<&mut Self::OfferData<S>>, seat: &Seat<D>) {
        <WlSurface as DndFocus<D>>::drop(&self.0, data, offer, seat)
    }
}
