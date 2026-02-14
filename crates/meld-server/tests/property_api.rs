use axum::{http::StatusCode, Json};
use meld_core::MeldError;
use meld_server::api::{
    map_domain_error_to_grpc, map_domain_error_to_rest, validation_error_with_source,
};
use proptest::prelude::*;
use validator::Validate;

#[derive(Debug, Clone, Validate)]
struct BoundaryDto {
    #[validate(length(min = 3, max = 12))]
    name: String,
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn validation_error_shape_is_stable_for_invalid_dto(input in "\\PC{0,2}") {
        let dto = BoundaryDto { name: input };
        let err = dto.validate().expect_err("expected boundary validation error");
        let (status, Json(body)) = validation_error_with_source(err, "body");

        prop_assert_eq!(status, StatusCode::BAD_REQUEST);
        prop_assert_eq!(body.code, "validation_error");

        let detail = body.detail.expect("validation detail should exist");
        prop_assert!(!detail.is_empty());

        for issue in detail {
            prop_assert_eq!(issue.loc.first().map(String::as_str), Some("body"));
            prop_assert_eq!(issue.loc.get(1).map(String::as_str), Some("name"));
            prop_assert!(!issue.msg.is_empty());
            prop_assert!(!issue.issue_type.is_empty());
        }
    }

    #[test]
    fn internal_error_mapping_is_sanitized(message in "(?s).{1,64}") {
        prop_assume!(message != "internal server error");

        let (status, Json(rest)) = map_domain_error_to_rest(MeldError::Internal(message.clone()));
        prop_assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        prop_assert_eq!(rest.code, "internal_error");
        prop_assert_eq!(rest.message.as_str(), "internal server error");
        prop_assert_ne!(rest.message.as_str(), message.as_str());

        let grpc = map_domain_error_to_grpc(MeldError::Internal(message));
        prop_assert_eq!(grpc.code(), tonic::Code::Internal);
        prop_assert_eq!(grpc.message(), "internal server error");
    }

    #[test]
    fn dto_length_boundary_matches_contract(input in "\\PC{0,16}") {
        let dto = BoundaryDto {
            name: input.clone(),
        };
        let is_valid = dto.validate().is_ok();
        let len = input.chars().count();

        prop_assert_eq!(is_valid, (3..=12).contains(&len));
    }

    #[test]
    fn validation_domain_error_mapping_preserves_bad_request(message in "(?s).{1,64}") {
        let (status, Json(rest)) = map_domain_error_to_rest(MeldError::Validation(message.clone()));
        prop_assert_eq!(status, StatusCode::BAD_REQUEST);
        prop_assert_eq!(rest.code, "validation_error");
        prop_assert!(!rest.message.is_empty());

        let grpc = map_domain_error_to_grpc(MeldError::Validation(message));
        prop_assert_eq!(grpc.code(), tonic::Code::InvalidArgument);
    }
}
