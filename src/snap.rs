/// Bounding rectangle of a window in canvas coordinates, used for edge snap detection.
pub struct SnapRect {
    pub x_low: f64,
    pub x_high: f64,
    pub y_low: f64,
    pub y_high: f64,
}

/// Parameters for snap candidate search along one axis.
pub struct SnapParams<'a> {
    pub extent: f64,
    pub perp_low: f64,
    pub perp_high: f64,
    pub horizontal: bool,
    pub others: &'a [SnapRect],
    pub gap: f64,
    pub threshold: f64,
    pub break_force: f64,
}

/// Per-axis snap state: tracks the snapped coordinate and the natural position
/// at the moment of engagement (used for directional break detection).
pub struct AxisSnap {
    pub snapped_pos: f64,
    pub natural_at_engage: f64,
}

/// Snap state for both axes plus cooldown after breaking a snap.
#[derive(Default)]
pub struct SnapState {
    pub x: Option<AxisSnap>,
    pub y: Option<AxisSnap>,
    pub cooldown_x: Option<f64>,
    pub cooldown_y: Option<f64>,
}

/// Find the best snap candidate along one axis, filtering out windows that
/// don't overlap on the perpendicular axis (within `threshold` tolerance).
///
/// Returns `Some((snapped_origin, abs_distance))` for the closest candidate
/// within `threshold`, or `None`.
pub fn find_snap_candidate(natural_edge_low: f64, p: &SnapParams<'_>) -> Option<(f64, f64)> {
    let natural_edge_high = natural_edge_low + p.extent;
    let mut best: Option<(f64, f64)> = None;

    for other in p.others {
        let (other_low, other_high, other_perp_low, other_perp_high) = if p.horizontal {
            (other.x_low, other.x_high, other.y_low, other.y_high)
        } else {
            (other.y_low, other.y_high, other.x_low, other.x_high)
        };

        // Skip windows with no perpendicular overlap (tolerance = threshold)
        if p.perp_high + p.threshold <= other_perp_low
            || other_perp_high + p.threshold <= p.perp_low
        {
            continue;
        }

        // dragged right edge → other left edge
        let snap_origin = other_low - p.gap - p.extent;
        let dist = (natural_edge_high - other_low).abs();
        if dist < p.threshold && best.is_none_or(|(_, bd)| dist < bd) {
            best = Some((snap_origin, dist));
        }

        // dragged left edge → other right edge
        let snap_origin = other_high + p.gap;
        let dist = (natural_edge_low - other_high).abs();
        if dist < p.threshold && best.is_none_or(|(_, bd)| dist < bd) {
            best = Some((snap_origin, dist));
        }
    }

    best
}

/// Update snap state for a single axis. Returns the final position for that axis.
pub fn update_axis(
    snap: &mut Option<AxisSnap>,
    cooldown: &mut Option<f64>,
    natural_pos: f64,
    p: &SnapParams<'_>,
) -> f64 {
    if let Some(ref s) = *snap {
        // Directional break: retreat past engagement point OR overshoot past snap
        let (retreat, overshoot) = if s.snapped_pos > s.natural_at_engage {
            (s.natural_at_engage - natural_pos, natural_pos - s.snapped_pos)
        } else {
            (natural_pos - s.natural_at_engage, s.snapped_pos - natural_pos)
        };
        if retreat >= p.break_force || overshoot >= p.break_force {
            *cooldown = Some(s.snapped_pos);
            *snap = None;
            natural_pos
        } else {
            s.snapped_pos
        }
    } else {
        // Clear cooldown when natural position leaves threshold of cooldown coord
        if let Some(cd) = *cooldown
            && (natural_pos - cd).abs() > p.threshold
        {
            *cooldown = None;
        }

        // Try to find a new snap candidate (skip if on cooldown)
        if cooldown.is_none()
            && let Some((snapped_pos, _)) = find_snap_candidate(natural_pos, p)
        {
            *snap = Some(AxisSnap {
                snapped_pos,
                natural_at_engage: natural_pos,
            });
            return snapped_pos;
        }

        natural_pos
    }
}
