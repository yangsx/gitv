use bytemuck::{Pod, Zeroable};

/// Per-instance data for a node rendered as an instanced quad with SDF circle.
///
/// Layout must match the WGSL `NodeInst` struct in `shaders.rs`, where
/// `vec4<f32>` and `vec3<f32>` are aligned to 16-byte boundaries in
/// `var<storage>` buffers.  Explicit padding fields make the alignment
/// explicit so `bytemuck::Pod` is satisfied.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NodeInstance {
    /// Center of the node in viewport space (px)
    pub center_x: f32,
    pub center_y: f32,
    // ── 8 bytes padding to align the following vec4 to 16 ──
    pub _pad0: [u8; 8],
    /// Node colour (0–1 linear)
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub color_a: f32,
    /// Bit 0: selected, Bit 1: dimmed, Bit 2: merge, Bit 3: comparison
    pub flags: u32,
    // ── 12 bytes padding to align the following vec3 to 16 ──
    pub _pad1: [u8; 12],
    /// Selection ring colour
    pub sel_r: f32,
    pub sel_g: f32,
    pub sel_b: f32,
    // ── 4 bytes padding to make struct size a multiple of 16 ──
    pub _pad2: [u8; 4],
}

impl NodeInstance {
    pub const SELECTED: u32 = 1;
    pub const DIMMED: u32 = 2;
    pub const MERGE: u32 = 4;
    pub const COMPARISON: u32 = 8;
}

/// Pre-tessellated edge quad vertex (simple position + colour).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct EdgeVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

pub const EDGE_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 8,
        shader_location: 1,
    },
];

/// Uniform buffer for orthographic projection + scroll offset + SDF constants.
///
/// Layout must match the WGSL `Uniforms` struct in `shaders.rs`.
/// Total size: 96 bytes (multiple of 16 for WGSL struct alignment).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GraphUniform {
    pub projection: [[f32; 4]; 4],
    pub scroll_y: f32,
    pub node_radius: f32,
    pub quad_half: f32,
    pub ring_outer: f32,
    pub ring_width: f32,
    _pad: [f32; 3],
}

impl GraphUniform {
    /// Create uniform data.
    ///
    /// `width`/`height` — render target size in physical pixels.
    /// `scroll_y` — vertical scroll offset in physical pixels.
    /// `node_radius_css` — node radius in CSS pixels.
    /// `scale` — device pixel ratio.
    pub fn new(width: u32, height: u32, scroll_y: f32, node_radius_css: f32, scale: f32) -> Self {
        let w = width as f32;
        let h = height as f32;
        // Standard orthographic: left=0, right=w, bottom=h, top=0 (Y-down to match canvas)
        let projection = [
            [2.0 / w, 0.0, 0.0, 0.0],
            [0.0, -2.0 / h, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        let nr = node_radius_css * scale;
        Self {
            projection,
            scroll_y,
            node_radius: nr,
            quad_half: (nr + 3.0 * scale) + 1.0 * scale,
            ring_outer: nr + 3.0 * scale,
            ring_width: 2.0 * scale,
            _pad: [0.0; 3],
        }
    }
}
