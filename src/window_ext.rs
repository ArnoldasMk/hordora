use smithay::desktop::Window;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Rectangle, Size};
use smithay::wayland::shell::xdg::ToplevelSurface;

/// Set all four Tiled states on the toplevel's pending state. niri trick:
/// GTK and other toolkits read Tiled as "drop your shadow + rounded corners"
/// even if they ignore xdg-decoration. hordora draws uniform shadow + corners
/// on every window, so client chrome would just collide with ours.
///
/// Caveat: Tiled also affects how some clients pick a default size — SCTK-based
/// terminals like Alacritty interpret `Tiled + size=None` as "stay at current
/// tile size" rather than "pick preferred". That's why the exit-fit / exit-
/// fullscreen paths call `unset_tiled_states` before sending the restore configure.
pub fn set_tiled_states(toplevel: &ToplevelSurface) {
    use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
    toplevel.with_pending_state(|state| {
        state.states.set(xdg_toplevel::State::TiledLeft);
        state.states.set(xdg_toplevel::State::TiledRight);
        state.states.set(xdg_toplevel::State::TiledTop);
        state.states.set(xdg_toplevel::State::TiledBottom);
    });
}

/// Inverse of `set_tiled_states`.
pub fn unset_tiled_states(toplevel: &ToplevelSurface) {
    use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
    toplevel.with_pending_state(|state| {
        state.states.unset(xdg_toplevel::State::TiledLeft);
        state.states.unset(xdg_toplevel::State::TiledRight);
        state.states.unset(xdg_toplevel::State::TiledTop);
        state.states.unset(xdg_toplevel::State::TiledBottom);
    });
}

/// Convert a `DecorationMode` to the wire-protocol mode we advertise to clients.
/// Anything non-Client means SSD on the wire.
pub fn decoration_mode_to_wire(
    mode: &crate::config::DecorationMode,
) -> smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode {
    use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
    match mode {
        crate::config::DecorationMode::Client => Mode::ClientSide,
        _ => Mode::ServerSide,
    }
}

/// Extension trait on `Window` for operations that differ per window type
/// (Wayland vs X11). Avoids `.toplevel().unwrap()` which panics for X11 windows.
pub trait WindowExt {
    fn send_close(&self);
    fn app_id_or_class(&self) -> Option<String>;
    fn window_title(&self) -> Option<String>;
    /// Whether the window wants compositor-drawn (server-side) decorations.
    /// For X11: checks MOTIF hints. For Wayland: checks xdg-decoration mode.
    fn wants_ssd(&self) -> bool;
    fn enter_fullscreen_configure(&self, size: Size<i32, Logical>);
    fn exit_fullscreen_configure(&self, saved_size: Size<i32, Logical>);
    fn enter_fit_configure(&self, size: Size<i32, Logical>);
    fn exit_fit_configure(&self, saved_size: Size<i32, Logical>);
    /// The parent surface set via xdg_toplevel.set_parent (Wayland) or
    /// WM_TRANSIENT_FOR (X11). Returns None for X11 (follow-up).
    fn parent_surface(&self) -> Option<WlSurface>;
    /// Whether this is a modal dialog (xdg-dialog-v1). Non-modal parented
    /// windows (palettes, find dialogs) return false.
    fn is_modal(&self) -> bool;
    /// Whether this window is marked as a widget by an applied window rule.
    /// Widgets are persistent canvas furniture and should be excluded from
    /// most user-facing window operations (close, nudge, focus-cycle, etc).
    fn is_widget(&self) -> bool;
}

impl WindowExt for Window {
    fn send_close(&self) {
        if let Some(toplevel) = self.toplevel() {
            toplevel.send_close();
        } else if let Some(x11) = self.x11_surface() {
            x11.close().ok();
        }
    }

    fn app_id_or_class(&self) -> Option<String> {
        if let Some(toplevel) = self.toplevel() {
            smithay::wayland::compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                    .and_then(|d| d.lock().ok())
                    .and_then(|guard| guard.app_id.clone())
            })
        } else {
            self.x11_surface().map(|x11| x11.class())
        }
    }

    fn window_title(&self) -> Option<String> {
        if let Some(toplevel) = self.toplevel() {
            smithay::wayland::compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                    .and_then(|d| d.lock().ok())
                    .and_then(|guard| guard.title.clone())
            })
        } else {
            self.x11_surface().map(|x11| x11.title())
        }
    }

    fn wants_ssd(&self) -> bool {
        if let Some(_toplevel) = self.toplevel() {
            // Wayland: SSD is negotiated via xdg-decoration protocol,
            // handled in handlers/mod.rs (XdgDecorationHandler). Not checked here.
            false
        } else if let Some(x11) = self.x11_surface() {
            // is_decorated() = true means CLIENT draws decorations (no SSD needed)
            // is_decorated() = false means no MOTIF hints or app wants WM decorations
            !x11.is_decorated()
        } else {
            false
        }
    }

    fn enter_fullscreen_configure(&self, size: Size<i32, Logical>) {
        if let Some(toplevel) = self.toplevel() {
            use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
            toplevel.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Fullscreen);
                state.size = Some(size);
            });
            toplevel.send_configure();
        } else if let Some(x11) = self.x11_surface() {
            x11.set_fullscreen(true).ok();
            x11.configure(Rectangle::from_size(size)).ok();
        }
    }

    fn exit_fullscreen_configure(&self, saved_size: Size<i32, Logical>) {
        if let Some(toplevel) = self.toplevel() {
            use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
            // See exit_fit_configure: keep Tiled, send saved_size explicitly.
            toplevel.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Fullscreen);
                state.size = Some(saved_size);
            });
            toplevel.send_configure();
        } else if let Some(x11) = self.x11_surface() {
            x11.set_fullscreen(false).ok();
            x11.configure(Rectangle::new(x11.geometry().loc, saved_size))
                .ok();
        }
    }

    fn enter_fit_configure(&self, size: Size<i32, Logical>) {
        if let Some(toplevel) = self.toplevel() {
            use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
            toplevel.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Maximized);
                state.size = Some(size);
            });
            toplevel.send_configure();
        } else if let Some(x11) = self.x11_surface() {
            x11.set_maximized(true).ok();
            x11.configure(Rectangle::new(x11.geometry().loc, size)).ok();
        }
    }

    fn exit_fit_configure(&self, saved_size: Size<i32, Logical>) {
        if let Some(toplevel) = self.toplevel() {
            use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
            // Keep Tiled set (suppresses CSD on GTK/Chromium — so there's no
            // chrome to subtract from an explicit size, avoiding the historical
            // shrink spiral on repeated toggles) and send saved_size explicitly
            // (avoids SCTK reading "Tiled + None" as "stay at current size").
            // Matches niri's approach of always configuring with an explicit size.
            toplevel.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Maximized);
                state.size = Some(saved_size);
            });
            toplevel.send_configure();
        } else if let Some(x11) = self.x11_surface() {
            x11.set_maximized(false).ok();
            x11.configure(Rectangle::new(x11.geometry().loc, saved_size))
                .ok();
        }
    }

    fn parent_surface(&self) -> Option<WlSurface> {
        if let Some(toplevel) = self.toplevel() {
            smithay::wayland::compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                    .and_then(|d| d.lock().ok())
                    .and_then(|guard| guard.parent.clone())
            })
        } else {
            // X11 transient-for returns a window ID, not a WlSurface — follow-up
            None
        }
    }

    fn is_modal(&self) -> bool {
        if let Some(toplevel) = self.toplevel() {
            smithay::wayland::compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                    .and_then(|d| d.lock().ok())
                    .is_some_and(|guard| guard.dialog_hint == smithay::wayland::shell::xdg::dialog::ToplevelDialogHint::Modal)
            })
        } else {
            false
        }
    }

    fn is_widget(&self) -> bool {
        use smithay::wayland::seat::WaylandFocus;
        self.wl_surface()
            .as_deref()
            .and_then(crate::config::applied_rule)
            .is_some_and(|r| r.widget)
    }
}
