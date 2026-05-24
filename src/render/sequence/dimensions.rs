use crate::model::ScaleSpec;
use crate::scene::Scene;

pub(super) fn compute_svg_dimensions(scene: &Scene) -> (String, String, String) {
    let w = scene.width;
    let h = scene.height;
    let viewbox = format!("0 0 {} {}", w, h);
    match &scene.scale {
        None => (w.to_string(), h.to_string(), viewbox),
        Some(ScaleSpec::Factor(f)) => {
            let sw = (w as f64 * f).round() as i32;
            let sh = (h as f64 * f).round() as i32;
            (sw.to_string(), sh.to_string(), viewbox)
        }
        Some(ScaleSpec::Width(target_w)) => {
            let factor = *target_w as f64 / w as f64;
            let sh = (h as f64 * factor).round() as i32;
            (target_w.to_string(), sh.to_string(), viewbox)
        }
        Some(ScaleSpec::Height(target_h)) => {
            let factor = *target_h as f64 / h as f64;
            let sw = (w as f64 * factor).round() as i32;
            (sw.to_string(), target_h.to_string(), viewbox)
        }
        Some(ScaleSpec::Fixed {
            width: fw,
            height: fh,
        }) => (fw.to_string(), fh.to_string(), viewbox),
        Some(ScaleSpec::Max(max)) => {
            let max = *max as f64;
            let larger = (w.max(h)) as f64;
            if larger <= max {
                (w.to_string(), h.to_string(), viewbox)
            } else {
                let factor = max / larger;
                let sw = (w as f64 * factor).round() as i32;
                let sh = (h as f64 * factor).round() as i32;
                (sw.to_string(), sh.to_string(), viewbox)
            }
        }
        Some(ScaleSpec::MaxWidth(max_w)) => {
            if w <= *max_w as i32 {
                (w.to_string(), h.to_string(), viewbox)
            } else {
                let factor = *max_w as f64 / w as f64;
                let sh = (h as f64 * factor).round() as i32;
                (max_w.to_string(), sh.to_string(), viewbox)
            }
        }
        Some(ScaleSpec::MaxHeight(max_h)) => {
            if h <= *max_h as i32 {
                (w.to_string(), h.to_string(), viewbox)
            } else {
                let factor = *max_h as f64 / h as f64;
                let sw = (w as f64 * factor).round() as i32;
                (sw.to_string(), max_h.to_string(), viewbox)
            }
        }
        Some(ScaleSpec::MaxFixed {
            width: max_w,
            height: max_h,
        }) => {
            if w <= *max_w as i32 && h <= *max_h as i32 {
                (w.to_string(), h.to_string(), viewbox)
            } else {
                let factor = (*max_w as f64 / w as f64).min(*max_h as f64 / h as f64);
                let sw = (w as f64 * factor).round() as i32;
                let sh = (h as f64 * factor).round() as i32;
                (sw.to_string(), sh.to_string(), viewbox)
            }
        }
    }
}
