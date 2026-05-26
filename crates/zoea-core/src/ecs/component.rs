use std::any::Any;

const GPU_ALIGNMENT: usize = 256;

/// A trait representing a component that can be processed by the system.
///
/// Components must be thread-safe (`Send` + `Sync`), have a known size at compile time (`Sized`),
/// and support dynamic downcasting (`Any`).
pub trait Component: Sized + Sync + Send + Any {
    /// Determines whether this component is processed on the GPU.
    ///
    /// # Important Note on Layout & Alignment
    /// **CRITICAL:** If you set `TARGET_GPU = true`, you **MUST** annotate your struct
    /// with `#[repr(C)]` (or `#[repr(transparent)]` where applicable).
    /// Standard Rust layout (`#[repr(Rust)]`) does not guarantee field ordering or alignment,
    /// which will cause severe data corruption or misaligned reads when transferred to the GPU.
    ///
    /// **GPU Padding Requirement:** Ensure that your struct fields mirror the alignment rules
    /// of your target shading language (e.g., WGSL's `std430` or `std140` layouts). You may
    /// need to insert explicit **padding fields** (e.g., `_pad: u32` or `_pad: [u8; 12]`)
    /// to ensure fields align correctly with GPU expectations.
    const TARGET_GPU: bool = false;

    /// The calculated alignment requirement for the component, automatically adjusting
    /// based on whether the CPU or GPU is the target.
    const ALIGNMENT: usize = {
        let base = align_of::<Self>();
        let gpu_align = if base > GPU_ALIGNMENT {
            base
        } else {
            GPU_ALIGNMENT
        };

        // Put both the CPU and GPU choices into a const array
        //    Index 0 (false) = CPU alignment
        //    Index 1 (true)  = GPU alignment
        let choices = [base, gpu_align];

        choices[Self::TARGET_GPU as usize]
    };
}
