use std::borrow::Cow;
// we have to create a new encoder since we need the target

// pub trait DeviceExt {
//     // fn
// }

// impl DeviceExt for wgpu::Device {
//     fn create_blit_encoder(&self) -> {

//     }
// }

// struct BindGroupCache {
//     inner: std::collections::HashMap<femtovg::ImageId, wgpu::BindGroup>,
// }

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
    let desc = wgpu::TextureDescriptor {
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
        label: None,
    };
    let red: [u8; 4] = [0, 0, 255, 255];
    let data = vec![red; width * height];

    let slice = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const _, data.len() * 4) };

    let texture = device.create_texture_with_data(queue, &desc, slice);

    texture
}

// impl BindGroupCache {
//     pub fn new() -> Self {
//         Self {
//             inner: Default::default(),
//         }
//     }

//     pub fn get(
//         &mut self,
//         device: &wgpu::Device,
//         layout: &wgpu::BindGroupLayout,
//         image: femtovg::ImageId,
//     ) -> &wgpu::BindGroup {
//         if !self.inner.contains_key(&image) {
//             let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
//                 label: None,
//                 layout,
//                 entries: &[],
//             });
//             self.inner.insert(image, bg);
//         }
//         &self.inner[&image]
//     }

//     pub fn clear(&mut self) {
//         self.inner.clear();
//     }
// }

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
        // let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // let path = root.join("src/shaders/blit.wgsl");

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

        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("blit bind group"),
        //     layout: &bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(input),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&sampler),
        //         },
        //     ],
        // });

        // let bind_group_cache = BindGroupCache::new();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: None,
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

    pub fn create_blit_pass<'a>(
        &'a mut self,
        encoder: &'a mut wgpu::CommandEncoder,
        target: &'a wgpu::TextureView,
    ) -> BlitPass<'a> {
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
            // depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            //     view: stencil_view,
            //     // depth_ops: None,
            //     depth_ops: Some(wgpu::Operations {
            //         load: wgpu::LoadOp::Clear(0.0),
            //         store: true,
            //     }),
            //     // todo: what is this?
            //     // stencil_ops: None,
            //     stencil_ops: Some(wgpu::Operations {
            //         load: wgpu::LoadOp::Clear(0),
            //         store: true,
            //     }), //Option<Operations<u32>>,
            //         // stencil_ops: Some(wgpu::Operations {
            //         //     load: wgpu::LoadOp::Clear(0),
            //         //     store: true,
            //         // }), //Option<Operations<u32>>,
            // }),
        };

        // let mut pass = encoder.begin_render_pass(&pass_desc);
        // pass.set_viewport(0.0, 0.0, view_size.w as _, view_size.h as _, 0.0, 1.0);

        // pass.set_vertex_buffer(0, vertex_buffer.as_ref().slice(..));
        // pass.set_stencil_reference(0);

        // pass.set_index_buffer(index_buffer.as_ref().slice(..), wgpu::IndexFormat::Uint32);
        let mut pass = encoder.begin_render_pass(&pass_desc);
        pass.set_pipeline(&self.pipeline);

        BlitPass {
            pass,
            bind_group_layout: &self.bind_group_layout,
            sampler: &self.sampler,
            bind_groups: vec![],
            // bind_group_cache: &mut self.bind_group_cache,
        }
    }

    // pub fn clear_cache(&mut self) {
    //     self.bind_group_cache.clear();
    // }

    // fn end(&mut self) {

    // }
}

pub struct BlitPass<'a> {
    pass: wgpu::RenderPass<'a>,
    bind_group_layout: &'a wgpu::BindGroupLayout,
    sampler: &'a wgpu::Sampler,
    bind_groups: Vec<wgpu::BindGroup>,
    // bind_group_cache: &'a mut BindGroupCache,
}

impl<'a> BlitPass<'a> {
    pub fn blit(
        &'a mut self,
        device: &wgpu::Device,

        // id: femtovg::ImageId,
        src: &wgpu::TextureView,
        src_size: (f32, f32),
        dst_origin: (f32, f32),
    ) {
        // pass.set
        // let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: None,
        //     layout: &self.bind_group_layout,
        //     entries: &[],
        // });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.bind_groups.push(bind_group);
        let bg = self.bind_groups.last().unwrap();
        // self.bind_group_cache
        // .get(device, self.bind_group_layout, id);
        self.pass
            .set_viewport(dst_origin.0, dst_origin.1, src_size.0, src_size.1, 0.0, 1.0);

        self.pass.set_bind_group(0, &bg, &[]);
        self.pass.draw(0..4, 0..1);
    }

    // fn finish(mut self) -> wgpu::CommandBuffer {
    //     self.pass.finish()
    // }
}

// impl first  {

// }
