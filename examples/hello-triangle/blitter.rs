use std::borrow::Cow;
use wgpu::util::DeviceExt;

pub fn create_blue_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: usize,
    height: usize,
) -> wgpu::Texture {
    let texture_extent = wgpu::Extent3d {
        width: width as u32,
        height: height as u32,
        depth_or_array_layers: 1,
    };

    // let texture = device.create_texture();
    let usage = wgpu::TextureUsage::RENDER_ATTACHMENT
        | wgpu::TextureUsage::COPY_SRC
        | wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::SAMPLED;
    let desc = wgpu::TextureDescriptor {
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage,
        label: None,
    };
    let red: [u8; 4] = [0, 0, 255, 255];
    let data = vec![red; width * height];

    let slice = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const _, data.len() * 4) };

    let texture = device.create_texture_with_data(queue, &desc, slice);

    texture
}

// use std::rc::Rc;
// use std::cell::RefCell;
pub struct WGPUBlitter {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    // bind_group_cache: BindGroupCache,
}

impl WGPUBlitter {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shader_flags: wgpu::ShaderFlags,
    ) -> Self {
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("blit.wgsl"))),
            flags: shader_flags,
        });
        // uniform
        // texture view
        // sampler
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                //uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Self {
            sampler,
            bind_group_layout,
            pipeline,
        }
    }
}

impl WGPUBlitter {
    pub fn create_blit_encoder<'a>(&'a self, device: &wgpu::Device) -> BlitEncoder<'a> {
        let encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        BlitEncoder {
            encoder,
            bind_groups: Default::default(),

            bind_group_layout: &self.bind_group_layout,
            sampler: &self.sampler,
            pipeline: &self.pipeline,
        }
    }
}

// a outlasts b
pub struct BlitEncoder<'a> {
    encoder: wgpu::CommandEncoder,
    bind_groups: typed_arena::Arena<wgpu::BindGroup>,

    bind_group_layout: &'a wgpu::BindGroupLayout,
    sampler: &'a wgpu::Sampler,
    pipeline: &'a wgpu::RenderPipeline,
}

impl<'b> BlitEncoder<'b> {
    pub fn create_blit_pass<'a>(&'a mut self, target: &'a wgpu::TextureView) -> BlitPass<'a> where 'b: 'a {
        let pass_desc = wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None, // todo! what's this?
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };
        let pass = self.encoder.begin_render_pass(&pass_desc);
        BlitPass {
            pass,
            bind_group_layout: &self.bind_group_layout,
            sampler: &self.sampler,
            bind_groups: &self.bind_groups,
        }
    }

    pub fn finish(self) -> wgpu::CommandBuffer {
        self.encoder.finish()
    }
}

pub struct BlitPass<'a> {
    pub pass: wgpu::RenderPass<'a>,
    pub bind_group_layout: &'a wgpu::BindGroupLayout,
    pub sampler: &'a wgpu::Sampler,
    pub bind_groups: &'a typed_arena::Arena<wgpu::BindGroup>,
}

impl<'a> BlitPass<'a> {
    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &'a wgpu::TextureView,
    ) -> &'a wgpu::BindGroup {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.bind_groups.alloc(bind_group)
    }

    pub fn blit(
        &mut self,
        device: &wgpu::Device,
        src: &'a wgpu::TextureView,
        src_size: (f32, f32),
        dst_origin: (f32, f32),
    ) {
        let bg = self.create_bind_group(device, src);

        self.pass
            .set_viewport(dst_origin.0, dst_origin.1, src_size.0, src_size.1, 0.0, 1.0);

        self.pass.set_bind_group(0, &bg, &[]);
        self.pass.draw(0..4, 0..1);
    }
}
