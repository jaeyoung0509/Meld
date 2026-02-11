use std::process::Command;

#[test]
fn descriptor_docgen_supports_complex_protobuf_features() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out_md = tmp.path().join("grpc-contracts.md");
    let out_openapi = tmp.path().join("grpc-openapi-bridge.json");

    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let proto_path = fixture_dir.join("complex.proto");

    let status = Command::new(env!("CARGO_BIN_EXE_grpc-docgen"))
        .arg("--proto")
        .arg(proto_path)
        .arg("--include")
        .arg(&fixture_dir)
        .arg("--out-md")
        .arg(&out_md)
        .arg("--out-openapi")
        .arg(&out_openapi)
        .status()
        .expect("run grpc-docgen");

    assert!(
        status.success(),
        "grpc-docgen must succeed for fixture proto"
    );

    let md = std::fs::read_to_string(&out_md).expect("read markdown");
    assert!(
        md.contains("Oneof groups:"),
        "markdown should document oneof groups"
    );
    assert!(
        md.contains("fixture.common.v1.CommonMeta"),
        "markdown should include imported message types"
    );
    assert!(
        md.contains("fixture.docs.v1.GetDocRequest.NestedInfo.Scope"),
        "markdown should include nested enums"
    );

    let json = std::fs::read_to_string(&out_openapi).expect("read openapi bridge");
    assert!(
        json.contains("/fixture.docs.v1.DocsService/GetDoc"),
        "openapi bridge must include grpc method path"
    );
    assert!(
        json.contains("x-meld-oneof"),
        "openapi bridge should expose oneof group metadata"
    );
    assert!(
        json.contains("x-meld-map"),
        "openapi bridge should mark map fields"
    );
    assert!(
        json.contains("fixture.common.v1.CommonMeta"),
        "openapi bridge should reference imported message schemas"
    );
}
