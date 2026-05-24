use std::any::Any;

const GPU_ALIGNMENT: usize = 256;

pub trait Component: Sized + Sync + Send + Any {
    const TARGET_GPU: bool = false;

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
