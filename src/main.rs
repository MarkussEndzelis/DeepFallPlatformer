use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex{
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex{
    fn desc() -> wgpu::VertexBufferLayout<'static>{
        wgpu::VertexBufferLayout{
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute{
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute{
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct Player {
    x: f32,
    y: f32,
    vel_x: f32,
    vel_y: f32,
    on_ground: bool,
    width: f32,
    height: f32,
}

impl Player {
    fn new() -> Self{
        Self{
            x: 0.0,
            y: 0.0,
            vel_x: 0.0,
            vel_y: 0.0,
            on_ground: false,
            width: 40.0,
            height: 50.0,
        }
    }

    fn update(&mut self, left: bool, right: bool, jump: bool, platforms: &[Platform], dt: f32){
        let speed = 200.0;
        let gravity = 800.0;
        let jump_force = -450.0;

        self.vel_x = 0.0;
        if left {self.vel_x = -speed; }
        if right {self.vel_x = speed; }

        if jump && self.on_ground{
            self.vel_y = jump_force;
            self.on_ground = false;
        }

        self.vel_y += gravity * dt;
        self.x += self.vel_x * dt;
        self.y += self.vel_y * dt;

        self.on_ground = false;
        for plat in platforms{
            if self.x + self.width > plat.x
                && self.x < plat.x + plat.width
                && self.y + self.height > plat.y
                && self.y + self.height < plat.y + plat.height + 20.0
                && self.vel_y >= 0.0
            {
                self.y = plat.y - self.height;
                self.vel_y = 0.0;
                self.on_ground = true;
            }
        }
    }
}

struct Platform{
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn world_to_clip(wx: f32, wy: f32, cam_x: f32, cam_y: f32, screen_w: f32, screen_h: f32) -> [f32; 2]{
    let cx = ((wx - cam_x) / screen_w) * 2.0 - 1.0;
    let cy = 1.0 - ((wy - cam_y) / screen_h) * 2.0;
    [cx, cy]
}

fn rect_to_vertices(
    x: f32, y: f32, w: f32, h: f32,
    color: [f32; 4],
    cam_x: f32, cam_y: f32,
    screen_w: f32, screen_h: f32,
) -> (Vec<Vertex>, Vec<u16>){
    let tl = world_to_clip(x, y, cam_x, cam_y, screen_w, screen_h);
    let tr = world_to_clip(x + w, y, cam_x, cam_y, screen_w, screen_h);
    let br = world_to_clip(x + w, y + h, cam_x, cam_y, screen_w, screen_h);
    let bl = world_to_clip(x, y + h, cam_x, cam_y, screen_w, screen_h);

    let vertices = vec![
        Vertex {position: tl, color},
        Vertex {position: tr, color},
        Vertex {position: br, color},
        Vertex {position: bl, color},
    ];
    let indices = vec![0u16, 1, 2, 0, 2, 3];
    (vertices, indices)
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,
    player: Player,
    platforms: Vec<Platform>,
    left: bool,
    right: bool,
    jump: bool,
    last_time: std::time::Instant,
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState{
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState{
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState{
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let platforms = vec![
            Platform {x: -400.0, y: 300.0, width: 800.0, height: 20.0},
            Platform {x: -200.0, y: 150.0, width: 200.0, height: 20.0},
            Platform {x: 100.0, y: 50.0, width: 200.0, height: 20.0},
            Platform {x: -300.0, y: 0.0, width: 150.0, height: 20.0},
        ];

        let mut player = Player::new();
        player.x = -20.0;
        player.y = 200.0;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            player,
            platforms,
            left: false,
            right: false,
            jump: false,
            last_time: std::time::Instant::now(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, key: KeyCode, pressed: bool){
        match key {
            KeyCode::ArrowLeft | KeyCode::KeyA => self.left = pressed,
            KeyCode::ArrowRight | KeyCode::KeyD => self.right = pressed,
            KeyCode::Space | KeyCode::ArrowUp | KeyCode::KeyW => self.jump = pressed,
            _ => {}
        }
    }

    fn update(&mut self){
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32().min(0.05);
        self.last_time = now;

        self.player.update(self.left, self.right, self.jump, &self.platforms, dt);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let sw = self.size.width as f32;
        let sh = self.size.height as f32;

        let cam_x = self.player.x + self.player.width / 2.0 - sw / 2.0;
        let cam_y = self.player.y + self.player.height / 2.0 - sh / 2.0;

        let mut all_verticas: Vec<Vertex> = Vec::new();
        let mut all_indices: Vec<u16> = Vec::new();

        for plat in &self.platforms{
            let base = all_verticas.len() as u16;
            let(verts, inds) = rect_to_vertices(
                plat.x, plat.y, plat.width, plat.height,
                [0.3, 0.6, 0.3, 1.0],
                cam_x, cam_y, sw, sh,
            );
            all_verticas.extend(verts);
            all_indices.extend(inds.iter().map(|i| i + base));
        }

        let base = all_verticas.len() as u16;
        let(verts, inds) = rect_to_vertices(
            self.player.x, self.player.y,
            self.player.width, self.player.height,
            [0.9, 0.3, 0.3, 1.0],
            cam_x, cam_y, sw, sh,
        );
        all_verticas.extend(verts);
        all_indices.extend(inds.iter().map(|i| i + base));

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_verticas),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.04,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Deepfall")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .unwrap(),
    );

    let mut state = pollster::block_on(State::new(window.clone()));

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent {event, window_id} if window_id == state.window.id() => {
                    match event{
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(physical_size) => state.resize(physical_size),
                        WindowEvent::KeyboardInput{
                            event: KeyEvent{
                                physical_key: PhysicalKey::Code(key),
                                state: key_state,
                                ..
                            },
                            ..
                        } => {
                            state.input(key, key_state == ElementState::Pressed);
                        }
                        WindowEvent::RedrawRequested => {
                            state.update();
                            match state.render(){
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                            state.window.request_redraw();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }).unwrap();
}