use super::*;

#[test]
fn code_returns_discriminant_value() {
    assert_eq!(Status::Continue.code(), 100);
    assert_eq!(Status::OK.code(), 200);
    assert_eq!(Status::NotFound.code(), 404);
    assert_eq!(Status::InternalServerError.code(), 500);
}

#[test]
fn bytes_are_ascii_code_string() {
    assert_eq!(Status::OK.bytes(), b"200");
}

#[test]
fn try_from_u16_known_codes() {
    assert_eq!(Status::try_from(200u16), Ok(Status::OK));
    assert_eq!(Status::try_from(404u16), Ok(Status::NotFound));
    assert_eq!(Status::try_from(500u16), Ok(Status::InternalServerError));
}

#[test]
fn try_from_u16_out_of_range_is_err() {
    assert_eq!(Status::try_from(0u16), Err(()));
    assert_eq!(Status::try_from(999u16), Err(()));
}

#[test]
fn from_status_ok_to_u16() {
    assert_eq!(u16::from(Status::OK), 200u16);
}

#[test]
fn from_status_not_found_to_u32() {
    assert_eq!(u32::from(Status::NotFound), 404u32);
}

#[test]
fn try_from_i32_negative_is_err() {
    assert_eq!(Status::try_from(-1i32), Err(()));
}

#[test]
fn ok_is_less_than_not_found() {
    assert!(Status::OK < Status::NotFound);
}

#[test]
fn informational_codes_in_range() {
    assert!((100..=199).contains(&Status::Continue.code()));
    assert!((100..=199).contains(&Status::EarlyHints.code()));
}

#[test]
fn display_uses_variant_name() {
    assert_eq!(Status::OK.to_string(), "OK");
}

#[test]
fn default_is_internal_server_error() {
    assert_eq!(Status::default().code(), 500);
}
