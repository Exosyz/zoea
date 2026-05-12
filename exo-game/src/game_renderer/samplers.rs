use wgpu::{AddressMode, CompareFunction, Device, FilterMode, MipmapFilterMode, Sampler, SamplerDescriptor};

pub struct Samplers {
    /// Smooth filtering, stops at the edge. Perfect for UI and high-res sprites.
    pub linear_clamp: Sampler,
    /// Pixelated filtering, stops at the edge. The "Gold Standard" for Pixel Art.
    pub nearest_clamp: Sampler,
    /// Smooth filtering, tiles infinitely. Use this for scrolling backgrounds or terrain.
    pub linear_repeat: Sampler,
    /// Pixelated filtering, tiles infinitely. Perfect for retro tiled floors/walls.
    pub nearest_repeat: Sampler,
    /// Special sampler for Shadow Mapping or Depth comparisons.
    pub shadow_comparison: Sampler,
}

impl Samplers {
    pub fn new(device: &Device) -> Self {
        Self {
            linear_clamp: device.create_sampler(&SamplerDescriptor {
                label: Some("Linear Clamp Sampler"),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: MipmapFilterMode::Linear,
                ..Default::default()
            }),

            nearest_clamp: device.create_sampler(&SamplerDescriptor {
                label: Some("Nearest Clamp Sampler"),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: MipmapFilterMode::Nearest,
                ..Default::default()
            }),

            linear_repeat: device.create_sampler(&SamplerDescriptor {
                label: Some("Linear Repeat Sampler"),
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..Default::default()
            }),

            nearest_repeat: device.create_sampler(&SamplerDescriptor {
                label: Some("Nearest Repeat Sampler"),
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                ..Default::default()
            }),

            shadow_comparison: device.create_sampler(&SamplerDescriptor {
                label: Some("Shadow Comparison Sampler"),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                compare: Some(CompareFunction::LessEqual),
                ..Default::default()
            }),
        }
    }
}