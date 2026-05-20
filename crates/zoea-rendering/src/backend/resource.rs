use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    Device, Queue, ShaderStages,
};

pub enum GpuResourceMode {
    Storage(usize),
    Uniform,
}

pub struct GpuResource<T> {
    pub buffer: Buffer,
    pub layout: BindGroupLayout,
    pub bind_group: BindGroup,
    mode: GpuResourceMode,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod> GpuResource<T> {
    pub fn new(
        device: Arc<Device>,
        label: &str,
        binding: u32,
        visibility: ShaderStages,
        mode: GpuResourceMode,
    ) -> Self {
        // 1. Layout
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&format!("{} Layout", label)),
            entries: &[BindGroupLayoutEntry {
                binding,
                visibility,
                ty: BindingType::Buffer {
                    ty: match mode {
                        GpuResourceMode::Uniform => BufferBindingType::Uniform,
                        GpuResourceMode::Storage(_) => {
                            BufferBindingType::Storage { read_only: true }
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // 2. Buffer

        let size = match mode {
            GpuResourceMode::Uniform => size_of::<T>() as u64,
            GpuResourceMode::Storage(count) => (size_of::<T>() * count) as u64,
        };
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("{} Buffer", label)),
            size,
            usage: match mode {
                GpuResourceMode::Uniform => BufferUsages::UNIFORM,
                GpuResourceMode::Storage(_) => BufferUsages::STORAGE,
            } | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 3. Bind Group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &layout,
            entries: &[BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }],
            label: Some(&format!("{} Bind Group", label)),
        });

        Self {
            buffer,
            layout,
            bind_group,
            mode,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn write_single(&self, queue: &Queue, data: &T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }

    pub fn write_slice(&self, queue: &Queue, data: &[T]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }
}
