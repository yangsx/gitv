/// WGSL vertex shader for nodes (SDF circle + selection ring via instanced quads).
pub const NODE_VERTEX: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    scroll_y: f32,
    node_radius: f32,
    quad_half: f32,
    ring_outer: f32,
    ring_width: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct NodeInst {
    center: vec2<f32>,
    color: vec4<f32>,
    flags: u32,
    sel_color: vec3<f32>,
};
@group(0) @binding(1) var<storage, read> nodes: array<NodeInst>;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) center: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) flags: u32,
    @location(4) sel_color: vec3<f32>,
};

@vertex
fn vs_node(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VOut {
    let corner = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let inst = nodes[ii];
    let world_center = vec2<f32>(inst.center.x, inst.center.y - uniforms.scroll_y);
    let quad_pos = world_center + (corner * 2.0 - 1.0) * uniforms.quad_half;

    var out: VOut;
    out.pos = uniforms.projection * vec4<f32>(quad_pos, 0.0, 1.0);
    out.color = inst.color;
    out.center = world_center;
    out.uv = corner * 2.0 - 1.0;
    out.flags = inst.flags;
    out.sel_color = inst.sel_color;
    return out;
}
";

/// WGSL fragment shader for nodes (SDF circle + selection ring).
///
/// All SDF distances use uniforms (scaled by DPR at runtime).
/// `in.uv` is in normalized [-1, 1] quad space, so we multiply by
/// `uniforms.quad_half` to convert to pixel space before distance checks.
pub const NODE_FRAGMENT: &str = r"
struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) center: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) flags: u32,
    @location(4) sel_color: vec3<f32>,
};

struct Uniforms {
    projection: mat4x4<f32>,
    scroll_y: f32,
    node_radius: f32,
    quad_half: f32,
    ring_outer: f32,
    ring_width: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@fragment
fn fs_node(in: VOut) -> @location(0) vec4<f32> {
    // Convert from normalized uv [-1, 1] to pixel space
    let d = length(in.uv) * uniforms.quad_half;

    // Node body: smoothstep circle edge
    let body_alpha = 1.0 - smoothstep(uniforms.node_radius - 0.5, uniforms.node_radius + 0.5, d);

    // Selection ring (only when selected or comparison)
    var ring_alpha = 0.0;
    if (in.flags & 1u) != 0u || (in.flags & 8u) != 0u {
        let ring_inner = uniforms.ring_outer - uniforms.ring_width;
        let ring = smoothstep(uniforms.ring_outer + 0.5, uniforms.ring_outer - 0.5, d)
                 - smoothstep(ring_inner + 0.5, ring_inner - 0.5, d);
        ring_alpha = ring;
    }

    // Merge node marker: diamond shape
    // Diamond SDF in pixel space: |px| + |py| < radius
    // px = uv.x * quad_half, py = uv.y * quad_half
    var diamond_alpha = 0.0;
    if (in.flags & 4u) != 0u {
        let diamond = 1.0 - smoothstep(uniforms.node_radius - 0.5, uniforms.node_radius + 0.5, (abs(in.uv.x) + abs(in.uv.y)) * uniforms.quad_half);
        diamond_alpha = diamond;
    }

    let total_alpha = max(body_alpha, ring_alpha);
    if total_alpha < 0.01 { discard; }

    var c = in.color;
    if ring_alpha > 0.0 {
        let ring_color = vec4<f32>(in.sel_color, 1.0);
        c = mix(c, ring_color, ring_alpha * 0.9);
    }
    if (in.flags & 2u) != 0u {
        c.a *= 0.35;
    }
    if diamond_alpha > 0.0 {
        c = mix(c, vec4<f32>(1.0, 1.0, 1.0, c.a), diamond_alpha * 0.3);
    }

    return vec4<f32>(c.rgb, c.a * total_alpha);
}
";

/// WGSL vertex shader for edges (coloured quads).
pub const EDGE_VERTEX: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    scroll_y: f32,
    node_radius: f32,
    quad_half: f32,
    ring_outer: f32,
    ring_width: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_edge(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> VOut {
    let world_pos = vec2<f32>(position.x, position.y - uniforms.scroll_y);
    var out: VOut;
    out.pos = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.color = color;
    return out;
}
";

/// WGSL fragment shader for edges (pass-through colour with alpha discard).
pub const EDGE_FRAGMENT: &str = r"
struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@fragment
fn fs_edge(in: VOut) -> @location(0) vec4<f32> {
    if in.color.a < 0.01 { discard; }
    return in.color;
}
";
