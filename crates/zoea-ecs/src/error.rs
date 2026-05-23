#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcsError {
    ComponentLimitExceeded,
    EntityAlreadyDead,
    UnknownArchetype,
    InvalidMask,
    /// The request failed because the target layout exceeds architectural hardware dimensions.
    ComponentTooLarge,
    /// The specified component combination cannot fit into the 16 KB alignment requirements.
    LayoutCalculationFailed,
    /// Attempted to write an entity payload into a completely filled memory chunk.
    ChunkIsFull,
    /// The specified component column layout array indexes out of organizational bounds.
    ColumnIndexOutOfBounds,
    /// The entity index provided exceeds the initialized length of the active chunk elements.
    EntityIndexOutOfBounds,
}
