use std::any::Any;

pub trait Component: Sized + Sync + Send + Any {}
