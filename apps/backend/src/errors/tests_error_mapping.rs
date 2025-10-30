// Unit tests for error mapping - pure domain logic without HTTP or database dependencies
use crate::errors::domain::{
    ConflictKind, DomainError, InfraErrorKind, NotFoundKind, ValidationKind,
};
use crate::{AppError, ErrorCode};

#[test]
fn maps_validation_to_422() {
    let de = DomainError::validation(
        ValidationKind::Other("VALIDATION_ERROR".into()),
        "bad field",
    );
    let app: AppError = de.into();
    assert_eq!(app.code(), ErrorCode::ValidationError);
    assert_eq!(app.status().as_u16(), 422);
}

#[test]
fn maps_conflicts() {
    let seat = DomainError::conflict(ConflictKind::SeatTaken, "seat taken");
    let app: AppError = seat.into();
    assert_eq!(app.code().as_str(), "SEAT_TAKEN");
    assert_eq!(app.status().as_u16(), 409);

    let unique = DomainError::conflict(ConflictKind::UniqueEmail, "email exists");
    let app: AppError = unique.into();
    assert_eq!(app.code().as_str(), "UNIQUE_EMAIL");
    assert_eq!(app.status().as_u16(), 409);

    // Test generic conflict fallback
    let other = DomainError::conflict(
        ConflictKind::Other("some conflict".to_string()),
        "generic conflict",
    );
    let app: AppError = other.into();
    assert_eq!(app.code().as_str(), "CONFLICT");
    assert_eq!(app.status().as_u16(), 409);
}

#[test]
fn maps_not_found() {
    let nf = DomainError::not_found(NotFoundKind::User, "no user");
    let app: AppError = nf.into();
    assert_eq!(app.code().as_str(), "USER_NOT_FOUND");
    assert_eq!(app.status().as_u16(), 404);
}

#[test]
fn maps_infra() {
    let t = DomainError::infra(InfraErrorKind::Timeout, "timeout");
    let app: AppError = t.into();
    assert_eq!(app.code().as_str(), "DB_TIMEOUT");
    assert_eq!(app.status().as_u16(), 504);
    // Verify it's a Timeout AppError, not Validation
    assert!(matches!(app, AppError::Timeout { .. }));

    let down = DomainError::infra(InfraErrorKind::DbUnavailable, "down");
    let app: AppError = down.into();
    assert_eq!(app.code().as_str(), "DB_UNAVAILABLE");
    assert_eq!(app.status().as_u16(), 503);

    let corr = DomainError::infra(InfraErrorKind::DataCorruption, "bad");
    let app: AppError = corr.into();
    assert_eq!(app.code().as_str(), "DATA_CORRUPTION");
    assert_eq!(app.status().as_u16(), 500);

    let other = DomainError::infra(InfraErrorKind::Other("unknown".to_string()), "other");
    let app: AppError = other.into();
    assert_eq!(app.code().as_str(), "INTERNAL_ERROR");
    assert_eq!(app.status().as_u16(), 500);
}

#[test]
fn domain_purity_check() {
    // This test verifies that domain modules can be used without HTTP/SeaORM imports
    // by creating DomainError instances and converting them to AppError
    use crate::errors::domain::{
        ConflictKind, DomainError, InfraErrorKind, NotFoundKind, ValidationKind,
    };
    use crate::AppError;

    // Test that we can create domain errors without HTTP imports
    let validation =
        DomainError::validation(ValidationKind::Other("VALIDATION_ERROR".into()), "test");
    let conflict = DomainError::conflict(ConflictKind::SeatTaken, "test");
    let not_found = DomainError::not_found(NotFoundKind::User, "test");
    let infra = DomainError::infra(InfraErrorKind::Timeout, "test");

    // Test that conversion to AppError works (this happens in the error module)
    let _: AppError = validation.into();
    let _: AppError = conflict.into();
    let _: AppError = not_found.into();
    let _: AppError = infra.into();
}

#[test]
fn constructor_helpers() {
    // Test validation constructor
    let validation = DomainError::validation(
        ValidationKind::Other("VALIDATION_ERROR".into()),
        "invalid input",
    );
    assert!(matches!(validation, DomainError::Validation(_, _)));

    // Test conflict constructor
    let conflict = DomainError::conflict(ConflictKind::SeatTaken, "seat taken");
    assert!(matches!(
        conflict,
        DomainError::Conflict(ConflictKind::SeatTaken, _)
    ));

    // Test not found constructor
    let not_found = DomainError::not_found(NotFoundKind::User, "user missing");
    assert!(matches!(
        not_found,
        DomainError::NotFound(NotFoundKind::User, _)
    ));

    // Test infra constructor
    let infra = DomainError::infra(InfraErrorKind::Timeout, "timeout");
    assert!(matches!(
        infra,
        DomainError::Infra(InfraErrorKind::Timeout, _)
    ));
}
