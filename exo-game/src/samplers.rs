use wgpu::{AddressMode, Device, FilterMode, Sampler, SamplerDescriptor};

pub struct Samplers {
    pub linear_clamp: Sampler,
    pub nearest_clamp: Sampler, // Great for pixel art
                                //pub linear_repeat: Sampler, // Great for tiled floors/walls
                                //pub shadow_map: Sampler,    // Special comparison sampler
}

impl Samplers {
    pub fn new(device: &Device) -> Self {
        Self {
            linear_clamp: device.create_sampler(&SamplerDescriptor {
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                address_mode_u: AddressMode::ClampToEdge,
                ..Default::default()
            }),
            nearest_clamp: device.create_sampler(&SamplerDescriptor {
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                ..Default::default()
            }),
        }
    }
}
