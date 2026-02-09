use serde::{Deserialize, Serialize};

/// A monitor's position and logical (scaled) dimensions in layout space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutMonitor {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl LayoutMonitor {
    pub fn right(&self) -> i32 { self.x + self.w }
    pub fn bottom(&self) -> i32 { self.y + self.h }

    /// Vertical overlap range between two monitors (shared vertical edge test).
    /// Returns Some((start, end)) if they overlap vertically, None if they don't.
    fn vertical_overlap(&self, other: &LayoutMonitor) -> Option<(i32, i32)> {
        let start = self.y.max(other.y);
        let end = self.bottom().min(other.bottom());
        if end > start { Some((start, end)) } else { None }
    }

    /// Horizontal overlap range between two monitors (shared horizontal edge test).
    fn horizontal_overlap(&self, other: &LayoutMonitor) -> Option<(i32, i32)> {
        let start = self.x.max(other.x);
        let end = self.right().min(other.right());
        if end > start { Some((start, end)) } else { None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Describes the relationship between two monitors sharing an edge.
#[derive(Debug, PartialEq)]
pub enum SharedEdge {
    /// They share a vertical edge (monitors are side by side).
    /// The i32 is the x coordinate of the shared edge.
    Vertical(i32),
    /// They share a horizontal edge (monitors are stacked).
    /// The i32 is the y coordinate of the shared edge.
    Horizontal(i32),
}

/// Find which edge two monitors share, if any.
/// They must be touching (edges exactly meeting) AND have overlap on the perpendicular axis.
pub fn shared_edge(a: &LayoutMonitor, b: &LayoutMonitor) -> Option<SharedEdge> {
    // Check vertical edge (side by side)
    if a.right() == b.x && a.vertical_overlap(b).is_some() {
        return Some(SharedEdge::Vertical(a.right()));
    }
    if b.right() == a.x && a.vertical_overlap(b).is_some() {
        return Some(SharedEdge::Vertical(a.x));
    }
    // Check horizontal edge (stacked)
    if a.bottom() == b.y && a.horizontal_overlap(b).is_some() {
        return Some(SharedEdge::Horizontal(a.bottom()));
    }
    if b.bottom() == a.y && a.horizontal_overlap(b).is_some() {
        return Some(SharedEdge::Horizontal(a.y));
    }
    None
}

/// Find the neighbor of `selected` in the given direction.
/// Returns the index of the neighbor.
pub fn find_neighbor(monitors: &[LayoutMonitor], selected: usize, dir: Direction) -> Option<usize> {
    let sel = &monitors[selected];
    let mut best: Option<(usize, i32)> = None;

    for (i, m) in monitors.iter().enumerate() {
        if i == selected { continue; }

        let qualifies = match dir {
            Direction::Left => m.right() <= sel.x && sel.vertical_overlap(m).is_some(),
            Direction::Right => m.x >= sel.right() && sel.vertical_overlap(m).is_some(),
            Direction::Up => m.bottom() <= sel.y && sel.horizontal_overlap(m).is_some(),
            Direction::Down => m.y >= sel.bottom() && sel.horizontal_overlap(m).is_some(),
        };

        if qualifies {
            let dist = match dir {
                Direction::Left => sel.x - m.right(),
                Direction::Right => m.x - sel.right(),
                Direction::Up => sel.y - m.bottom(),
                Direction::Down => m.y - sel.bottom(),
            };
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((i, dist));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Find the nearest neighbor in a direction, ignoring edge sharing (for snap operations).
pub fn find_nearest(monitors: &[LayoutMonitor], selected: usize, dir: Direction) -> Option<usize> {
    let sel = &monitors[selected];
    let mut best: Option<(usize, i32)> = None;

    for (i, m) in monitors.iter().enumerate() {
        if i == selected { continue; }

        let dist = match dir {
            Direction::Left => sel.x - m.right(),
            Direction::Right => m.x - sel.right(),
            Direction::Up => sel.y - m.bottom(),
            Direction::Down => m.y - sel.bottom(),
        };

        // Must be in the right direction (positive distance)
        if dist < 0 { continue; }

        if best.is_none() || dist < best.unwrap().1 {
            best = Some((i, dist));
        }
    }

    // If no monitor is strictly in that direction, find closest by center
    if best.is_none() {
        let sel_cx = sel.x + sel.w / 2;
        let sel_cy = sel.y + sel.h / 2;
        let mut closest: Option<(usize, i32)> = None;
        for (i, m) in monitors.iter().enumerate() {
            if i == selected { continue; }
            let cx = m.x + m.w / 2;
            let cy = m.y + m.h / 2;
            let is_correct_side = match dir {
                Direction::Left => cx < sel_cx,
                Direction::Right => cx > sel_cx,
                Direction::Up => cy < sel_cy,
                Direction::Down => cy > sel_cy,
            };
            if !is_correct_side { continue; }
            let d = (cx - sel_cx).abs() + (cy - sel_cy).abs();
            if closest.is_none() || d < closest.unwrap().1 {
                closest = Some((i, d));
            }
        }
        return closest.map(|(i, _)| i);
    }

    best.map(|(i, _)| i)
}

/// Move the selected monitor in the given direction.
/// - Perpendicular to shared edge: swap positions
/// - Parallel to shared edge: slide along it
/// - If no neighbor with shared edge: try snap
pub fn move_monitor(monitors: &mut Vec<LayoutMonitor>, selected: usize, dir: Direction, step: i32) {
    if monitors.len() <= 1 { return; }

    // Find neighbors that share an edge with the selected monitor.
    // Separate into: perpendicular neighbor in the pressed direction, and parallel neighbors.
    let sel = &monitors[selected];
    let mut perp_neighbor: Option<(usize, SharedEdge)> = None;
    let mut parallel_neighbor: Option<(usize, SharedEdge)> = None;

    for (i, m) in monitors.iter().enumerate() {
        if i == selected { continue; }
        if let Some(edge) = shared_edge(sel, m) {
            let is_perp = match (&edge, dir) {
                (SharedEdge::Vertical(_), Direction::Left | Direction::Right) => true,
                (SharedEdge::Horizontal(_), Direction::Up | Direction::Down) => true,
                _ => false,
            };

            if is_perp {
                // Only accept if the neighbor is actually in the direction we're pressing
                let in_dir = match (&edge, dir) {
                    (SharedEdge::Vertical(ex), Direction::Left) => *ex == sel.x,
                    (SharedEdge::Vertical(ex), Direction::Right) => *ex == sel.right(),
                    (SharedEdge::Horizontal(ey), Direction::Up) => *ey == sel.y,
                    (SharedEdge::Horizontal(ey), Direction::Down) => *ey == sel.bottom(),
                    _ => false,
                };
                if in_dir && perp_neighbor.is_none() {
                    perp_neighbor = Some((i, edge));
                }
            } else if parallel_neighbor.is_none() {
                parallel_neighbor = Some((i, edge));
            }
        }
    }

    if let Some((ni, _)) = perp_neighbor {
        // Perpendicular neighbor in the right direction — swap
        swap_monitors(monitors, selected, ni);
    } else if let Some((ni, _)) = parallel_neighbor {
        // No perpendicular neighbor in that direction — slide along a parallel edge
        slide_monitor(monitors, selected, ni, dir, step);
    }
    // If neither: monitor is already at the edge in that direction — do nothing
}

/// Swap two monitors' positions. Each takes the other's position,
/// adjusted so they remain touching. Also shifts other monitors
/// to fill gaps caused by different sizes.
pub fn swap_monitors(monitors: &mut Vec<LayoutMonitor>, a: usize, b: usize) {
    let a_x = monitors[a].x;
    let a_y = monitors[a].y;
    let b_x = monitors[b].x;
    let b_y = monitors[b].y;
    let a_w = monitors[a].w;
    let b_w = monitors[b].w;
    let a_h = monitors[a].h;
    let b_h = monitors[b].h;

    if let Some(edge) = shared_edge(&monitors[a], &monitors[b]) {
        match edge {
            SharedEdge::Vertical(_) => {
                let left = a_x.min(b_x);
                let size_diff = b_w - a_w;
                if a_x < b_x {
                    // a was left, b was right → swap
                    monitors[b].x = left;
                    monitors[a].x = left + b_w;
                    // Shift all monitors to the right of the old b position
                    let old_b_right = b_x + b_w;
                    for i in 0..monitors.len() {
                        if i == a || i == b { continue; }
                        if monitors[i].x >= old_b_right {
                            monitors[i].x -= size_diff;
                        } else if monitors[i].x >= a_x + a_w && monitors[i].x < b_x {
                            monitors[i].x += size_diff;
                        }
                    }
                } else {
                    monitors[a].x = left;
                    monitors[b].x = left + a_w;
                    let old_a_right = a_x + a_w;
                    for i in 0..monitors.len() {
                        if i == a || i == b { continue; }
                        if monitors[i].x >= old_a_right {
                            monitors[i].x += size_diff;
                        } else if monitors[i].x >= b_x + b_w && monitors[i].x < a_x {
                            monitors[i].x -= size_diff;
                        }
                    }
                }
            }
            SharedEdge::Horizontal(_) => {
                let top = a_y.min(b_y);
                let size_diff = b_h - a_h;
                if a_y < b_y {
                    monitors[b].y = top;
                    monitors[a].y = top + b_h;
                    let old_b_bottom = b_y + b_h;
                    for i in 0..monitors.len() {
                        if i == a || i == b { continue; }
                        if monitors[i].y >= old_b_bottom {
                            monitors[i].y -= size_diff;
                        }
                    }
                } else {
                    monitors[a].y = top;
                    monitors[b].y = top + a_h;
                    let old_a_bottom = a_y + a_h;
                    for i in 0..monitors.len() {
                        if i == a || i == b { continue; }
                        if monitors[i].y >= old_a_bottom {
                            monitors[i].y += size_diff;
                        }
                    }
                }
            }
        }
    }
}

/// Slide a monitor along a shared edge.
/// If the slide causes them to lose their shared edge, snap to stacked/side-by-side.
pub fn slide_monitor(monitors: &mut Vec<LayoutMonitor>, selected: usize, neighbor: usize, dir: Direction, step: i32) {
    let delta = match dir {
        Direction::Up => -step,
        Direction::Down => step,
        Direction::Left => -step,
        Direction::Right => step,
    };

    // Apply the slide
    match dir {
        Direction::Up | Direction::Down => {
            monitors[selected].y += delta;
        }
        Direction::Left | Direction::Right => {
            monitors[selected].x += delta;
        }
    }

    // Check if they still share an edge
    let still_touching = shared_edge(&monitors[selected], &monitors[neighbor]).is_some();

    if !still_touching {
        // Copy neighbor values to avoid borrow issues
        let nbr_x = monitors[neighbor].x;
        let nbr_y = monitors[neighbor].y;
        let nbr_w = monitors[neighbor].w;
        let nbr_h = monitors[neighbor].h;
        let nbr_right = nbr_x + nbr_w;
        let nbr_bottom = nbr_y + nbr_h;
        let nbr_cy = nbr_y + nbr_h / 2;
        let nbr_cx = nbr_x + nbr_w / 2;

        let sel_cy = monitors[selected].y + monitors[selected].h / 2;
        let sel_cx = monitors[selected].x + monitors[selected].w / 2;

        match dir {
            Direction::Up | Direction::Down => {
                if sel_cy < nbr_cy {
                    monitors[selected].y = nbr_y - monitors[selected].h;
                    monitors[selected].x = nbr_x;
                } else {
                    monitors[selected].y = nbr_bottom;
                    monitors[selected].x = nbr_x;
                }
            }
            Direction::Left | Direction::Right => {
                if sel_cx < nbr_cx {
                    monitors[selected].x = nbr_x - monitors[selected].w;
                    monitors[selected].y = nbr_y;
                } else {
                    monitors[selected].x = nbr_right;
                    monitors[selected].y = nbr_y;
                }
            }
        }
    }
}

/// Snap `selected` to a specific side of `target`.
pub fn snap_to_side(monitors: &mut Vec<LayoutMonitor>, selected: usize, target: usize, dir: Direction) {
    let tw = monitors[target].w;
    let th = monitors[target].h;
    let tx = monitors[target].x;
    let ty = monitors[target].y;

    match dir {
        Direction::Left => {
            monitors[selected].x = tx - monitors[selected].w;
            monitors[selected].y = ty;
        }
        Direction::Right => {
            monitors[selected].x = tx + tw;
            monitors[selected].y = ty;
        }
        Direction::Up => {
            monitors[selected].x = tx;
            monitors[selected].y = ty - monitors[selected].h;
        }
        Direction::Down => {
            monitors[selected].x = tx;
            monitors[selected].y = ty + th;
        }
    }
}

/// Snap `selected` to the far side of the entire layout in the given direction.
/// E.g. Shift+L moves the monitor to the rightmost position, Shift+H to the leftmost.
pub fn snap_to_far_side(monitors: &mut Vec<LayoutMonitor>, selected: usize, dir: Direction) {
    if monitors.len() <= 1 { return; }

    // Remove selected from consideration to find the remaining layout bounds
    let sel_w = monitors[selected].w;
    let sel_h = monitors[selected].h;

    // Find the extreme edges of all OTHER monitors
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut ref_y = 0; // y of the monitor closest to the target edge
    let mut ref_x = 0;

    for (i, m) in monitors.iter().enumerate() {
        if i == selected { continue; }
        min_x = min_x.min(m.x);
        max_x = max_x.max(m.right());
        min_y = min_y.min(m.y);
        max_y = max_y.max(m.bottom());
    }

    // Find the monitor at the far edge to align y/x with
    match dir {
        Direction::Left => {
            // Find leftmost other monitor for y alignment
            for (i, m) in monitors.iter().enumerate() {
                if i == selected { continue; }
                if m.x == min_x { ref_y = m.y; break; }
            }
            monitors[selected].x = min_x - sel_w;
            monitors[selected].y = ref_y;
        }
        Direction::Right => {
            for (i, m) in monitors.iter().enumerate() {
                if i == selected { continue; }
                if m.right() == max_x { ref_y = m.y; break; }
            }
            monitors[selected].x = max_x;
            monitors[selected].y = ref_y;
        }
        Direction::Up => {
            for (i, m) in monitors.iter().enumerate() {
                if i == selected { continue; }
                if m.y == min_y { ref_x = m.x; break; }
            }
            monitors[selected].y = min_y - sel_h;
            monitors[selected].x = ref_x;
        }
        Direction::Down => {
            for (i, m) in monitors.iter().enumerate() {
                if i == selected { continue; }
                if m.bottom() == max_y { ref_x = m.x; break; }
            }
            monitors[selected].y = max_y;
            monitors[selected].x = ref_x;
        }
    }
}

/// Validate that all monitors form a connected graph (each shares an edge with at least one other).
pub fn is_layout_connected(monitors: &[LayoutMonitor]) -> bool {
    if monitors.len() <= 1 { return true; }

    let mut visited = vec![false; monitors.len()];
    let mut stack = vec![0usize];
    visited[0] = true;

    while let Some(current) = stack.pop() {
        for (i, m) in monitors.iter().enumerate() {
            if visited[i] { continue; }
            if shared_edge(&monitors[current], m).is_some() {
                visited[i] = true;
                stack.push(i);
            }
        }
    }

    visited.iter().all(|&v| v)
}

/// Recalculate positions: place all monitors left to right with no gaps.
/// Preserves y offsets.
pub fn recalculate_horizontal(monitors: &mut Vec<LayoutMonitor>) {
    monitors.sort_by_key(|m| m.x);
    let mut x = 0;
    for m in monitors.iter_mut() {
        m.x = x;
        x += m.w;
    }
}

/// Ensure all monitors are connected to the layout by snapping any floating ones
/// to the nearest monitor. Call after every move operation.
pub fn auto_snap_all(monitors: &mut Vec<LayoutMonitor>) {
    if monitors.len() <= 1 { return; }

    // Iterate until stable (max iterations = len to prevent infinite loops)
    for _ in 0..monitors.len() {
        let mut any_fixed = false;
        for i in 0..monitors.len() {
            // Check if this monitor touches at least one other
            let touches_any = (0..monitors.len())
                .any(|j| j != i && shared_edge(&monitors[i], &monitors[j]).is_some());

            if !touches_any {
                // Find nearest monitor by center distance and snap to closest edge
                let cx = monitors[i].x + monitors[i].w / 2;
                let cy = monitors[i].y + monitors[i].h / 2;

                let nearest = (0..monitors.len())
                    .filter(|&j| j != i)
                    .min_by_key(|&j| {
                        let ocx = monitors[j].x + monitors[j].w / 2;
                        let ocy = monitors[j].y + monitors[j].h / 2;
                        (cx - ocx).abs() + (cy - ocy).abs()
                    });

                if let Some(ni) = nearest {
                    // Determine which side to snap to based on relative position
                    let nx = monitors[ni].x + monitors[ni].w / 2;
                    let ny = monitors[ni].y + monitors[ni].h / 2;
                    let dx = cx - nx;
                    let dy = cy - ny;

                    // Copy target values
                    let tx = monitors[ni].x;
                    let ty = monitors[ni].y;
                    let tw = monitors[ni].w;
                    let th = monitors[ni].h;

                    if dx.abs() > dy.abs() {
                        // Snap horizontally
                        if dx > 0 {
                            monitors[i].x = tx + tw;
                        } else {
                            monitors[i].x = tx - monitors[i].w;
                        }
                        // Align y to maximize overlap
                        monitors[i].y = ty;
                    } else {
                        // Snap vertically
                        if dy > 0 {
                            monitors[i].y = ty + th;
                        } else {
                            monitors[i].y = ty - monitors[i].h;
                        }
                        monitors[i].x = tx;
                    }
                    any_fixed = true;
                }
            }
        }
        if !any_fixed { break; }
    }
}

/// Push `moved` monitor out of any overlapping monitors.
/// Picks the push direction that places the monitor closest to `orig_x, orig_y`
/// (its position before the operation), so it doesn't overshoot to the wrong side.
pub fn resolve_overlaps(monitors: &mut Vec<LayoutMonitor>, moved: usize, orig_x: i32, orig_y: i32) {
    for _ in 0..monitors.len() {
        let mut best_push: Option<(i32, i32, i64)> = None; // (dx, dy, dist_to_origin)

        for j in 0..monitors.len() {
            if j == moved { continue; }

            let h_overlap = monitors[moved].horizontal_overlap(&monitors[j]);
            let v_overlap = monitors[moved].vertical_overlap(&monitors[j]);

            if h_overlap.is_none() || v_overlap.is_none() {
                continue;
            }

            let push_left = monitors[j].x - monitors[moved].right();
            let push_right = monitors[j].right() - monitors[moved].x;
            let push_up = monitors[j].y - monitors[moved].bottom();
            let push_down = monitors[j].bottom() - monitors[moved].y;

            let candidates = [
                (push_left, 0),
                (push_right, 0),
                (0, push_up),
                (0, push_down),
            ];

            // Score each candidate by how close the result would be to the original position
            let best_for_j = candidates.iter()
                .map(|&(dx, dy)| {
                    let rx = (monitors[moved].x + dx - orig_x) as i64;
                    let ry = (monitors[moved].y + dy - orig_y) as i64;
                    (dx, dy, rx * rx + ry * ry)
                })
                .min_by_key(|c| c.2)
                .unwrap();

            if best_push.is_none() || best_for_j.2 < best_push.unwrap().2 {
                best_push = Some(best_for_j);
            }
        }

        match best_push {
            Some((dx, dy, _)) => {
                monitors[moved].x += dx;
                monitors[moved].y += dy;
            }
            None => break,
        }
    }
}

/// Normalize layout so the top-left monitor is at (0, 0).
pub fn normalize(monitors: &mut Vec<LayoutMonitor>) {
    if monitors.is_empty() { return; }
    let min_x = monitors.iter().map(|m| m.x).min().unwrap();
    let min_y = monitors.iter().map(|m| m.y).min().unwrap();
    for m in monitors.iter_mut() {
        m.x -= min_x;
        m.y -= min_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_side_by_side() -> Vec<LayoutMonitor> {
        vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "C".into(), x: 3840, y: 0, w: 1920, h: 1080 },
        ]
    }

    fn two_stacked() -> Vec<LayoutMonitor> {
        vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 0, y: 1080, w: 1920, h: 1080 },
        ]
    }

    fn two_side_by_side_different_heights() -> Vec<LayoutMonitor> {
        vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 2560, h: 1440 },
        ]
    }

    // --- shared_edge tests ---

    #[test]
    fn test_shared_edge_side_by_side() {
        let a = LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 };
        let b = LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 1920, h: 1080 };
        assert_eq!(shared_edge(&a, &b), Some(SharedEdge::Vertical(1920)));
        assert_eq!(shared_edge(&b, &a), Some(SharedEdge::Vertical(1920)));
    }

    #[test]
    fn test_shared_edge_stacked() {
        let a = LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 };
        let b = LayoutMonitor { id: "B".into(), x: 0, y: 1080, w: 1920, h: 1080 };
        assert_eq!(shared_edge(&a, &b), Some(SharedEdge::Horizontal(1080)));
    }

    #[test]
    fn test_shared_edge_offset_but_overlapping() {
        let a = LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 };
        let b = LayoutMonitor { id: "B".into(), x: 1920, y: 500, w: 1920, h: 1080 };
        // They share a vertical edge at x=1920, and have vertical overlap (500..1080)
        assert_eq!(shared_edge(&a, &b), Some(SharedEdge::Vertical(1920)));
    }

    #[test]
    fn test_shared_edge_no_overlap() {
        let a = LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 };
        let b = LayoutMonitor { id: "B".into(), x: 1920, y: 1080, w: 1920, h: 1080 };
        // They touch at a single corner point (1920, 1080) — no edge overlap
        assert_eq!(shared_edge(&a, &b), None);
    }

    #[test]
    fn test_shared_edge_gap() {
        let a = LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 };
        let b = LayoutMonitor { id: "B".into(), x: 1921, y: 0, w: 1920, h: 1080 };
        assert_eq!(shared_edge(&a, &b), None);
    }

    // --- swap tests ---

    #[test]
    fn test_swap_side_by_side() {
        let mut m = three_side_by_side();
        swap_monitors(&mut m, 0, 1);
        // A and B should swap. B is now left, A is now right
        assert_eq!(m[1].x, 0); // B moved to 0
        assert_eq!(m[0].x, 1920); // A moved to 1920
        assert_eq!(m[0].id, "A");
        assert_eq!(m[1].id, "B");
    }

    #[test]
    fn test_swap_stacked() {
        let mut m = two_stacked();
        swap_monitors(&mut m, 0, 1);
        assert_eq!(m[1].y, 0); // B is now on top
        assert_eq!(m[0].y, 1080); // A is below
    }

    // --- slide tests ---

    #[test]
    fn test_slide_vertical_along_shared_vertical_edge() {
        let mut m = two_side_by_side_different_heights();
        // A and B share a vertical edge. Sliding A down (parallel to edge).
        slide_monitor(&mut m, 0, 1, Direction::Down, 100);
        assert_eq!(m[0].y, 100);
        assert_eq!(m[0].x, 0); // x unchanged
        // Still sharing edge
        assert!(shared_edge(&m[0], &m[1]).is_some());
    }

    #[test]
    fn test_slide_past_edge_snaps() {
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 1920, h: 1080 },
        ];
        // Slide A down by more than B's height — should snap below B
        slide_monitor(&mut m, 0, 1, Direction::Down, 1200);
        // A should now be below B with left edges aligned
        assert_eq!(m[0].y, 1080);
        assert_eq!(m[0].x, m[1].x);
    }

    // --- move_monitor tests ---

    #[test]
    fn test_move_perpendicular_swaps() {
        let mut m = three_side_by_side();
        // Move B (index 1) left — perpendicular to the vertical edge it shares with A
        move_monitor(&mut m, 1, Direction::Left, 10);
        // B and A should swap
        assert_eq!(m[1].x, 0); // B
        assert_eq!(m[0].x, 1920); // A
    }

    #[test]
    fn test_move_parallel_slides() {
        let mut m = three_side_by_side();
        // Move B (index 1) down — parallel to the vertical edges it shares
        move_monitor(&mut m, 1, Direction::Down, 50);
        assert_eq!(m[1].y, 50);
        // B still shares edge with A and C
        assert!(shared_edge(&m[0], &m[1]).is_some());
        assert!(shared_edge(&m[1], &m[2]).is_some());
    }

    #[test]
    fn test_move_leftmost_left_is_noop() {
        let mut m = three_side_by_side();
        let before = m.clone();
        // A is already the leftmost — pressing left should do nothing
        move_monitor(&mut m, 0, Direction::Left, 10);
        assert_eq!(m, before, "Leftmost monitor should not move when pressing left");
    }

    #[test]
    fn test_move_rightmost_right_is_noop() {
        let mut m = three_side_by_side();
        let before = m.clone();
        move_monitor(&mut m, 2, Direction::Right, 10);
        assert_eq!(m, before, "Rightmost monitor should not move when pressing right");
    }

    #[test]
    fn test_move_stacked_topmost_up_is_noop() {
        // Two stacked — pressing up on the top one should do nothing
        let mut m = two_stacked();
        let before = m.clone();
        move_monitor(&mut m, 0, Direction::Up, 10);
        assert_eq!(m, before, "Topmost stacked monitor should not move when pressing up");
    }

    #[test]
    fn test_move_single_monitor_noop() {
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
        ];
        move_monitor(&mut m, 0, Direction::Left, 10);
        assert_eq!(m[0].x, 0);
        assert_eq!(m[0].y, 0);
    }

    // --- connected layout tests ---

    #[test]
    fn test_connected_side_by_side() {
        let m = three_side_by_side();
        assert!(is_layout_connected(&m));
    }

    #[test]
    fn test_connected_single() {
        let m = vec![LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 }];
        assert!(is_layout_connected(&m));
    }

    #[test]
    fn test_disconnected() {
        let m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 5000, y: 5000, w: 1920, h: 1080 },
        ];
        assert!(!is_layout_connected(&m));
    }

    #[test]
    fn test_connected_l_shape() {
        let m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "C".into(), x: 1920, y: 1080, w: 1920, h: 1080 },
        ];
        assert!(is_layout_connected(&m));
    }

    // --- normalize tests ---

    #[test]
    fn test_normalize() {
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 100, y: 50, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 2020, y: 50, w: 1920, h: 1080 },
        ];
        normalize(&mut m);
        assert_eq!(m[0].x, 0);
        assert_eq!(m[0].y, 0);
        assert_eq!(m[1].x, 1920);
        assert_eq!(m[1].y, 0);
    }

    // --- snap_to_side tests ---

    #[test]
    fn test_snap_to_right() {
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 5000, y: 5000, w: 2560, h: 1440 },
        ];
        snap_to_side(&mut m, 1, 0, Direction::Right);
        assert_eq!(m[1].x, 1920);
        assert_eq!(m[1].y, 0);
    }

    #[test]
    fn test_snap_above() {
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 1080, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 5000, y: 5000, w: 2560, h: 1440 },
        ];
        snap_to_side(&mut m, 1, 0, Direction::Up);
        assert_eq!(m[1].x, 0);
        assert_eq!(m[1].y, 1080 - 1440);
    }

    // --- auto_snap_all tests ---

    #[test]
    fn test_auto_snap_fixes_floating_monitor() {
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "C".into(), x: 5000, y: 5000, w: 1920, h: 1080 },
        ];
        auto_snap_all(&mut m);
        // C should now be touching something
        let c_touches = (0..m.len())
            .any(|j| j != 2 && shared_edge(&m[2], &m[j]).is_some());
        assert!(c_touches, "C should be snapped to touch another monitor: {:?}", m);
    }

    #[test]
    fn test_auto_snap_already_connected() {
        let mut m = three_side_by_side();
        let before = m.clone();
        auto_snap_all(&mut m);
        // Should not change anything
        assert_eq!(m, before);
    }

    #[test]
    fn test_auto_snap_after_swap_with_third() {
        // Simulate: 3 monitors, swap 0 and 1, check that 2 stays connected
        let mut m = vec![
            LayoutMonitor { id: "A".into(), x: 0, y: 0, w: 1920, h: 1080 },
            LayoutMonitor { id: "B".into(), x: 1920, y: 0, w: 2560, h: 1440 },
            LayoutMonitor { id: "C".into(), x: 4480, y: 0, w: 1920, h: 1080 },
        ];
        swap_monitors(&mut m, 0, 1);
        auto_snap_all(&mut m);
        assert!(is_layout_connected(&m), "Layout should be connected after swap+snap: {:?}", m);
    }

    // --- snap_to_far_side tests ---

    #[test]
    fn test_snap_to_far_right() {
        let mut m = three_side_by_side();
        // Snap A (index 0, leftmost) to far right
        snap_to_far_side(&mut m, 0, Direction::Right);
        auto_snap_all(&mut m);
        // A should now be at the rightmost position
        let max_right = m.iter().map(|m| m.right()).max().unwrap();
        assert_eq!(m[0].right(), max_right, "A should be at far right: {:?}", m);
    }

    #[test]
    fn test_snap_to_far_left() {
        let mut m = three_side_by_side();
        // Snap C (index 2, rightmost) to far left
        snap_to_far_side(&mut m, 2, Direction::Left);
        auto_snap_all(&mut m);
        normalize(&mut m);
        assert_eq!(m[2].x, 0, "C should be at far left: {:?}", m);
    }

    #[test]
    fn test_snap_to_far_down() {
        let mut m = three_side_by_side();
        snap_to_far_side(&mut m, 1, Direction::Down);
        auto_snap_all(&mut m);
        // B should be below some other monitor
        assert!(m[1].y > 0, "B should be below: {:?}", m);
    }

    // --- find_neighbor tests ---

    #[test]
    fn test_find_neighbor_right() {
        let m = three_side_by_side();
        assert_eq!(find_neighbor(&m, 0, Direction::Right), Some(1));
        assert_eq!(find_neighbor(&m, 1, Direction::Right), Some(2));
        assert_eq!(find_neighbor(&m, 2, Direction::Right), None);
    }

    #[test]
    fn test_find_neighbor_left() {
        let m = three_side_by_side();
        assert_eq!(find_neighbor(&m, 0, Direction::Left), None);
        assert_eq!(find_neighbor(&m, 1, Direction::Left), Some(0));
        assert_eq!(find_neighbor(&m, 2, Direction::Left), Some(1));
    }
}
