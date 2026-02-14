use axum::Json;
use openportio_server::api::RequestValidation;
use openportio_server::utoipa::OpenApi;
use serde_json::Value;

#[openportio_server::dto]
struct LegacyDto {
    #[validate(length(min = 2, max = 120))]
    title: String,
}

#[derive(
    openportio_server::serde::Deserialize,
    openportio_server::OpenPortIOValidate,
    openportio_server::OpenPortIOSchema,
)]
struct ComposableDto {
    #[validate(length(min = 2, max = 120))]
    title: String,
}

#[derive(
    openportio_server::serde::Deserialize,
    openportio_server::MeldValidate,
    openportio_server::MeldSchema,
)]
struct MeldAliasDto {
    #[validate(length(min = 2, max = 120))]
    title: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(LegacyDto, ComposableDto, MeldAliasDto)))]
struct DtoModesApiDoc;

#[test]
fn dto_macro_and_composable_derives_match_validation_contract() {
    let legacy = LegacyDto {
        title: "x".to_string(),
    };
    let composable = ComposableDto {
        title: "x".to_string(),
    };
    let meld_alias = MeldAliasDto {
        title: "x".to_string(),
    };

    let (legacy_status, Json(legacy_body)) = legacy
        .validate_request("body")
        .expect_err("legacy dto should fail validation");
    let (composable_status, Json(composable_body)) = composable
        .validate_request("body")
        .expect_err("composable dto should fail validation");
    let (meld_alias_status, Json(meld_alias_body)) = meld_alias
        .validate_request("body")
        .expect_err("meld alias dto should fail validation");

    assert_eq!(legacy_status, composable_status);
    assert_eq!(legacy_status, meld_alias_status);
    assert_eq!(legacy_body.code, composable_body.code);
    assert_eq!(legacy_body.code, meld_alias_body.code);
    assert_eq!(legacy_body.message, composable_body.message);
    assert_eq!(legacy_body.message, meld_alias_body.message);

    let legacy_detail = legacy_body.detail.expect("legacy detail");
    let composable_detail = composable_body.detail.expect("composable detail");
    let meld_alias_detail = meld_alias_body.detail.expect("meld alias detail");
    assert_eq!(legacy_detail.len(), composable_detail.len());
    assert_eq!(legacy_detail.len(), meld_alias_detail.len());
    assert_eq!(legacy_detail[0].loc, composable_detail[0].loc);
    assert_eq!(legacy_detail[0].loc, meld_alias_detail[0].loc);
}

#[test]
fn dto_macro_and_composable_derives_match_schema_shape() {
    let components = DtoModesApiDoc::openapi()
        .components
        .expect("components should be generated");
    let mut legacy_schema = serde_json::to_value(
        components
            .schemas
            .get("LegacyDto")
            .expect("legacy schema should exist"),
    )
    .expect("legacy schema value");
    let mut composable_schema = serde_json::to_value(
        components
            .schemas
            .get("ComposableDto")
            .expect("composable schema should exist"),
    )
    .expect("composable schema value");
    let mut meld_alias_schema = serde_json::to_value(
        components
            .schemas
            .get("MeldAliasDto")
            .expect("meld alias schema should exist"),
    )
    .expect("meld alias schema value");

    strip_titles(&mut legacy_schema);
    strip_titles(&mut composable_schema);
    strip_titles(&mut meld_alias_schema);

    assert_eq!(legacy_schema, composable_schema);
    assert_eq!(legacy_schema, meld_alias_schema);
}

fn strip_titles(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("title");
            for nested in map.values_mut() {
                strip_titles(nested);
            }
        }
        Value::Array(items) => {
            for nested in items {
                strip_titles(nested);
            }
        }
        _ => {}
    }
}
