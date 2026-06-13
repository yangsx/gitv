use crate::shaders;
use crate::vertex::{EDGE_ATTRIBUTES, EdgeVertex, GraphUniform, NodeInstance};
use std::num::NonZero;
use tracing::{Level, debug, info, info_span, span};
use wgpu::util::DeviceExt;

const STAGING_BUFFER_COUNT: usize = 2;

/// Configuration for the render target.
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

/// Graph layout viewport data received from the frontend.
pub struct GraphViewportData {
    pub visible_start: usize,
    pub visible_end: usize,
    pub total_columns: usize,
    pub row_height: f32,
    pub lane_width: f32,
    pub padding_left: f32,
    pub node_radius: f32,
    pub scale: f32,
    /// Nodes in the visible range.
    pub nodes: Vec<NodeInstance>,
    /// Pre-tessellated edge quad vertices (4 per straight segment).
    pub edge_vertices: Vec<EdgeVertex>,
    /// Edge indices for indexed drawing.
    pub edge_indices: Vec<u32>,
}

/// WGSL-based GPU renderer for the commit graph.
///
/// Renders offscreen and returns RGBA pixel data via `render()`.
pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    // Render target
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,

    // Pipelines
    node_pipeline: wgpu::RenderPipeline,
    node_bind_group: wgpu::BindGroup,

    edge_pipeline: wgpu::RenderPipeline,
    edge_bind_group: wgpu::BindGroup,

    // Uniforms
    uniform_buffer: wgpu::Buffer,

    // Node instance storage buffer
    node_storage: wgpu::Buffer,
    node_storage_capacity: u64,

    // Edge vertex/index buffers
    edge_vertex_buf: Option<wgpu::Buffer>,
    edge_index_buf: Option<wgpu::Buffer>,
    edge_vertex_capacity: u64,
    edge_index_capacity: u64,

    // Staging ring-buffer for readback
    staging_buffers: Vec<wgpu::Buffer>,
    staging_index: usize,
    staging_size: u64,

    pub config: RenderConfig,
}

impl WgpuRenderer {
    /// Create a new renderer. Initialises GPU device, pipelines, and initial textures.
    pub async fn new(config: RenderConfig) -> Result<Self, String> {
        let span = info_span!("wgpu_init");
        let _enter = span.enter();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or("no GPU adapter found")?;

        let adapter_info = adapter.get_info();
        debug!(
            "wgpu adapter: {} ({:?})",
            adapter_info.name, adapter_info.backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("gitv-wgpu-device"),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| format!("device request failed: {e}"))?;

        let texture = Self::create_texture(&device, config.width, config.height);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("graph_uniforms"),
            contents: bytemuck::bytes_of(&GraphUniform::new(
                config.width,
                config.height,
                0.0,
                4.0,
                config.scale,
            )),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let node_pipeline = Self::create_node_pipeline(&device)?;
        let edge_pipeline = Self::create_edge_pipeline(&device)?;

        let unpadded_bpr = config.width.max(1) * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;
        let staging_size = padded_bpr as u64 * config.height.max(1) as u64;
        let staging_buffers = (0..STAGING_BUFFER_COUNT)
            .map(|i| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("staging_{i}")),
                    size: staging_size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();

        // Initial small storage buffer
        let node_storage = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("node_storage"),
            size: 64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let node_bind_group = Self::create_node_bind_group(&device, &uniform_buffer, &node_storage);
        let edge_bind_group = Self::create_edge_bind_group(&device, &uniform_buffer);

        debug!(
            "wgpu renderer initialised ({}x{} scale={})",
            config.width, config.height, config.scale
        );

        Ok(Self {
            device,
            queue,
            texture,
            texture_view,
            node_pipeline,
            node_bind_group,
            edge_pipeline,
            edge_bind_group,
            uniform_buffer,
            node_storage,
            node_storage_capacity: 0,
            edge_vertex_buf: None,
            edge_index_buf: None,
            edge_vertex_capacity: 0,
            edge_index_capacity: 0,
            staging_buffers,
            staging_index: 0,
            staging_size,
            config,
        })
    }

    // ── public API ─────────────────────────────────────────

    /// Resize the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == self.config.width && height == self.config.height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.texture = Self::create_texture(&self.device, width, height);
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let unpadded_bpr = width.max(1) * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;
        self.staging_size = padded_bpr as u64 * height.max(1) as u64;
        self.staging_buffers = (0..STAGING_BUFFER_COUNT)
            .map(|i| {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("staging_{i}")),
                    size: self.staging_size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        self.staging_index = 0;

        self.node_pipeline =
            Self::create_node_pipeline(&self.device).expect("recreate node pipeline");
        self.edge_pipeline =
            Self::create_edge_pipeline(&self.device).expect("recreate edge pipeline");
        self.node_bind_group =
            Self::create_node_bind_group(&self.device, &self.uniform_buffer, &self.node_storage);
        self.edge_bind_group = Self::create_edge_bind_group(&self.device, &self.uniform_buffer);
    }

    /// Render a single frame and return RGBA pixel data.
    ///
    /// Blocks the current thread for GPU readback (`device.poll(Maintain::Wait)`).
    /// Call from a blocking Tauri command thread.
    pub fn render(&mut self, viewport: &GraphViewportData) -> Result<Vec<u8>, String> {
        let _render_span = span!(Level::INFO, "wgpu_render").entered();
        let t_start = std::time::Instant::now();

        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let unpadded_bpr = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;

        // ── 1. Update uniforms ──────────────────────────────
        let scroll_y = viewport.visible_start as f32 * viewport.row_height * viewport.scale;
        let uniforms = GraphUniform::new(
            width,
            height,
            scroll_y,
            viewport.node_radius,
            viewport.scale,
        );
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // ── 2. Upload node instances ────────────────────────
        let t_nodes_start = std::time::Instant::now();
        let node_count = viewport.nodes.len();
        let node_bytes = bytemuck::cast_slice(&viewport.nodes);
        if node_bytes.len() as u64 > self.node_storage.size() {
            let new_size = (node_bytes.len() as u64).next_power_of_two();
            self.node_storage = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("node_storage"),
                size: new_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.node_storage_capacity = new_size;
        }
        if !node_bytes.is_empty() {
            self.queue.write_buffer(&self.node_storage, 0, node_bytes);
        }
        self.node_bind_group =
            Self::create_node_bind_group(&self.device, &self.uniform_buffer, &self.node_storage);
        let nodes_us = t_nodes_start.elapsed().as_micros() as u64;

        // ── 3. Upload edge geometry ─────────────────────────
        let t_edges_start = std::time::Instant::now();
        let edge_verts = &viewport.edge_vertices;
        let edge_idx = &viewport.edge_indices;

        let vb_needed = if edge_verts.is_empty() {
            0u64
        } else {
            (edge_verts.len() * size_of::<EdgeVertex>()).next_power_of_two() as u64
        };
        let ib_needed = if edge_idx.is_empty() {
            0u64
        } else {
            (edge_idx.len() * size_of::<u32>()).next_power_of_two() as u64
        };

        if vb_needed > self.edge_vertex_capacity {
            self.edge_vertex_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("edge_vertex"),
                size: vb_needed,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.edge_vertex_capacity = vb_needed;
        }
        if ib_needed > self.edge_index_capacity && !edge_idx.is_empty() {
            self.edge_index_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("edge_index"),
                size: ib_needed,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.edge_index_capacity = ib_needed;
        }

        let has_edges = !edge_verts.is_empty();
        if has_edges {
            if let Some(ref vb) = self.edge_vertex_buf {
                self.queue
                    .write_buffer(vb, 0, bytemuck::cast_slice(edge_verts));
            }
            if !edge_idx.is_empty()
                && let Some(ref ib) = self.edge_index_buf
            {
                self.queue
                    .write_buffer(ib, 0, bytemuck::cast_slice(edge_idx));
            }
        }
        let edge_index_count = edge_idx.len();

        // ── 4. Update edge bind group ───────────────────────
        self.edge_bind_group = Self::create_edge_bind_group(&self.device, &self.uniform_buffer);
        let edges_us = t_edges_start.elapsed().as_micros() as u64;

        // ── 5. Encode render pass ──────────────────────────
        let t_pass_start = std::time::Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graph_encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("graph_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            // Draw edges first (behind nodes)
            if has_edges {
                rpass.set_pipeline(&self.edge_pipeline);
                rpass.set_bind_group(0, &self.edge_bind_group, &[]);
                if let Some(ref vb) = self.edge_vertex_buf {
                    rpass.set_vertex_buffer(0, vb.slice(..));
                }
                if edge_index_count > 0 {
                    if let Some(ref ib) = self.edge_index_buf {
                        rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        rpass.draw_indexed(0..edge_index_count as u32, 0, 0..1);
                    }
                } else if !edge_verts.is_empty() {
                    rpass.draw(0..edge_verts.len() as u32, 0..1);
                }
            }

            // Draw nodes on top
            if node_count > 0 {
                rpass.set_pipeline(&self.node_pipeline);
                rpass.set_bind_group(0, &self.node_bind_group, &[]);
                rpass.draw(0..4, 0..node_count as u32);
            }
        }

        let pass_us = t_pass_start.elapsed().as_micros() as u64;

        // ── 6. Copy to staging + submit ────────────────────
        let t_copy_start = std::time::Instant::now();
        let staging = &self.staging_buffers[self.staging_index % STAGING_BUFFER_COUNT];
        self.staging_index += 1;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bpr),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit([encoder.finish()]);
        let copy_us = t_copy_start.elapsed().as_micros() as u64;

        // ── 7. Readback ───────────────────────────────────
        let t_readback_start = std::time::Instant::now();
        let (tx, rx) = std::sync::mpsc::channel();
        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        // Poll synchronously — safe because this runs on a blocking Tauri thread
        self.device.poll(wgpu::Maintain::Wait);

        // Check mapping result
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(format!("buffer map failed: {e}")),
            Err(_) => return Err("staging buffer map timed out".into()),
        }

        // Copy out, stripping row-padding bytes
        let data = slice.get_mapped_range();
        let pixels = if padded_bpr == unpadded_bpr {
            data.to_vec()
        } else {
            let mut out = Vec::with_capacity((unpadded_bpr * height) as usize);
            for row in 0..height {
                let offset = (row * padded_bpr) as usize;
                out.extend_from_slice(&data[offset..offset + unpadded_bpr as usize]);
            }
            out
        };
        drop(data);
        staging.unmap();

        let total_us = t_start.elapsed().as_micros() as u64;
        info!(
            nodes = node_count,
            edge_verts = edge_verts.len(),
            upload_nodes_us = nodes_us,
            upload_edges_us = edges_us,
            encode_pass_us = pass_us,
            copy_submit_us = copy_us,
            readback_us = t_readback_start.elapsed().as_micros() as u64,
            total_us,
            "wgpu render complete"
        );

        Ok(pixels)
    }

    // ── private helpers ──────────────────────────────────────

    fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("graph_render_target"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn create_node_pipeline(device: &wgpu::Device) -> Result<wgpu::RenderPipeline, String> {
        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("node_vs"),
            source: wgpu::ShaderSource::Wgsl(shaders::NODE_VERTEX.into()),
        });
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("node_fs"),
            source: wgpu::ShaderSource::Wgsl(shaders::NODE_FRAGMENT.into()),
        });

        let node_layout = Self::node_bind_group_layout(device);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("node_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("node_layout"),
                    bind_group_layouts: &[&node_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_node"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("fs_node"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Ok(pipeline)
    }

    fn node_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("node_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            NonZero::new(size_of::<GraphUniform>() as u64).expect("GraphUniform size is non-zero"),
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_node_bind_group(
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        node_storage: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let layout = Self::node_bind_group_layout(device);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("node_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: NonZero::new(size_of::<GraphUniform>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: node_storage,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        })
    }

    fn create_edge_pipeline(device: &wgpu::Device) -> Result<wgpu::RenderPipeline, String> {
        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("edge_vs"),
            source: wgpu::ShaderSource::Wgsl(shaders::EDGE_VERTEX.into()),
        });
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("edge_fs"),
            source: wgpu::ShaderSource::Wgsl(shaders::EDGE_FRAGMENT.into()),
        });

        let edge_layout = Self::edge_bind_group_layout(device);
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<EdgeVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &EDGE_ATTRIBUTES,
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edge_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("edge_layout"),
                    bind_group_layouts: &[&edge_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_edge"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[vertex_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("fs_edge"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Ok(pipeline)
    }

    fn edge_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("edge_bg_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZero::new(size_of::<GraphUniform>() as u64),
                },
                count: None,
            }],
        })
    }

    fn create_edge_bind_group(device: &wgpu::Device, uniform: &wgpu::Buffer) -> wgpu::BindGroup {
        let layout = Self::edge_bind_group_layout(device);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("edge_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: NonZero::new(size_of::<GraphUniform>() as u64),
                }),
            }],
        })
    }
}
