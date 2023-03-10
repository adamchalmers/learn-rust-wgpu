use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
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
        let config = wgpu::SurfaceConfiguration {
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
        surface.configure(&device, &config);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
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

        // Clear the screen. Start a new block so that begin_render_pass (which holds &mut)
        // is dropped so that encoder can finish.
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // Describe where to draw the color to.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    // Same as view, unless multisampling is used.
                    resolve_target: None,
                    // What to do with the colours on the screen.
                    ops: wgpu::Operations {
                        // 'load' field is what to do with colours stored from previous frame.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        // Submit the cmdbuf to the GPU.
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
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

                    // Resize events.
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor: _,
                        new_inner_size,
                    } => {
                        state.resize(**new_inner_size);
                    }

                    _ => {}
                }
            }
        }
        // TODO: Support window resize events
        _ => {}
    });
}
