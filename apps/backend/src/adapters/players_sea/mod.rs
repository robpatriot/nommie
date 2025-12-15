//! SeaORM adapter for player repository - generic over ConnectionTrait.

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.
