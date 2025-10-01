//! Domain-level error type used across services and adapters.
//!
//! This error type is HTTP- and DB-agnostic. Handlers should return
//! `Result<T, crate::error::AppError>` and convert from `DomainError`
//! using the provided `From<DomainError> for AppError` implementation.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Infra error kinds to distinguish operational failures
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InfraErrorKind {
    Timeout,
    DbUnavailable,
    DataCorruption,
    Other(String),
}

/// Domain-level not found entities (minimal set; extend as needed)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotFoundKind {
    User,
    Game,
    Other(String),
}

/// Domain-level conflict kinds (extend as needed)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictKind {
    SeatTaken,
    UniqueEmail,
    OptimisticLock,
    JoinCodeConflict,
    Other(String),
}

/// Central domain error type
#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    /// Input/user validation or business rule violation
    Validation(String),
    /// Semantic conflict
    Conflict(ConflictKind, String),
    /// Missing resource in domain terms
    NotFound(NotFoundKind, String),
    /// Infrastructure/operational failures
    Infra(InfraErrorKind, String),
}

impl Display for DomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DomainError::Validation(d) => write!(f, "validation error: {d}"),
            DomainError::Conflict(kind, d) => write!(f, "conflict {kind:?}: {d}"),
            DomainError::NotFound(kind, d) => write!(f, "not found {kind:?}: {d}"),
            DomainError::Infra(kind, d) => write!(f, "infra {kind:?}: {d}"),
        }
    }
}

impl Error for DomainError {}

impl DomainError {
    pub fn validation(detail: impl Into<String>) -> Self {
        Self::Validation(detail.into())
    }
    pub fn conflict(kind: ConflictKind, detail: impl Into<String>) -> Self {
        Self::Conflict(kind, detail.into())
    }
    pub fn not_found(kind: NotFoundKind, detail: impl Into<String>) -> Self {
        Self::NotFound(kind, detail.into())
    }
    pub fn infra(kind: InfraErrorKind, detail: impl Into<String>) -> Self {
        Self::Infra(kind, detail.into())
    }
}
