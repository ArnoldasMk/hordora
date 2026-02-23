use smithay::{
    desktop::{PopupManager, Space, Window},
    input::{Seat, SeatState, keyboard::XkbConfig, pointer::CursorImageStatus},
    reexports::{
        calloop::{LoopHandle, LoopSignal},
        wayland_server::{
            Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{Logical, Point},
    wayland::output::OutputManagerState,
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        cursor_shape::CursorShapeManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::XdgShellState,
        shm::ShmState,
    },
};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::utils::Transform;

use driftwm::canvas::MomentumState;
use driftwm::config::Config;

/// Log an error result with context, discarding the Ok value.
#[inline]
pub fn log_err(context: &str, result: Result<impl Sized, impl std::fmt::Display>) {
    if let Err(e) = result {
        tracing::error!("{context}: {e}");
    }
}

/// Wrapper held by the calloop event loop — gives callbacks access
/// to both compositor state and the Wayland display.
pub struct CalloopData {
    pub state: DriftWm,
    pub display: Display<DriftWm>,
}

/// Central compositor state.
pub struct DriftWm {
    pub start_time: Instant,
    pub display_handle: DisplayHandle,
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub loop_signal: LoopSignal,

    // Desktop
    pub space: Space<Window>,
    pub popups: PopupManager,

    // Protocol state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<DriftWm>,
    pub data_device_state: DataDeviceState,

    // Input
    pub seat: Seat<DriftWm>,

    // Viewport / camera
    pub camera: Point<f64, Logical>,
    /// Timestamp of the last scroll-pan event. Used to keep panning sticky
    /// within a scroll gesture (150ms window) even if a window slides under.
    pub last_scroll_pan: Option<Instant>,
    /// Scroll momentum: velocity, friction, frame-based skip.
    pub momentum: MomentumState,
    /// Monotonic frame counter, incremented each render tick.
    pub frame_counter: u64,

    // Cursor
    pub cursor_status: CursorImageStatus,
    /// True while a compositor grab (pan/resize) owns the cursor icon.
    /// Blocks client cursor updates in `cursor_image()`.
    pub grab_cursor: bool,
    pub cursor_buffers: HashMap<String, (MemoryRenderBuffer, Point<i32, Logical>)>,

    // Protocols
    pub cursor_shape_state: CursorShapeManagerState,

    // Keybindings and settings
    pub config: Config,

    /// Surfaces awaiting their first buffer commit, to be centered once size is known.
    pub pending_center: HashSet<WlSurface>,
}

/// Per-client state stored by wayland-server for each connected client.
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

impl DriftWm {
    pub fn new(
        dh: DisplayHandle,
        loop_handle: LoopHandle<'static, CalloopData>,
        loop_signal: LoopSignal,
    ) -> Self {
        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);

        let cursor_shape_state = CursorShapeManagerState::new::<Self>(&dh);

        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "seat-0");
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to add keyboard");
        seat.add_pointer();

        let config = Config::default();
        Self {
            start_time: Instant::now(),
            display_handle: dh,
            loop_handle,
            loop_signal,
            space: Space::default(),
            popups: PopupManager::default(),
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
            camera: Point::from((0.0, 0.0)),
            last_scroll_pan: None,
            momentum: MomentumState::new(config.friction),
            frame_counter: 0,
            cursor_status: CursorImageStatus::default_named(),
            grab_cursor: false,
            cursor_buffers: HashMap::new(),
            cursor_shape_state,
            config,
            pending_center: HashSet::new(),
        }
    }

    /// Apply scroll momentum each frame. Skips frames where a scroll event
    /// already moved the camera (via frame counter). Otherwise decays velocity.
    pub fn apply_scroll_momentum(&mut self) {
        let Some(delta) = self.momentum.tick(self.frame_counter) else {
            return;
        };

        self.camera += delta;

        // Move pointer so cursor stays at the same screen position
        let pointer = self.seat.get_pointer().unwrap();
        let pos = pointer.current_location();
        let new_pos = pos + delta;
        let under = self.surface_under(new_pos);
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        pointer.motion(
            self,
            under,
            &smithay::input::pointer::MotionEvent {
                location: new_pos,
                serial,
                time: self.start_time.elapsed().as_millis() as u32,
            },
        );
        pointer.frame(self);
    }

    /// Sync each output's position to the current camera, so render_output
    /// automatically applies the canvas→screen transform.
    pub fn update_output_from_camera(&mut self) {
        let camera_i32 = self.camera.to_i32_round();
        for output in self.space.outputs().cloned().collect::<Vec<_>>() {
            self.space.map_output(&output, camera_i32);
        }
    }

    /// Load an xcursor image by name and cache the resulting MemoryRenderBuffer.
    /// Returns a reference to the cached (buffer, hotspot) pair.
    pub fn load_xcursor(
        &mut self,
        name: &str,
    ) -> Option<&(MemoryRenderBuffer, Point<i32, Logical>)> {
        if !self.cursor_buffers.contains_key(name) {
            let theme_name = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".into());
            let theme = xcursor::CursorTheme::load(&theme_name);
            let path = theme.load_icon(name)?;
            let data = std::fs::read(path).ok()?;
            let images = xcursor::parser::parse_xcursor(&data)?;

            // Pick the image closest to 24px (standard cursor size)
            let target_size = std::env::var("XCURSOR_SIZE")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(24);
            let image = images
                .iter()
                .min_by_key(|img| (img.size as i32 - target_size as i32).unsigned_abs())?;

            let buffer = MemoryRenderBuffer::from_slice(
                &image.pixels_rgba,
                Fourcc::Abgr8888,
                (image.width as i32, image.height as i32),
                1,
                Transform::Normal,
                None,
            );
            let hotspot = Point::from((image.xhot as i32, image.yhot as i32));
            self.cursor_buffers
                .insert(name.to_string(), (buffer, hotspot));
        }
        self.cursor_buffers.get(name)
    }
}
