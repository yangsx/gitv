use crate::wgpu_state::WgpuState;
use gitv_wgpu_renderer::renderer::GraphViewportData;
use gitv_wgpu_renderer::vertex::{EdgeVertex, NodeInstance, edge_style};
use serde::Deserialize;
use tauri::{State, ipc::Response};
use tracing::instrument;

/// IPC input: a viewport of renderable graph data from the frontend.
#[derive(Deserialize)]
pub struct RenderGraphInput {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub visible_start: usize,
    pub visible_end: usize,
    pub total_columns: usize,
    pub row_height: f32,
    pub lane_width: f32,
    pub padding_left: f32,
    pub node_radius: f32,
    pub nodes: Vec<RenderNode>,
    pub edges: Vec<RenderEdge>,
}

#[derive(Deserialize)]
pub struct RenderNode {
    pub row: usize,
    pub column: usize,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub color_a: u8,
    pub is_dimmed: bool,
    pub is_selected: bool,
    pub is_comparison: bool,
    pub is_merge: bool,
    pub is_stash: bool,
    pub sel_color_r: u8,
    pub sel_color_g: u8,
    pub sel_color_b: u8,
}

#[derive(Deserialize)]
pub struct RenderEdge {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub is_dimmed: bool,
    #[allow(dead_code)]
    pub edge_type: String,
    pub edge_style: String,
}

fn to_norm(c: u8) -> f32 {
    c as f32 / 255.0
}

/// Tessellate edges into quad vertices. Straight edges produce one quad,
/// cross-column edges are tessellated as quadratic bezier curves.
/// All positions are in CSS pixels — caller scales by `input.scale`.
///
/// `edge_param[0]` = cumulative pixel distance from edge start (for dash pattern).
/// `edge_param[1]` = style flag from `edge_style` module.
fn tessellate_edge(
    edge: &RenderEdge,
    row_height: f32,
    lane_width: f32,
    padding_left: f32,
    scale: f32,
    is_dimmed: bool,
) -> Vec<EdgeVertex> {
    let x1 = (edge.from_col as f32 * lane_width + padding_left + lane_width / 2.0) * scale;
    let y1 = (edge.from_row as f32 * row_height + row_height / 2.0) * scale;
    let x2 = (edge.to_col as f32 * lane_width + padding_left + lane_width / 2.0) * scale;
    let y2 = (edge.to_row as f32 * row_height + row_height / 2.0) * scale;

    let color = [
        to_norm(edge.color_r),
        to_norm(edge.color_g),
        to_norm(edge.color_b),
        if is_dimmed { 0.35 } else { 0.8 },
    ];

    let style_flag = match edge.edge_style.as_str() {
        "Dashed" => edge_style::DASHED,
        "Dotted" => edge_style::DOTTED,
        _ => edge_style::SOLID,
    };

    let half_width = 0.75 * scale;

    if edge.from_col == edge.to_col {
        let total_dist = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
        vec![
            EdgeVertex {
                position: [x1 - half_width, y1],
                color,
                edge_param: [0.0, style_flag],
            },
            EdgeVertex {
                position: [x1 + half_width, y1],
                color,
                edge_param: [0.0, style_flag],
            },
            EdgeVertex {
                position: [x2 - half_width, y2],
                color,
                edge_param: [total_dist, style_flag],
            },
            EdgeVertex {
                position: [x2 + half_width, y2],
                color,
                edge_param: [total_dist, style_flag],
            },
        ]
    } else {
        let mid_x = (x1 + x2) / 2.0;
        let dy = y2 - y1;
        let cp1_x = mid_x;
        let cp1_y = y1 + dy * 0.25;
        let cp2_x = mid_x;
        let cp2_y = y2 - dy * 0.25;
        let segments = 16usize;
        let mut verts = Vec::with_capacity(segments * 4);

        let mut cum_dist = 0.0f32;

        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;

            let u0 = 1.0 - t0;
            let u1 = 1.0 - t1;

            let bx0 = u0.powi(3) * x1
                + 3.0 * u0.powi(2) * t0 * cp1_x
                + 3.0 * u0 * t0.powi(2) * cp2_x
                + t0.powi(3) * x2;
            let by0 = u0.powi(3) * y1
                + 3.0 * u0.powi(2) * t0 * cp1_y
                + 3.0 * u0 * t0.powi(2) * cp2_y
                + t0.powi(3) * y2;
            let bx1 = u1.powi(3) * x1
                + 3.0 * u1.powi(2) * t1 * cp1_x
                + 3.0 * u1 * t1.powi(2) * cp2_x
                + t1.powi(3) * x2;
            let by1 = u1.powi(3) * y1
                + 3.0 * u1.powi(2) * t1 * cp1_y
                + 3.0 * u1 * t1.powi(2) * cp2_y
                + t1.powi(3) * y2;

            let dx = bx1 - bx0;
            let dy = by1 - by0;
            let seg_len = (dx * dx + dy * dy).sqrt().max(0.001);
            let perp_x = -dy / seg_len * half_width;
            let perp_y = dx / seg_len * half_width;

            let d0 = cum_dist;
            cum_dist += seg_len;
            let d1 = cum_dist;

            verts.push(EdgeVertex {
                position: [bx0 - perp_x, by0 - perp_y],
                color,
                edge_param: [d0, style_flag],
            });
            verts.push(EdgeVertex {
                position: [bx0 + perp_x, by0 + perp_y],
                color,
                edge_param: [d0, style_flag],
            });
            verts.push(EdgeVertex {
                position: [bx1 - perp_x, by1 - perp_y],
                color,
                edge_param: [d1, style_flag],
            });
            verts.push(EdgeVertex {
                position: [bx1 + perp_x, by1 + perp_y],
                color,
                edge_param: [d1, style_flag],
            });
        }

        verts
    }
}

/// Wrapper for the render command that catches panics (e.g. GPU driver crashes)
/// so they become `Err(String)` instead of aborting the process under
/// `panic = "abort"`.
fn render_inner(wgpu_state: &WgpuState, input: &RenderGraphInput) -> Result<Vec<u8>, String> {
    wgpu_state.ensure_init(input.width, input.height)?;

    let mut guard = wgpu_state.renderer.lock().map_err(|e| e.to_string())?;
    let renderer = guard
        .as_mut()
        .ok_or_else(|| "wgpu renderer not initialised".to_string())?;

    let node_instances: Vec<NodeInstance> = input
        .nodes
        .iter()
        .map(|n| {
            let cx =
                (n.column as f32 * input.lane_width + input.padding_left + input.lane_width / 2.0)
                    * input.scale;
            let cy = (n.row as f32 * input.row_height + input.row_height / 2.0) * input.scale;
            let mut flags: u32 = 0;
            if n.is_selected {
                flags |= NodeInstance::SELECTED;
            }
            if n.is_dimmed {
                flags |= NodeInstance::DIMMED;
            }
            if n.is_merge {
                flags |= NodeInstance::MERGE;
            }
            if n.is_comparison {
                flags |= NodeInstance::COMPARISON;
            }
            if n.is_stash {
                flags |= NodeInstance::IS_STASH;
            }

            NodeInstance {
                center_x: cx,
                center_y: cy,
                color_r: to_norm(n.color_r),
                color_g: to_norm(n.color_g),
                color_b: to_norm(n.color_b),
                color_a: to_norm(n.color_a),
                flags,
                sel_r: to_norm(n.sel_color_r),
                sel_g: to_norm(n.sel_color_g),
                sel_b: to_norm(n.sel_color_b),
                _pad0: [0u8; 8],
                _pad1: [0u8; 12],
                _pad2: [0u8; 4],
            }
        })
        .collect();

    let edge_vertices: Vec<EdgeVertex> = input
        .edges
        .iter()
        .flat_map(|e| {
            tessellate_edge(
                e,
                input.row_height,
                input.lane_width,
                input.padding_left,
                input.scale,
                e.is_dimmed,
            )
        })
        .collect();

    // For each 4-vertex quad generate 6 indices (2 triangles) for TriangleList.
    // Quad vertices: [left@start, right@start, left@end, right@end].
    // Triangles: [0,1,2] + [2,1,3] (both CCW under Y-down projection).
    let edge_indices: Vec<u32> = (0..edge_vertices.len())
        .step_by(4)
        .flat_map(|base| {
            let i = base as u32;
            [i, i + 1, i + 2, i + 2, i + 1, i + 3]
        })
        .collect();

    let viewport = GraphViewportData {
        visible_start: input.visible_start,
        visible_end: input.visible_end,
        total_columns: input.total_columns,
        row_height: input.row_height,
        lane_width: input.lane_width,
        padding_left: input.padding_left,
        node_radius: input.node_radius,
        scale: input.scale,
        nodes: node_instances,
        edge_vertices,
        edge_indices,
    };

    renderer.render(&viewport)
}

#[tauri::command]
#[instrument(skip(wgpu_state, input), fields(command = "render_graph"))]
pub fn render_graph(
    wgpu_state: State<'_, WgpuState>,
    input: RenderGraphInput,
) -> Result<Response, String> {
    // Catch panics from GPU operations to avoid abort under `panic = "abort"`.
    // Response sends pixel data through Tauri's binary IPC, bypassing JSON
    // serialization — avoids allocating a 10 MB JSON array of 2M integers.
    let pixels = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render_inner(&wgpu_state, &input)
    })) {
        Ok(result) => result?,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "wgpu render panicked (unknown cause)".to_string()
            };
            tracing::error!("wgpu render panic: {msg}");
            return Err(msg);
        }
    };
    Ok(Response::new(pixels))
}
