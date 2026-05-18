/// Pick attachment ports on source and target rectangles based on relative
/// position. Returns (src_anchor, tgt_anchor) as (x1, y1, x2, y2). For
/// horizontally-dominant edges, attaches to left/right midpoints; for
/// vertically-dominant, to top/bottom midpoints.
///
/// Part of the layout engine refactor (#591, #590 epic).
pub(crate) fn pick_port(
    src: (i32, i32, i32, i32),
    tgt: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let (sx, sy, sw, sh) = src;
    let (tx, ty, tw, th) = tgt;
    let scx = sx + sw / 2;
    let scy = sy + sh / 2;
    let tcx = tx + tw / 2;
    let tcy = ty + th / 2;
    let dx = tcx - scx;
    let dy = tcy - scy;
    if dx.abs() > dy.abs() {
        // horizontal-dominant
        if dx > 0 {
            (sx + sw, scy, tx, tcy)
        } else {
            (sx, scy, tx + tw, tcy)
        }
    } else {
        // vertical-dominant (also handles dx == dy == 0 via dy branch)
        if dy > 0 {
            (scx, sy + sh, tcx, ty)
        } else {
            (scx, sy, tcx, ty + th)
        }
    }
}

pub(crate) fn compute_edge_anchors_for_direction(
    from: (i32, i32, i32, i32),
    to: (i32, i32, i32, i32),
    direction: Option<&str>,
) -> (i32, i32, i32, i32) {
    let (fx, fy, fw, fh) = from;
    let (tx, ty, tw, th) = to;
    match direction {
        Some("left") => (fx, fy + fh / 2, tx + tw, ty + th / 2),
        Some("right") => (fx + fw, fy + fh / 2, tx, ty + th / 2),
        Some("up") => (fx + fw / 2, fy, tx + tw / 2, ty + th),
        Some("down") => (fx + fw / 2, fy + fh, tx + tw / 2, ty),
        _ => compute_edge_anchors_tuple(from, to),
    }
}

pub(crate) fn compute_edge_anchors_tuple(
    from: (i32, i32, i32, i32),
    to: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let (fx, fy, fw, fh) = from;
    let (tx, ty, tw, th) = to;
    let fcx = fx + fw / 2;
    let fcy = fy + fh / 2;
    let tcx = tx + tw / 2;
    let tcy = ty + th / 2;
    let (x1, y1) = anchor_on_rect(fx, fy, fw, fh, tcx, tcy);
    let (x2, y2) = anchor_on_rect(tx, ty, tw, th, fcx, fcy);
    (x1, y1, x2, y2)
}

fn anchor_on_rect(x: i32, y: i32, w: i32, h: i32, tx: i32, ty: i32) -> (i32, i32) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let dx = tx - cx;
    let dy = ty - cy;
    if dx == 0 && dy == 0 {
        return (cx, cy);
    }
    // Determine which side to exit
    let half_w = (w as f64) / 2.0;
    let half_h = (h as f64) / 2.0;
    let abs_dx = (dx as f64).abs();
    let abs_dy = (dy as f64).abs();
    if abs_dx * half_h > abs_dy * half_w {
        // Exit via left or right edge
        if dx > 0 {
            (x + w, cy + ((half_w / abs_dx) * (dy as f64)) as i32)
        } else {
            (x, cy + ((half_w / abs_dx) * (dy as f64)) as i32)
        }
    } else if dy > 0 {
        (cx + ((half_h / abs_dy) * (dx as f64)) as i32, y + h)
    } else {
        (cx + ((half_h / abs_dy) * (dx as f64)) as i32, y)
    }
}

#[allow(dead_code)]
pub(crate) fn clip_to_box_edge(
    center: (i32, i32),
    target: (i32, i32),
    rect: (i32, i32, i32, i32),
) -> (i32, i32) {
    let (cx, cy) = center;
    let (tx, ty) = target;
    let (bx, by, bw, bh) = rect;
    let dx = (tx - cx) as f64;
    let dy = (ty - cy) as f64;
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (cx, cy);
    }
    let half_w = (bw as f64) / 2.0;
    let half_h = (bh as f64) / 2.0;
    let scale_x = if dx.abs() > 1e-6 {
        half_w / dx.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if dy.abs() > 1e-6 {
        half_h / dy.abs()
    } else {
        f64::INFINITY
    };
    let s = scale_x.min(scale_y);
    let ex = (cx as f64) + dx * s;
    let ey = (cy as f64) + dy * s;
    // Keep within box bounds
    let ex = ex.clamp(bx as f64, (bx + bw) as f64);
    let ey = ey.clamp(by as f64, (by + bh) as f64);
    (ex as i32, ey as i32)
}
