//! `doctor`'s input, built from outside the crate as a caller builds it:
//! with `default()`, since it is `#[non_exhaustive]` (ADR-0039 kind 4,
//! ticket 2095).

use shuyi_cli::contract::doctor::DoctorInput;

#[test]
fn doctors_input_is_built_with_default_and_reads_as_the_empty_object() {
    let input = DoctorInput::default();
    assert_eq!(serde_json::to_string(&input).unwrap(), "{}");
    assert_eq!(serde_json::from_str::<DoctorInput>("{}").unwrap(), input);
    assert!(serde_json::from_str::<DoctorInput>(r#"{"signing":true}"#).is_err());
}
