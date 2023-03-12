use std::num::NonZeroU32;

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const BLUE: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

const R: [f32; 3] = [1.0, 0.0, 0.0];
const G: [f32; 3] = [0.0, 1.0, 0.0];
const B: [f32; 3] = [0.0, 0.0, 1.0];

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.99240386],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.56958647],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.05060294],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.1526709],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.7347359],
    }, // E
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    color: wgpu::Color,
    render_pipelines: Vec<wgpu::RenderPipeline>,
    active_pipeline: usize,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_bind_group: wgpu::BindGroup,
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU.
        // Backends::all => Vulkan + Metal + DX12 + Browser.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // The part of the window our code draws to.
        // Safety
        // Surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        // Adapter is a handle to the actual graphics card.
        // Use this to get info about GPU e.g. name, which backend it uses.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable graphics card available.");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    // Extra device features we need.
                    // We don't need any for now.
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // This tutorial assumes sRGB surface texture. If you want to support others, account for
        // them when drawing. If you don't, colours will come out darker than intended.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.describe().srgb)
            .unwrap_or(surface_caps.formats[0]);

        // Defines how surface creates its underlying SurfaceTextures.
        let surface_config = wgpu::SurfaceConfiguration {
            // How will the SurfaceTexture be used? They'll be used to write to the screen.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // How will they be stored on the GPU.
            format: surface_format,
            width: size.width,
            height: size.height,
            // This present_mode should be "Fifo" i.e. vsync. But in later extensions, users can
            // choose to disable that, so maybe it'll be customizable.
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let diffuse_image = image::load_from_memory(include_bytes!("tree.png")).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();
        use image::GenericImageView;
        let (width, height) = diffuse_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width,
            height,
            // All textures are stored as 3D, we represent our 2D texture
            // by setting depth to 1.
            depth_or_array_layers: 1,
        };
        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1, // What is a Mip?
            sample_count: 1,    // What is multisampling?
            dimension: wgpu::TextureDimension::D2,
            // Most images are stored using sRGB so we need to reflect that here.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            // This is the same as with the SurfaceConfig. It
            // specifies what texture formats can be used to
            // create TextureViews for this texture. The base
            // texture format (Rgba8UnormSrgb in this case) is
            // always supported. Note that using a different
            // texture format is not supported on the WebGL2
            // backend.
            view_formats: &[],
        });

        queue.write_texture(
            // Where should wgpu copy the data to?
            wgpu::ImageCopyTextureBase {
                texture: &&diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // Data to copy
            &diffuse_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * width), // I guess it's 4 for RGB and A?
                rows_per_image: NonZeroU32::new(height),
            },
            texture_size,
        );

        // We don't need to configure the texture view much, so let's
        // let wgpu define it.
        let diffuse_texture_view =
            diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // How the GPU lays out the texture on its side of memory.
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                // This needs two entries:
                entries: &[
                    // Entry 0 is the sampled texture itself.
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, // index
                        // This binding should be visible to the fragment shader, because it's
                        // used to colour the pixels.
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Entry 1 is the sampler.
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, // index
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // Conforms to the Bind Group Layout defined above.
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let boring_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Boring Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipelines = vec![create_pipeline(
            &device,
            &render_pipeline_layout,
            &boring_shader,
            &surface_config,
        )];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            size,
            color: BLUE,
            render_pipelines,
            active_pipeline: 0,
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            diffuse_bind_group,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    // Returns if event has been fully processed.
    // If so, main loop won't process event any further.
    // For now, return false because we don't handle any events.
    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        // This is where we would e.g. move objects. But there's nothing to do yet.
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get a frame to render to. Wait for the surface to provide a SurfaceTexture (frame),
        // which we'll render to.
        let output = self.surface.get_current_texture()?;
        // Controls how the render code interacts with the texture.
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Create a CommandEncoder which creates the actual commands sent to the GPU.
        // Modern graphics frameworks expect cmds to be stored in a cmdbuf, before being sent to GPU.
        // (presumably to minimize IO overhead). So, build the cmdbuf.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Clear the screen. Start a new block, because `render_pass` holds a &mut to `encoder`.
        // This way when render_pass is dropped, encoder becomes usable again.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // Describe where to draw the color to.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    // Same as view, unless multisampling is used.
                    resolve_target: None,
                    // What to do with the colours on the screen.
                    ops: wgpu::Operations {
                        // 'load' field is what to do with colours stored from previous frame.
                        load: wgpu::LoadOp::Clear(self.color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipelines[self.active_pipeline]);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            let buffer_slot = 0;
            render_pass.set_vertex_buffer(buffer_slot, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Submit the cmdbuf to the GPU.
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    render_pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    surface_config: &wgpu::SurfaceConfiguration,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            // Define how the vertex buffer is laid out.
            buffers: &[Vertex::descriptor()],
        },
        // Stores color data in the `surface`.
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            // What colour outputs it should set up.
            targets: &[
                // We only need one colour output, the `surface`.
                Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    // Replace old pixel data with new data. I guess other alternatives would be
                    // 'blend them together' somehow.
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        primitive: wgpu::PrimitiveState {
            // i.e. every 3 vertices corresponds to one triangle.
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            // How wgpu should tell if a given triangle is facing forwards or not.
            // CCW means it's facing forwards if vertices are arranged counter-clockwise.
            front_face: wgpu::FrontFace::Ccw,
            // What to cull (i.e. not draw). Anything facing backwards.
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            // How many samples the pipeline will use
            count: 1,
            // Which samples should be active? All of them.
            mask: !0,
            // For antialiasing.
            alpha_to_coverage_enabled: false,
        },
        // How many array layers the render attachments can have. Not using this.
        multiview: None,
    })
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2], // NEW!
}

impl Vertex {
    /// How does the vertex buffer's internal layout correspond to a set of these Vertices?
    /// Note this is pretty verbose, a macro `vertex_attr_array` exists to help.
    fn descriptor<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            /// How many bytes are in each element of the array
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            /// Do you increment the array index per-vertex or per-instance?
            /// I don't know what instances are yet, so, vertices here.
            step_mode: wgpu::VertexStepMode::Vertex,
            /// Maps attributes of the struct to locations in each element of the buffer.
            attributes: &[
                wgpu::VertexAttribute {
                    // Where the attribute starts.
                    offset: 0,
                    // In WGSL each attribute has a 'location' (analogous to protobuf's field number)
                    // This describes which location number the given attribute corresponds to.
                    shader_location: 0,
                    // Internal format of the attribute
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    // Offset after the [f32; 3] used for the previous attribute
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    // Store in @location(1)
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Adam GPU Demo")
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // If OOM, quit.
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // Other errors should be resolved by next frame.
                Err(e) => eprintln!("{:?}", e),
            }
        }

        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually request it.
            state.window().request_redraw();
        }

        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window().id() => {
            if !state.input(event) {
                match event {
                    // Detect window close.
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,

                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Space),
                                ..
                            },
                        ..
                    } => {
                        state.active_pipeline += 1;
                        state.active_pipeline %= state.render_pipelines.len();
                    }

                    // Resize events.
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }

                    // Mouse movement
                    WindowEvent::CursorMoved { position, .. } => {
                        let percent_of_screen_width = position.x / state.size.width as f64;
                        let percent_of_screen_height = position.y / state.size.height as f64;
                        state.color = wgpu::Color {
                            r: percent_of_screen_width,
                            g: percent_of_screen_height,
                            ..state.color
                        };
                    }

                    _ => {}
                }
            }
        }
        // TODO: Support window resize events
        _ => {}
    });
}
