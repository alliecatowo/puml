use super::util::gantt_task_key;
use super::*;

/// Build a [`RenderScene`] from the laid-out Gantt geometry. Each task bar
/// and milestone diamond centre is recorded as a [`SceneNode`] at the
/// exact pixel position the SVG draws it, so scene and SVG never diverge.
// Scene builder threads the already-computed Gantt layout geometry (widths,
// row tops, bar height, day map); grouping into a struct would just duplicate
// the layout fields one-to-one.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_gantt_scene(
    document: &TimelineDocument,
    ordered_tasks: &[&TimelineTask],
    milestone_day: &BTreeMap<&str, u32>,
    width: i32,
    total_h: i32,
    chart_top: i32,
    bar_height: i32,
    row_gap: i32,
    task_count: i32,
    bar_geom: &dyn Fn(&TimelineTask) -> (i32, i32),
    day_to_x: &dyn Fn(u32) -> i32,
    chart_left: i32,
    chart_w: i32,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, total_h as f64));

    // Add a scene node for each task bar at its drawn position.
    for (i, task) in ordered_tasks.iter().enumerate() {
        let row = i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2;
        let (bx, bw) = bar_geom(task);
        if bw == 0 {
            continue;
        }
        let id = format!("task::{}", gantt_task_key(task));
        let bounds = Rect::new(bx as f64, y as f64, bw as f64, bar_height as f64);
        let label = LabelBox {
            id: format!("{id}::label"),
            text: task.name.clone(),
            bounds,
            owner_id: Some(id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: id.clone(),
            node_box: NodeBox {
                id,
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    // Add a scene node for each milestone diamond (diamond centre ± r as a square).
    for (i, milestone) in document.milestones.iter().enumerate() {
        let row = task_count + i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2;
        let cy = y + bar_height / 2;
        let cx = milestone_day
            .get(milestone.name.as_str())
            .map(|d| day_to_x(*d))
            .unwrap_or(chart_left + chart_w / 2);
        let r = (bar_height / 2 - 2).max(4);
        let id = format!("milestone::{}", milestone.name);
        let bounds = Rect::new(
            (cx - r) as f64,
            (cy - r) as f64,
            (r * 2) as f64,
            (r * 2) as f64,
        );
        let label = LabelBox {
            id: format!("{id}::label"),
            text: milestone.name.clone(),
            bounds,
            owner_id: Some(id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: id.clone(),
            node_box: NodeBox {
                id,
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    scene
}
