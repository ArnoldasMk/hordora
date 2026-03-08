# Protocol Roadmap

Current: 28 protocols implemented. Below are missing protocols available in smithay 0.7.0,
prioritized by user-facing impact.

## High Priority

These cause visible breakage or missing functionality for common workflows.

### idle-notify (ext-idle-notify-v1)
Notifies clients when the user goes idle. Required by swayidle for auto-lock and screen
dimming. (Note: idle-*inhibit* is already implemented — that prevents idle, this detects it.)

### single-pixel-buffer (wp-single-pixel-buffer-v1)
Lets clients create 1×1 solid-color buffers without SHM allocation. GTK4 uses this heavily
for backgrounds and separators. Trivial to add (~5 lines, no handler trait).

### xdg-dialog (xdg-dialog-v1)
Marks toplevel windows as modal dialogs. File pickers and confirmation dialogs should stay
on top of their parent and not get lost on the canvas.

### text-input + input-method + virtual-keyboard
Full IME stack for CJK input and on-screen keyboards. Three protocols that work together:
- `text-input` (zwp_text_input_v3) — app ↔ compositor text state
- `input-method` (zwp_input_method_v2) — compositor ↔ IME engine
- `virtual-keyboard` (zwp_virtual_keyboard_v1) — IME sends keystrokes

### xdg-foreign (xdg-foreign-v2)
Lets apps export/import surface handles across process boundaries. xdg-desktop-portal needs
this for screen sharing dialogs (the portal process references the requesting app's window).

## Medium Priority

Useful for specific hardware or app categories.

### alpha-modifier (wp-alpha-modifier-v1)
Per-surface opacity control. Some compositors use this for fade-in/out animations.

### tablet-manager (zwp_tablet_v2)
Stylus and drawing tablet input. Required for Wacom tablets, pen input in art apps.

### content-type (wp-content-type-v1)
Surface content type hints (none/photo/video/game). Enables adaptive sync and
compositor-side optimizations for games and video playback.

### security-context (wp-security-context-v1)
Identifies sandboxed clients (Flatpak, Snap). Allows per-sandbox permission policies.

### xwayland-keyboard-grab
Lets Xwayland grab compositor keybindings. Improves keyboard handling for X11 games
and apps that need raw key access.

## Low Priority

Niche use cases or already covered by existing protocols.

### xdg-toplevel-icon
Custom per-window icons (beyond app_id-based icon lookup).

### drm-syncobj
Explicit GPU synchronization for Vulkan compositing. Matters for multi-GPU setups.

### fifo + commit-timing
Advanced frame scheduling. FIFO guarantees in-order presentation; commit-timing
allows apps to target specific presentation times.

### drm-lease
Leases DRM resources to clients. Used for VR headsets (OpenXR).

### ext-data-control
Newer replacement for wlr-data-control. Not urgent since wlr version is widely supported.

### foreign-toplevel-list (ext-foreign-toplevel-list-v1)
Smithay's ext- replacement for zwlr-foreign-toplevel-management. Eventually migrate,
but wlr version works fine with existing taskbars.

### kde-decoration
KDE's decoration negotiation. xdg-decoration already covers this.

### xdg-system-bell
System bell notification (terminal beep → visual/audio feedback).

### xdg-toplevel-tag
Window tagging for grouping/filtering. No clients use this yet.
