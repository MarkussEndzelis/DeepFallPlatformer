use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use rusttype::{Font, Scale, point};
use image::{ImageBuffer, Rgba};

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
    uv: [f32; 2],
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
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const PLAYER_SPRITE: [[u8; 12]; 16] = [
    [0,0,0,2,2,2,2,2,2,0,0,0],
    [0,0,2,4,4,4,4,4,4,1,0,0],
    [0,2,4,4,4,4,4,4,4,4,1,0],
    [0,2,4,4,1,1,1,1,4,4,2,0],
    [0,2,1,1,1,1,1,1,1,1,2,0],
    [0,2,1,2,1,1,1,1,2,1,2,0],
    [0,2,1,1,1,1,1,1,1,1,2,0],
    [0,0,2,1,1,1,1,1,1,2,0,0],
    [0,0,2,3,3,3,3,3,3,2,0,0],
    [0,2,3,3,3,3,3,3,3,3,2,0],
    [0,2,3,3,2,3,3,2,3,3,2,0],
    [0,2,3,3,2,3,3,2,3,3,2,0],
    [0,2,1,1,0,0,0,0,1,1,2,0],
    [0,2,1,1,0,0,0,0,1,1,2,0],
    [0,2,2,2,0,0,0,0,2,2,2,0],
    [0,0,0,0,0,0,0,0,0,0,0,0],
];

const TILE_SPRITE: [[u8; 8]; 8] = [
    [0,0,0,0,0,0,0,0],
    [1,0,0,1,0,0,1,0],
    [2,2,2,2,2,2,2,2],
    [2,3,2,2,3,2,2,3],
    [2,2,2,3,2,2,2,2],
    [3,2,2,2,2,3,2,2],
    [2,2,3,2,2,2,3,2],
    [2,2,2,2,2,2,2,2],
];

const FLAG_SPRITE: [[u8; 8]; 16] = [
    [0,0,1,1,1,1,0,0],
    [0,0,1,2,2,2,0,0],
    [0,0,1,2,2,2,0,0],
    [0,0,1,2,2,2,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,0,1,0,0,0,0,0],
    [0,3,3,3,0,0,0,0],
    [0,3,3,3,0,0,0,0],
    [3,3,3,3,3,0,0,0],
    [3,3,3,3,3,0,0,0],
];

const ENEMY_SPRITE: [[u8; 12]; 12] = [
    [0,0,0,1,1,1,1,1,1,0,0,0],
    [0,0,1,1,1,1,1,1,1,1,0,0],
    [0,1,2,2,1,1,1,1,2,2,1,0],
    [0,1,2,2,1,1,1,1,2,2,1,0],
    [0,1,1,1,1,1,1,1,1,1,1,0],
    [0,1,1,1,1,1,1,1,1,1,1,0],
    [0,1,1,1,1,1,1,1,1,1,1,0],
    [0,1,1,3,3,3,3,3,3,1,1,0],
    [0,1,1,3,3,3,3,3,3,1,1,0],
    [0,1,1,1,0,0,0,0,1,1,1,0],
    [0,1,1,1,0,0,0,0,1,1,1,0],
    [0,0,0,0,0,0,0,0,0,0,0,0],
];

const SKY_SPRITE: [[u8; 4]; 2] = [
    [30, 60, 130, 255],
    [80, 140, 210, 255],
];

fn sky_to_rgba(sprite: &[[u8; 4]; 2]) -> (Vec<u8>, u32, u32){
    let mut data = Vec::new();
    for row in sprite.iter(){
        data.extend_from_slice(row);
    }
    (data, 1, 2)
}

fn tile_to_rgba(sprite: &[[u8; 8]; 8]) -> (Vec<u8>, u32, u32){
    let palette: [[u8; 4]; 4] = [
        [90, 180, 90, 255],
        [70, 150, 70, 255],
        [120, 80, 50, 255],
        [95, 60, 35, 255],
    ];

    let width = 8u32;
    let height = 8u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for row in sprite.iter(){
        for &px in row.iter(){
            data.extend_from_slice(&palette[px as usize]);
        }
    }
    (data, width, height)
}

fn flag_to_rgba(sprite: &[[u8; 8]; 16]) -> (Vec<u8>, u32, u32){
    let palette: [[u8; 4]; 4] = [
    [0, 0, 0, 0],
    [80, 60, 40, 255],
    [220, 50, 50, 255],
    [60, 40, 20, 255],
    ];
    let width = 8u32;
    let height = 16u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for row in sprite.iter(){
        for &px in row.iter(){
            data.extend_from_slice(&palette[px as usize]);
        }
    }
    (data, width, height)
}

fn enemy_to_rgba(sprite: &[[u8; 12]; 12]) -> (Vec<u8>, u32, u32){
    let palette: [[u8; 4]; 4] = [
        [0, 0, 0, 0],
        [200, 50, 50, 255],
        [255, 255, 255, 255],
        [120, 30, 30, 255],
    ];
    let width = 12u32;
    let height = 12u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for row in sprite.iter(){
        for &px in row.iter(){
            data.extend_from_slice(&palette[px as usize]);
        }
    }
    (data, width, height)
}

fn sprite_to_rgba(sprite: &[[u8; 12]; 16]) -> (Vec<u8>, u32, u32){
    let palette: [[u8; 4]; 5] = [
    [0, 0, 0, 0],
    [240, 200, 170, 255],
    [40, 30, 30, 255],
    [60, 120, 220, 255],
    [90, 60, 40, 255],
    ];

    let width = 12u32;
    let height = 16u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for row in sprite.iter(){
        for &px in row.iter(){
            data.extend_from_slice(&palette[px as usize]);
        }
    }
    (data, width, height)
}

fn render_text_to_image(text: &str, font: &Font, font_size: f32, width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let scale = Scale::uniform(font_size);
    let glyphs: Vec<_> = font.layout(text, scale, point(0.0, 0.0)).collect();

    let mut img = ImageBuffer::<Rgba<u8>, _>::from_pixel(width, height, Rgba([0, 0, 0, 0]));

    for g in glyphs {
        if let Some(_bb) = g.pixel_bounding_box() {
            let pos = g.position();
            let x_offset = pos.x as i32;
            let y_offset = pos.y as i32;
            g.draw(|dx, dy, v| {
                let px = x_offset + dx as i32;
                let py = y_offset + dy as i32;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let alpha = (v * 255.0) as u8;
                    if alpha > 0 {
                        let pixel = img.get_pixel_mut(px as u32, py as u32);
                        *pixel = Rgba([255, 255, 255, alpha]);
                    }
                }
            });
        }
    }
    let raw = img.into_raw();
    (raw, width, height)
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

struct Coin {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    active: bool,
}

struct Goal {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

struct Enemy {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    vel_x: f32,
    alive: bool,
    platform_id: usize,
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

fn screen_to_clip(sx: f32, sy: f32, screen_w: f32, screen_h: f32) -> [f32; 2]{
    let cx = (sx / screen_w) * 2.0 - 1.0;
    let cy = 1.0 - (sy / screen_h) * 2.0;
    [cx, cy]
}

fn rect_to_vertices(
    x: f32, y: f32, w: f32, h: f32,
    uv_repeat_x: f32, uv_repeat_y: f32,
    cam_x: f32, cam_y: f32,
    screen_w: f32, screen_h: f32,
) -> (Vec<Vertex>, Vec<u16>){
    let tl = world_to_clip(x, y, cam_x, cam_y, screen_w, screen_h);
    let tr = world_to_clip(x + w, y, cam_x, cam_y, screen_w, screen_h);
    let br = world_to_clip(x + w, y + h, cam_x, cam_y, screen_w, screen_h);
    let bl = world_to_clip(x, y + h, cam_x, cam_y, screen_w, screen_h);

    let vertices = vec![
        Vertex {position: tl, uv: [0.0, 0.0]},
        Vertex {position: tr, uv: [uv_repeat_x, 0.0]},
        Vertex {position: br, uv: [uv_repeat_x, uv_repeat_y]},
        Vertex {position: bl, uv: [0.0, uv_repeat_y]},
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
    hud_pipeline: wgpu::RenderPipeline,
    diffuse_bind_group: wgpu::BindGroup,
    tile_bind_group: wgpu::BindGroup,
    sky_bind_group: wgpu::BindGroup,
    coin_bind_group: wgpu::BindGroup,
    flag_bind_group: wgpu::BindGroup,
    enemies: Vec<Enemy>,
    enemy_bind_group: wgpu::BindGroup,
    win_bind_group: wgpu::BindGroup,
    gameover_bind_group: wgpu::BindGroup,
    win_texture: wgpu::Texture,
    player: Player,
    platforms: Vec<Platform>,
    coins: Vec<Coin>,
    score: u32,
    score_texture: wgpu::Texture,
    score_texture_view: wgpu::TextureView,
    score_bind_group: wgpu::BindGroup,
    score_sampler: wgpu::Sampler,
    goal: Goal,
    game_won: bool,
    lives: u32,
    game_over: bool,
    left: bool,
    right: bool,
    jump: bool,
    last_time: std::time::Instant,
    font_bytes: Vec<u8>,
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

        let(sprite_rgba, sprite_w, sprite_h) = sprite_to_rgba(&PLAYER_SPRITE);

        let texture_size = wgpu::Extent3d{
            width: sprite_w,
            height: sprite_h,
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor{
            label: Some("player_sprite_texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture{
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &sprite_rgba,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: Some(4 * sprite_w),
                rows_per_image: Some(sprite_h),
            },
            texture_size,
        );

        let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture{
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float{filterable: true},
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding:0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let (tile_rgba, tile_w, tile_h) = tile_to_rgba(&TILE_SPRITE);

        let tile_texture_size = wgpu::Extent3d{
            width: tile_w,
            height: tile_h,
            depth_or_array_layers: 1,
        };

        let tile_texture = device.create_texture(&wgpu::TextureDescriptor{
            label: Some("tile_texture"),
            size: tile_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture{
                texture: &tile_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &tile_rgba,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: Some(4 * tile_w),
                rows_per_image: Some(tile_h),
            },
            tile_texture_size,
        );

        let tile_texture_view = tile_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let tile_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let tile_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tile_texture_view),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&tile_sampler),
                },
            ],
            label: Some("tile_bind_group"),
        });

        let(sky_rgba, sky_w, sky_h) = sky_to_rgba(&SKY_SPRITE);
        let sky_texture_size = wgpu::Extent3d{width: sky_w, height: sky_h, depth_or_array_layers: 1};
        let sky_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sky_texture"),
            size: sky_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{texture: &sky_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All},
            &sky_rgba,
            wgpu::ImageDataLayout{offset: 0, bytes_per_row: Some(4 * sky_w), rows_per_image: Some(sky_h)},
            sky_texture_size,
        );
        let sky_texture_view = sky_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{binding: 0, resource: wgpu::BindingResource::TextureView(&sky_texture_view)},
                wgpu::BindGroupEntry{binding: 1, resource: wgpu::BindingResource::Sampler(&sky_sampler)},
            ],
            label: Some("sky_bind_group"),
        });

        const COIN_SPRITE: [[u8; 8]; 8] = [
            [0,0,1,1,1,1,0,0],
            [0,1,2,2,2,2,1,0],
            [1,2,2,2,2,2,2,1],
            [1,2,2,3,3,2,2,1],
            [1,2,2,3,3,2,2,1],
            [1,2,2,2,2,2,2,1],
            [0,1,2,2,2,2,1,0],
            [0,0,1,1,1,1,0,0],
        ];

        fn coin_to_rgba(sprite: &[[u8; 8]; 8]) -> (Vec<u8>, u32, u32){
            let palette: [[u8; 4]; 4] = [
                [0, 0, 0, 0],
                [200, 120, 30, 255],
                [255, 200, 50, 255],
                [255, 255, 200, 255],
            ];
            let width = 8u32;
            let height = 8u32;
            let mut data = Vec::with_capacity((width * height * 4) as usize);
            for row in sprite.iter(){
                for &px in row.iter(){
                    data.extend_from_slice(&palette[px as usize]);
                }
            }
            (data, width, height)
        }

        let (coin_rgba, coin_w, coin_h) = coin_to_rgba(&COIN_SPRITE);
        let coin_texture_size = wgpu::Extent3d{
            width: coin_w,
            height: coin_h,
            depth_or_array_layers: 1,
        };
        let coin_texture = device.create_texture(&wgpu::TextureDescriptor{
            label: Some("coin_texture"),
            size: coin_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{
                texture: &coin_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &coin_rgba,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: Some(4 * coin_w),
                rows_per_image: Some(coin_h),
            },
            coin_texture_size,
        );
        let coin_texture_view = coin_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let coin_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let coin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&coin_texture_view),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&coin_sampler),
                },
            ],
            label: Some("coin_bind_group"),
        });

        let (flag_rgba, flag_w, flag_h) = flag_to_rgba(&FLAG_SPRITE);
        let flag_texture_size = wgpu::Extent3d {width: flag_w, height: flag_h, depth_or_array_layers: 1};
        let flag_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("flag_texture"),
            size: flag_texture_size,
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{texture: &flag_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All},
            &flag_rgba,
            wgpu::ImageDataLayout{offset: 0, bytes_per_row: Some(4 * flag_w), rows_per_image: Some(flag_h)},
            flag_texture_size,
        );
        let flag_texture_view = flag_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let flag_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let flag_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{binding: 0, resource: wgpu::BindingResource::TextureView(&flag_texture_view)},
                wgpu::BindGroupEntry{binding: 1, resource: wgpu::BindingResource::Sampler(&flag_sampler)},
            ],
            label: Some("flag_bind_group"),
        });

        let (enemy_rgba, enemy_w, enemy_h) = enemy_to_rgba(&ENEMY_SPRITE);
        let enemy_texture_size = wgpu::Extent3d { width: enemy_w, height: enemy_h, depth_or_array_layers: 1};
        let enemy_texture = device.create_texture(&wgpu::TextureDescriptor{
            label: Some("enemy_texture"),
            size: enemy_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{texture: &enemy_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All},
            &enemy_rgba,
            wgpu::ImageDataLayout{offset: 0, bytes_per_row: Some(4 * enemy_w), rows_per_image: Some(enemy_h)},
            enemy_texture_size,
        );
        let enemy_texture_view = enemy_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let enemy_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let enemy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{binding: 0, resource: wgpu::BindingResource::TextureView(&enemy_texture_view)},
                wgpu::BindGroupEntry{binding: 1, resource: wgpu::BindingResource::Sampler(&enemy_sampler)},
            ],
            label: Some("enemy_bind_group"),
        });

        let win_text = "You Win!".to_string();
        let win_font_data = include_bytes!("../assets/DejaVuSans.ttf");
        let win_font = Font::try_from_bytes(win_font_data).expect("Failed to load font");
        let (win_img, win_w, win_h) = render_text_to_image(&win_text, &win_font, 100.0, 1024, 256);
        let win_texture_size = wgpu::Extent3d {width: win_w, height: win_h, depth_or_array_layers: 1};
        let win_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("win_texture"),
            size: win_texture_size,
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{texture: &win_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All},
            &win_img,
            wgpu::ImageDataLayout{offset: 0, bytes_per_row: Some(4 * win_w), rows_per_image: Some(win_h)},
            win_texture_size,
        );
        let win_texture_view = win_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let win_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let win_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{binding: 0, resource: wgpu::BindingResource::TextureView(&win_texture_view)},
                wgpu::BindGroupEntry{binding: 1, resource: wgpu::BindingResource::Sampler(&win_sampler)},
            ],
            label: Some("win_bind_group"),
        });

        let gameover_text = "Game Over".to_string();
        let gameover_font_data = include_bytes!("../assets/DejaVuSans.ttf");
        let gameover_font = Font::try_from_bytes(gameover_font_data).expect("Failed to load font");
        let (gameover_img, gameover_w, gameover_h) = render_text_to_image(&gameover_text, &gameover_font, 72.0, 512, 128);
        let gameover_texture_size = wgpu::Extent3d{width: gameover_w, height: gameover_h, depth_or_array_layers: 1};
        let gameover_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gameover_texture"),
            size: gameover_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{texture: &gameover_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All},
            &gameover_img,
            wgpu::ImageDataLayout{offset: 0, bytes_per_row: Some(4 * gameover_w), rows_per_image: Some(gameover_h)},
            gameover_texture_size,
        );
        let gameover_texture_view = gameover_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let gameover_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let gameover_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{binding: 0, resource: wgpu::BindingResource::TextureView(&gameover_texture_view)},
                wgpu::BindGroupEntry{binding: 1, resource: wgpu::BindingResource::Sampler(&gameover_sampler)},
            ],
            label: Some("gameover_bind_group"),
        });

        let font_data = include_bytes!("../assets/DejaVuSans.ttf");
        let font = Font::try_from_bytes(font_data).expect("Failed to load font");
        let font_bytes = font_data.to_vec();

        let score_text = "Score: 0".to_string();
        let (img, width, height) = render_text_to_image(&score_text, &font, 28.0, 256, 64);

        let score_texture_size = wgpu::Extent3d{
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        };
        let score_texture = device.create_texture(&wgpu::TextureDescriptor{
            label: Some("score_texture"),
            size: score_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture{
                texture: &score_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: Some(4 * width as u32),
                rows_per_image: Some(height as u32),
            },
            score_texture_size,
        );

        let score_texture_view = score_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let score_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let score_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&score_texture_view),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&score_sampler),
                },
            ],
            label: Some("score_bind_group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
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

        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HUD Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState{
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState{
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent{
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent{
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
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
            Platform {x: -400.0, y: 300.0, width: 600.0, height: 20.0},

            Platform {x: 200.0, y: 300.0, width: 300.0, height: 20.0},
            Platform {x: 600.0, y: 240.0, width: 150.0, height: 20.0},
            Platform {x: 820.0, y: 180.0, width: 150.0, height: 20.0},
            Platform {x: 1040.0, y: 120.0, width: 150.0, height: 20.0},

            Platform{x: 1300.0, y: 60.0, width: 120.0, height: 20.0},
            Platform{x: 1500.0, y: 120.0, width: 120.0, height: 20.0},
            Platform{x: 1700.0, y: 200.0, width: 200.0, height: 20.0},

            Platform{x: 1980.0, y: 280.0, width: 180.0, height: 20.0},
            Platform{x: 2240.0, y: 340.0, width: 160.0, height: 20.0},
            Platform{x: 2480.0, y: 400.0, width: 160.0, height: 20.0},

            Platform{x: 2720.0, y: 460.0, width: 200.0, height: 20.0},

            Platform{x: -600.0, y: 450.0, width: 200.0, height: 20.0},
            Platform{x: -850.0, y: 550.0, width: 200.0, height: 20.0},
        ];

        let coins = vec![
            Coin{x: -300.0, y: 280.0, width: 16.0, height: 16.0, active: true},
            Coin{x: -200.0, y: 280.0, width: 16.0, height: 16.0, active: true},
            Coin{x: -100.0, y: 280.0, width: 16.0, height: 16.0, active: true},

            Coin{x: 250.0, y: 280.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 320.0, y: 280.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 640.0, y: 220.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 860.0, y: 160.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 1080.0, y: 100.0, width: 16.0, height: 16.0, active: true},

            Coin{x: 1330.0, y: 40.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 1530.0, y: 100.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 1730.0, y: 180.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 1780.0, y: 180.0, width: 16.0, height: 16.0, active: true},

            Coin{x: 2020.0, y: 260.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 2070.0, y: 260.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 2280.0, y: 320.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 2520.0, y: 380.0, width: 16.0, height: 16.0, active: true},

            Coin{x: 2740.0, y: 440.0, width: 16.0, height: 16.0, active: true},
            Coin{x: 2790.0, y: 440.0, width: 16.0, height: 16.0, active: true},

            Coin{x: -550.0, y: 430.0, width: 16.0, height: 16.0, active: true},
            Coin{x: -800.0, y: 530.0, width: 16.0, height: 16.0, active: true},
        ];

        let enemies = vec![
            Enemy{x: 250.0, y: 300.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 1},
            Enemy{x: 650.0, y: 240.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0, alive: true, platform_id: 2},
            Enemy{x: 870.0, y: 180.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 3},
            Enemy{x: 1090.0, y: 120.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 4},
            Enemy{x: 1350.0, y: 60.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 5},
            Enemy{x: 1550.0, y: 120.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 6},
            Enemy{x: 1750.0, y: 200.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 7},
            Enemy{x: 2030.0, y: 280.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 8},
            Enemy{x: 2290.0, y: 340.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 9},
            Enemy{x: 2530.0, y: 400.0 - 40.0, width: 40.0, height: 40.0, vel_x: 80.0,  alive: true, platform_id: 10},
        ];

        let mut player = Player::new();
        player.x = -20.0;
        player.y = 200.0;

        let goal = Goal{x: 2800.0, y: 460.0 - 64.0, width: 32.0, height: 64.0};

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            hud_pipeline,
            diffuse_bind_group,
            tile_bind_group,
            sky_bind_group,
            coin_bind_group,
            score_texture,
            score_texture_view,
            score_bind_group,
            score_sampler,
            goal,
            game_won: false,
            lives: 3,
            game_over: false,
            flag_bind_group,
            win_bind_group,
            gameover_bind_group,
            win_texture,
            font_bytes,
            coins,
            score: 0,
            enemy_bind_group,
            enemies,
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
            KeyCode::KeyR => {
            if self.game_over || self.game_won {
                self.lives = 3;
                self.score = 0;
                self.game_won = false;
                self.game_over = false;
                self.player.x = -20.0;
                self.player.y = 200.0;
                self.player.vel_x = 0.0;
                self.player.vel_y = 0.0;
                self.player.on_ground = false;
                for coin in &mut self.coins {
                    coin.active = true;
                }
                for enemy in &mut self.enemies {
                    enemy.alive = true;
                }
                self.update_score_texture();
                println!("Game restarted!");
            }
        }
            _ => {}
            
        }  
    }

    fn update(&mut self){
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32().min(0.05);
        self.last_time = now;

        self.player.update(self.left, self.right, self.jump, &self.platforms, dt);

        for coin in &mut self.coins{
            if coin.active{
                if self.player.x + self.player.width > coin.x
                    && self.player.x < coin.x + coin.width
                    && self.player.y + self.player.height > coin.y 
                    && self.player.y < coin.y + coin.height
                {
                    coin.active = false;
                    self.score += 1;
                }
            }
        }

        for enemy in &mut self.enemies {
            if !enemy.alive {continue; }

            let plat = &self.platforms[enemy.platform_id];

            enemy.x += enemy.vel_x * dt;

            let margin = 5.0;
            let left_edge = plat.x + margin;
            let right_edge = plat.x + plat.width - enemy.width - margin;

            if enemy.x < left_edge {
                enemy.x = left_edge;
                enemy.vel_x = enemy.vel_x.abs();
            }else if enemy.x > right_edge {
                enemy.x = right_edge;
                enemy.vel_x = -enemy.vel_x.abs();
            }
            enemy.y = plat.y - enemy.height;
        }
        
            for enemy in &mut self.enemies {
                if !enemy.alive {continue;}
                if self.player.x + self.player.width > enemy.x + 5.0
                    && self.player.x < enemy.x + enemy.width - 5.0
                    && self.player.y + self.player.height > enemy.y + 5.0
                    && self.player.y < enemy.y + enemy.height - 5.0
                {
                    if self.player.vel_y > 0.0 {
                        enemy.alive = false;
                        self.score += 1;
                        self.player.vel_y = -300.0;
                        println!("Enemy destroyed! Score: {}", self.score);
                    }else{
                        if self.lives == 0{
                            self.game_over = true;
                            return;
                        }
                        self.lives -= 1;
                        self.player.x = -20.0;
                        self.player.y = 200.0;
                        self.player.vel_x = 0.0;
                        self.player.vel_y = 0.0;
                        self.player.on_ground = false;
                        self.score = 0;
                        self.game_won = false;
                        for coin in &mut self.coins{
                            coin.active = true;
                        }
                        for enemy in &mut self.enemies{
                            enemy.alive = true;
                        }
                        println!("Respawned! Lives remaining: {}", self.lives);
                        break;
                    }
                }
            }

        if self.player.y > 1500.0 {
            if self.lives == 0 {
                self.game_over = true;
                return;
            }
            self.lives -= 1;
            self.player.x = -20.0;
            self.player.y = 200.0;
            self.player.vel_x = 0.0;
            self.player.vel_y = 0.0;
            self.player.on_ground = false;
            self.score = 0;
            self.game_won = false;
            for coin in &mut self.coins{
                coin.active = true;
            }
            for enemy in &mut self.enemies {
                enemy.alive = true;
            }
            println!("Respawned! Lives remaining: {}", self.lives);
        }

        self.update_score_texture();

        if !self.game_won {
            if self.player.x + self.player.width > self.goal.x
                && self.player.x < self.goal.x + self.goal.width
                && self.player.y + self.player.height > self.goal.y
                && self.player.y < self.goal.y + self.goal.height
            {
                self.game_won = true;
            }
        }
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

        let mut bg_vertices: Vec<Vertex> = Vec::new();
        let mut bg_indices: Vec<u16> = Vec::new();
        {
            let (verts, inds) = rect_to_vertices(
                cam_x, cam_y, sw, sh,
                1.0, 1.0,
                cam_x, cam_y, sw, sh,
            );
            bg_vertices.extend(verts);
            bg_indices.extend(inds);
        }

        let bg_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BG Vertex Buffer"),
            contents: bytemuck::cast_slice(&bg_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let bg_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BG Index Buffer"),
            contents: bytemuck::cast_slice(&bg_indices),
            usage: wgpu::BufferUsages::INDEX,
        });


        let mut platform_vertices: Vec<Vertex> = Vec::new();
        let mut platform_indices: Vec<u16> = Vec::new();
        for plat in &self.platforms{
            let base = platform_vertices.len() as u16;
            let repeat_x = (plat.width / 24.0).max(1.0);
            let repeat_y = (plat.height / 24.0).max(1.0);
            let(verts, inds) = rect_to_vertices(
                plat.x, plat.y, plat.width, plat.height,
                repeat_x, repeat_y,
                cam_x, cam_y, sw, sh,
            );
            platform_vertices.extend(verts);
            platform_indices.extend(inds.iter().map(|i| i + base));
        }

        let mut player_vertices: Vec<Vertex> = Vec::new();
        let mut player_indices: Vec<u16> = Vec::new();
        let(verts, inds) = rect_to_vertices(
            self.player.x, self.player.y,
            self.player.width, self.player.height,
            1.0, 1.0,
            cam_x, cam_y, sw, sh,
        );
        player_vertices.extend(verts);
        player_indices.extend(inds);

        let mut coin_vertices: Vec<Vertex> = Vec::new();
        let mut coin_indices: Vec<u16> = Vec::new();
        for coin in &self.coins{
            if !coin.active {continue; }
            let base = coin_vertices.len() as u16;
            let (verts, inds) = rect_to_vertices(
                coin.x, coin.y,
                coin.width, coin.height,
                1.0, 1.0,
                cam_x, cam_y, sw, sh,
            );
            coin_vertices.extend(verts);
            coin_indices.extend(inds.iter().map(|i| i + base));
        }

        let platform_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Platform Vartex Buffer"),
            contents: bytemuck::cast_slice(&platform_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let platform_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Platform Index Buffer"),
            contents: bytemuck::cast_slice(&platform_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let player_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Player Vertex Buffer"),
            contents: bytemuck::cast_slice(&player_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let player_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Player Index Buffer"),
            contents: bytemuck::cast_slice(&player_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let coin_vertex_buffer = if !coin_vertices.is_empty(){
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
                label: Some("Coin Vertex Buffer"),
                contents: bytemuck::cast_slice(&coin_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        }else{None};

        let coin_index_buffer = if !coin_indices.is_empty(){
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
                label: Some("Coin Index Buffer"),
                contents: bytemuck::cast_slice(&coin_indices),
                usage: wgpu::BufferUsages::INDEX,
            }))
        }else{None};

        let mut flag_vertices: Vec<Vertex> = Vec::new();
        let mut flag_indices: Vec<u16> = Vec::new();
        let (verts, inds) = rect_to_vertices(
            self.goal.x, self.goal.y,
            self.goal.width, self.goal.height,
            1.0, 1.0,
            cam_x, cam_y, sw, sh,
        );
        flag_vertices.extend(verts);
        flag_indices.extend(inds);
        let flag_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flag Vertex Buffer"),
            contents: bytemuck::cast_slice(&flag_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let flag_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flag Index Buffer"),
            contents: bytemuck::cast_slice(&flag_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut enemy_vertices: Vec<Vertex> = Vec::new();
        let mut enemy_indices: Vec<u16> = Vec::new();
        for enemy in &self.enemies{
            if !enemy.alive{continue;}
            let base = enemy_vertices.len() as u16;
            let (verts, inds) = rect_to_vertices(
                enemy.x, enemy.y,
                enemy.width, enemy.height,
                1.0, 1.0,
                cam_x, cam_y, sw, sh,
            );
            enemy_vertices.extend(verts);
            enemy_indices.extend(inds.iter().map(|i| i + base));
        }
        let enemy_vertex_buffer = if !enemy_vertices.is_empty(){
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Enemy Vertex Buffer"),
                contents: bytemuck::cast_slice(&enemy_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        }else{None};

        let enemy_index_buffer = if !enemy_indices.is_empty(){
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Enemy Index Buffer"),
                contents: bytemuck::cast_slice(&enemy_indices),
                usage: wgpu::BufferUsages::INDEX,
            }))
        }else{None};

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.24,
                            b: 0.51,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
                render_pass.set_pipeline(&self.hud_pipeline);

                render_pass.set_bind_group(0, &self.sky_bind_group, &[]);
                render_pass.set_vertex_buffer(0, bg_vertex_buffer.slice(..));
                render_pass.set_index_buffer(bg_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..bg_indices.len() as u32, 0, 0..1);



                render_pass.set_bind_group(0, &self.tile_bind_group, &[]);
                render_pass.set_vertex_buffer(0, platform_vertex_buffer.slice(..));
                render_pass.set_index_buffer(platform_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..platform_indices.len() as u32, 0, 0..1);

                render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
                render_pass.set_vertex_buffer(0, player_vertex_buffer.slice(..));
                render_pass.set_index_buffer(player_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..player_indices.len() as u32, 0, 0..1);

                if let (Some(vb), Some(ib)) = (coin_vertex_buffer.as_ref(), coin_index_buffer.as_ref()){
                    render_pass.set_bind_group(0, &self.coin_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..coin_indices.len() as u32, 0, 0..1);
                }
                
                render_pass.set_bind_group(0, &self.flag_bind_group, &[]);
                render_pass.set_vertex_buffer(0, flag_vertex_buffer.slice(..));
                render_pass.set_index_buffer(flag_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..flag_indices.len() as u32, 0, 0..1);

                if let (Some(vb), Some(ib)) = (enemy_vertex_buffer.as_ref(), enemy_index_buffer.as_ref()){  
                    render_pass.set_bind_group(0, &self.enemy_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..enemy_indices.len() as u32, 0, 0..1);
                }
        }

        
        
        let hud_width = 256.0;
        let hud_height = 64.0;
        let hud_x = 20.0;
        let hud_y = 20.0;

        let tl = screen_to_clip(hud_x, hud_y, sw, sh);
        let tr = screen_to_clip(hud_x + hud_width, hud_y, sw, sh);
        let br = screen_to_clip(hud_x + hud_width, hud_y + hud_height, sw, sh);
        let bl = screen_to_clip(hud_x, hud_y + hud_height, sw, sh);

        let hud_vertices = vec![
            Vertex { position: tl, uv: [0.0, 0.0] },
            Vertex { position: tr, uv: [1.0, 0.0] },
            Vertex { position: br, uv: [1.0, 1.0] },
            Vertex { position: bl, uv: [0.0, 1.0] },
        ];

        let hud_indices = vec![0u16, 1, 2, 0, 2, 3];

        let hud_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("HUD Vertex Buffer"),
            contents: bytemuck::cast_slice(&hud_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let hud_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("HUD Index Buffer"),
            contents: bytemuck::cast_slice(&hud_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                label: Some("HUD Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment{
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations{
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.hud_pipeline);
            render_pass.set_bind_group(0, &self.score_bind_group, &[]);
            render_pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
            render_pass.set_index_buffer(hud_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..hud_indices.len() as u32, 0, 0..1);
        }

        if self.game_won {
            let win_w = 512.0;
            let win_h = 128.0;
            let win_x = (sw - win_w) / 2.0;
            let win_y = (sh - win_h) / 2.0;
            let tl = screen_to_clip(win_x, win_y, sw, sh);
            let tr = screen_to_clip(win_x + win_w, win_y, sw, sh);
            let br = screen_to_clip(win_x + win_w, win_y + win_h, sw, sh);
            let bl = screen_to_clip(win_x, win_y + win_h, sw, sh);
            let win_vertices = vec![
                Vertex { position: tl, uv: [0.0, 0.0] },
                Vertex { position: tr, uv: [1.0, 0.0] },
                Vertex { position: br, uv: [1.0, 1.0] },
                Vertex { position: bl, uv: [0.0, 1.0] },
            ];
            let win_indices = vec![0u16, 1, 2, 0, 2, 3];
            let win_vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Win VB"),
                contents: bytemuck::cast_slice(&win_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let win_ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Win IB"),
                contents: bytemuck::cast_slice(&win_indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Win Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.hud_pipeline);
            render_pass.set_bind_group(0, &self.win_bind_group, &[]);
            render_pass.set_vertex_buffer(0, win_vb.slice(..));
            render_pass.set_index_buffer(win_ib.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..win_indices.len() as u32, 0, 0..1);
        }

        if self.game_over {
            let go_w = 512.0;
            let go_h = 128.0;
            let go_x = (sw - go_w) / 2.0;
            let go_y = (sh - go_h) / 2.0 - 50.0;

            let tl = screen_to_clip(go_x, go_y, sw, sh);
            let tr = screen_to_clip(go_x + go_w, go_y, sw, sh);
            let br = screen_to_clip(go_x + go_w, go_y + go_h, sw, sh);
            let bl = screen_to_clip(go_x, go_y + go_h, sw, sh);
            let go_vertices = vec![
                Vertex { position: tl, uv: [0.0, 0.0] },
                Vertex { position: tr, uv: [1.0, 0.0] },
                Vertex { position: br, uv: [1.0, 1.0] },
                Vertex { position: bl, uv: [0.0, 1.0] },
            ];
            let go_indices = vec![0u16, 1, 2, 0, 2, 3];
            let go_vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GameOver VB"),
                contents: bytemuck::cast_slice(&go_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let go_ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GameOver IB"),
                contents: bytemuck::cast_slice(&go_indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GameOver Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.hud_pipeline);
            render_pass.set_bind_group(0, &self.gameover_bind_group, &[]);
            render_pass.set_vertex_buffer(0, go_vb.slice(..));
            render_pass.set_index_buffer(go_ib.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..go_indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn update_score_texture(&mut self){
        let font = Font::try_from_bytes(&self.font_bytes).expect("Failed to load font");
        let text = format!("Score: {}", self.score);
        let (img, width, height) = render_text_to_image(&text, &font, 28.0, 256, 64);

        let size = wgpu::Extent3d{
            width,
            height,
            depth_or_array_layers: 1,
        };
        self.queue.write_texture(
            wgpu::ImageCopyTexture{
                texture: &self.score_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
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