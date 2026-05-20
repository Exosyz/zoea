use std::any::Any;

pub struct EntityId(pub usize);

pub trait Entity: Sync + Send + Any {}