/// Errors that can occur during low-level memory layout, entity management,
/// or structural graph transitions within the ECS engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcsError {
    /// The maximum number of distinct component types registered globally or
    /// per archetype signature has been exceeded.
    ComponentLimitExceeded,

    /// The requested entity has been destroyed or does not exist within the current location registry.
    EntityAlreadyDead,

    /// The system requested an operational reference to an archetype signature
    /// that does not exist in the world graph.
    UnknownArchetype,

    /// The generated or applied component bitmask does not correspond to a valid
    /// layout or is structurally malformed.
    InvalidMask,

    /// The request failed because the target layout exceeds architectural hardware dimensions.
    ComponentTooLarge,

    /// The specified component combination cannot fit into the chunk alignment requirements.
    LayoutCalculationFailed,

    /// Attempted to write an entity payload into a completely filled memory chunk.
    ChunkIsFull,

    /// The specified component column layout array indexes out of organizational bounds.
    ColumnIndexOutOfBounds,

    /// The entity index provided exceeds the initialized length of the active chunk elements.
    EntityIndexOutOfBounds,

    /// The entity already possesses an active instance of this component type,
    /// violating unique signature constraints.
    DuplicateComponent,

    /// The requested component type was not found in the targeted entity's archetype layout.
    ComponentNotFound,

    /// An unrecoverable internal safety guardrail failed, indicating a bug in
    /// raw memory allocation or routing logic.
    InternalError,
}
