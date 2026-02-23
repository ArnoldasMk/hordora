use smithay::utils::{Logical, Point};

/// A position in screen-local coordinates (0,0 = top-left of the output).
#[derive(Debug, Clone, Copy)]
pub struct ScreenPos(pub Point<f64, Logical>);

/// A position in infinite canvas coordinates (absolute world position).
#[derive(Debug, Clone, Copy)]
pub struct CanvasPos(pub Point<f64, Logical>);

#[inline]
pub fn screen_to_canvas(screen: ScreenPos, camera: Point<f64, Logical>) -> CanvasPos {
    CanvasPos(screen.0 + camera)
}

#[inline]
pub fn canvas_to_screen(canvas: CanvasPos, camera: Point<f64, Logical>) -> ScreenPos {
    ScreenPos(canvas.0 - camera)
}

/// Scroll momentum physics: velocity decays by friction each frame.
/// Uses EMA (exponential moving average) for accumulation to smooth
/// out jittery trackpad deltas.
pub struct MomentumState {
    pub velocity: Point<f64, Logical>,
    pub friction: f64,
    /// Stop when |velocity|^2 < threshold_sq (default 0.25 = 0.5 px/frame)
    pub threshold_sq: f64,
    /// Frame number of the last scroll event. Prevents double-counting
    /// camera movement on frames where a scroll event fired.
    pub last_scroll_frame: u64,
}

impl MomentumState {
    pub fn new(friction: f64) -> Self {
        Self {
            velocity: Point::from((0.0, 0.0)),
            friction,
            threshold_sq: 0.25,
            last_scroll_frame: 0,
        }
    }

    /// EMA accumulate: velocity = velocity * 0.3 + delta * 0.7
    pub fn accumulate(&mut self, delta: Point<f64, Logical>, frame: u64) {
        self.velocity = Point::from((
            self.velocity.x * 0.3 + delta.x * 0.7,
            self.velocity.y * 0.3 + delta.y * 0.7,
        ));
        self.last_scroll_frame = frame;
    }

    /// Returns Some(delta) to apply, or None if skipped/finished.
    pub fn tick(&mut self, current_frame: u64) -> Option<Point<f64, Logical>> {
        // Skip on frames where a scroll event already moved the camera
        if self.last_scroll_frame == current_frame {
            return None;
        }
        if self.velocity.x.powi(2) + self.velocity.y.powi(2) < self.threshold_sq {
            self.velocity = Point::from((0.0, 0.0));
            return None;
        }
        let delta = self.velocity;
        self.velocity = Point::from((delta.x * self.friction, delta.y * self.friction));
        Some(delta)
    }

    pub fn stop(&mut self) {
        self.velocity = Point::from((0.0, 0.0));
    }
}
