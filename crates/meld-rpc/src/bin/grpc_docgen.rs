use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
    process::Command,
};

use prost::Message;
use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
    FileDescriptorSet, MethodDescriptorProto, ServiceDescriptorProto,
};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
struct Config {
    proto: PathBuf,
    includes: Vec<PathBuf>,
    out_markdown: PathBuf,
    out_openapi: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proto: PathBuf::from("crates/meld-rpc/proto/service.proto"),
            includes: vec![PathBuf::from("crates/meld-rpc/proto")],
            out_markdown: PathBuf::from("docs/generated/grpc-contracts.md"),
            out_openapi: PathBuf::from("docs/generated/grpc-openapi-bridge.json"),
        }
    }
}

#[derive(Debug, Clone)]
struct DescriptorIndex {
    packages: BTreeMap<String, FileDescriptorProto>,
    messages: BTreeMap<String, DescriptorProto>,
    enums: BTreeMap<String, EnumDescriptorProto>,
    services: BTreeMap<String, ServiceDescriptorProto>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_args(std::env::args().skip(1))?;
    let descriptor = compile_descriptor_set(&config)?;
    let index = build_index(&descriptor);

    let markdown = build_markdown(&index);
    let openapi = build_openapi_bridge(&index);

    if let Some(parent) = config.out_markdown.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = config.out_openapi.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&config.out_markdown, markdown)?;
    fs::write(
        &config.out_openapi,
        serde_json::to_string_pretty(&openapi)? + "\n",
    )?;

    Ok(())
}

fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> Result<Config, Box<dyn std::error::Error>> {
    let mut cfg = Config::default();
    let mut args = args.into_iter().peekable();
    let mut includes_set = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--proto" => {
                let value = args.next().ok_or("missing value for --proto")?;
                cfg.proto = PathBuf::from(value);
            }
            "--include" => {
                let value = args.next().ok_or("missing value for --include")?;
                if !includes_set {
                    cfg.includes.clear();
                    includes_set = true;
                }
                cfg.includes.push(PathBuf::from(value));
            }
            "--out-md" => {
                let value = args.next().ok_or("missing value for --out-md")?;
                cfg.out_markdown = PathBuf::from(value);
            }
            "--out-openapi" => {
                let value = args.next().ok_or("missing value for --out-openapi")?;
                cfg.out_openapi = PathBuf::from(value);
            }
            "--help" | "-h" => {
                println!(
                    "grpc-docgen [--proto <path>] [--include <path>]... [--out-md <path>] [--out-openapi <path>]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    if cfg.includes.is_empty() {
        if let Some(parent) = cfg.proto.parent() {
            cfg.includes.push(parent.to_path_buf());
        }
    }

    Ok(cfg)
}

fn compile_descriptor_set(
    config: &Config,
) -> Result<FileDescriptorSet, Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let tmp = tempfile::NamedTempFile::new()?;

    let mut cmd = Command::new(protoc);
    cmd.arg(format!("--descriptor_set_out={}", tmp.path().display()))
        .arg("--include_imports")
        .arg("--include_source_info");

    for include in &config.includes {
        cmd.arg("-I").arg(include);
    }
    cmd.arg(&config.proto);

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("protoc failed: {stderr}").into());
    }

    let bytes = fs::read(tmp.path())?;
    Ok(FileDescriptorSet::decode(bytes.as_slice())?)
}

fn build_index(descriptor_set: &FileDescriptorSet) -> DescriptorIndex {
    let mut packages = BTreeMap::new();
    let mut messages = BTreeMap::new();
    let mut enums = BTreeMap::new();
    let mut services = BTreeMap::new();

    for file in &descriptor_set.file {
        let package = file.package.clone().unwrap_or_default();
        packages.insert(package.clone(), file.clone());

        for message in &file.message_type {
            collect_message_and_nested(&package, message, &mut messages, &mut enums);
        }

        for en in &file.enum_type {
            let enum_name = qualify(&package, en.name.as_deref().unwrap_or("UnknownEnum"));
            enums.insert(enum_name, en.clone());
        }

        for service in &file.service {
            let full = qualify(
                &package,
                service.name.as_deref().unwrap_or("UnknownService"),
            );
            services.insert(full, service.clone());
        }
    }

    DescriptorIndex {
        packages,
        messages,
        enums,
        services,
    }
}

fn collect_message_and_nested(
    prefix: &str,
    message: &DescriptorProto,
    messages: &mut BTreeMap<String, DescriptorProto>,
    enums: &mut BTreeMap<String, EnumDescriptorProto>,
) {
    let name = message.name.as_deref().unwrap_or("UnknownMessage");
    let full = qualify(prefix, name);
    messages.insert(full.clone(), message.clone());

    for en in &message.enum_type {
        let enum_name = qualify(&full, en.name.as_deref().unwrap_or("UnknownEnum"));
        enums.insert(enum_name, en.clone());
    }

    for nested in &message.nested_type {
        collect_message_and_nested(&full, nested, messages, enums);
    }
}

fn qualify(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

fn normalize_type_name(type_name: &str) -> String {
    type_name.trim_start_matches('.').to_string()
}

fn build_openapi_bridge(index: &DescriptorIndex) -> Value {
    let mut schemas = BTreeMap::<String, Value>::new();

    for (name, message) in &index.messages {
        if is_map_entry(message) {
            continue;
        }
        schemas.insert(name.clone(), message_schema(name, message, index));
    }

    for (name, en) in &index.enums {
        let values: Vec<Value> = en
            .value
            .iter()
            .filter_map(|v| v.name.as_ref().map(|n| Value::String(n.clone())))
            .collect();
        schemas.insert(
            name.clone(),
            json!({
                "type": "string",
                "enum": values,
            }),
        );
    }

    let mut paths = BTreeMap::<String, Value>::new();

    for (service_full_name, service) in &index.services {
        let mut service_parts: Vec<&str> = service_full_name.rsplitn(2, '.').collect();
        service_parts.reverse();
        let (package, service_name) = match service_parts.as_slice() {
            [package, name] => ((*package).to_string(), (*name).to_string()),
            [name] => (String::new(), (*name).to_string()),
            _ => (String::new(), service_full_name.clone()),
        };

        for method in &service.method {
            let path = grpc_path(&package, &service_name, method);
            let input = normalize_type_name(method.input_type.as_deref().unwrap_or(""));
            let output = normalize_type_name(method.output_type.as_deref().unwrap_or(""));
            let method_name = method.name.as_deref().unwrap_or("UnknownMethod");
            paths.insert(
                path,
                json!({
                    "post": {
                        "summary": format!("{service_name}.{method_name}"),
                        "description": "Swagger-compatible bridge for gRPC method contract discovery.",
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/grpc+proto": {
                                    "schema": {"$ref": format!("#/components/schemas/{input}")}
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "gRPC success response payload shape",
                                "content": {
                                    "application/grpc+proto": {
                                        "schema": {"$ref": format!("#/components/schemas/{output}")}
                                    }
                                }
                            }
                        },
                        "x-meld-grpc": {
                            "package": package,
                            "service": service_name,
                            "method": method_name,
                            "client_streaming": method.client_streaming.unwrap_or(false),
                            "server_streaming": method.server_streaming.unwrap_or(false)
                        }
                    }
                }),
            );
        }
    }

    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Meld gRPC Contract Bridge",
            "version": "0.1.0",
            "description": "Swagger-compatible contract view generated from protobuf descriptors."
        },
        "paths": paths,
        "components": {
            "schemas": schemas
        }
    })
}

fn grpc_path(package: &str, service_name: &str, method: &MethodDescriptorProto) -> String {
    let method_name = method.name.as_deref().unwrap_or("UnknownMethod");
    if package.is_empty() {
        format!("/{service_name}/{method_name}")
    } else {
        format!("/{package}.{service_name}/{method_name}")
    }
}

fn message_schema(name: &str, message: &DescriptorProto, index: &DescriptorIndex) -> Value {
    let oneof_groups: HashMap<i32, String> = message
        .oneof_decl
        .iter()
        .enumerate()
        .map(|(idx, oneof)| {
            (
                idx as i32,
                oneof.name.clone().unwrap_or_else(|| format!("oneof_{idx}")),
            )
        })
        .collect();

    let mut props = BTreeMap::<String, Value>::new();
    let mut required = Vec::<String>::new();
    let mut oneof_map = BTreeMap::<String, Vec<String>>::new();

    for field in &message.field {
        let field_name = field
            .name
            .clone()
            .unwrap_or_else(|| "unknown_field".to_string());

        if field.label == Some(Label::Required as i32) {
            required.push(field_name.clone());
        }

        if let Some(group_idx) = field.oneof_index {
            let group = oneof_groups
                .get(&group_idx)
                .cloned()
                .unwrap_or_else(|| format!("oneof_{group_idx}"));
            oneof_map.entry(group).or_default().push(field_name.clone());
        }

        props.insert(field_name, field_schema(field, index));
    }

    let mut schema = json!({
        "type": "object",
        "properties": props,
    });

    if !required.is_empty() {
        schema["required"] = json!(required);
    }

    if !oneof_map.is_empty() {
        schema["x-meld-oneof"] = json!(oneof_map);
    }

    if is_map_entry(message) {
        schema["x-meld-map-entry"] = json!(true);
    }

    schema["x-meld-message"] = json!(name);
    schema
}

fn field_schema(field: &FieldDescriptorProto, index: &DescriptorIndex) -> Value {
    let field_type = Type::try_from(field.r#type.unwrap_or_default()).unwrap_or(Type::String);
    let repeated = field.label == Some(Label::Repeated as i32);

    if repeated && field_type == Type::Message {
        if let Some(type_name) = field.type_name.as_deref() {
            let message_name = normalize_type_name(type_name);
            if let Some(msg) = index.messages.get(&message_name) {
                if is_map_entry(msg) {
                    return map_field_schema(msg, index);
                }
            }
        }
    }

    let base = base_schema(field, index);
    if repeated {
        json!({"type": "array", "items": base})
    } else {
        base
    }
}

fn map_field_schema(map_entry: &DescriptorProto, index: &DescriptorIndex) -> Value {
    let value_field = map_entry
        .field
        .iter()
        .find(|f| f.name.as_deref() == Some("value"));

    let value_schema = if let Some(v) = value_field {
        base_schema(v, index)
    } else {
        json!({"type": "object"})
    };

    json!({
        "type": "object",
        "additionalProperties": value_schema,
        "x-meld-map": true
    })
}

fn base_schema(field: &FieldDescriptorProto, index: &DescriptorIndex) -> Value {
    let field_type = Type::try_from(field.r#type.unwrap_or_default()).unwrap_or(Type::String);
    match field_type {
        Type::Double => json!({"type": "number", "format": "double"}),
        Type::Float => json!({"type": "number", "format": "float"}),
        Type::Int64 | Type::Sint64 | Type::Sfixed64 => {
            json!({"type": "integer", "format": "int64"})
        }
        Type::Uint64 | Type::Fixed64 => json!({"type": "integer", "format": "uint64"}),
        Type::Int32 | Type::Sint32 | Type::Sfixed32 => {
            json!({"type": "integer", "format": "int32"})
        }
        Type::Uint32 | Type::Fixed32 => json!({"type": "integer", "format": "uint32"}),
        Type::Bool => json!({"type": "boolean"}),
        Type::String => json!({"type": "string"}),
        Type::Bytes => json!({"type": "string", "format": "byte"}),
        Type::Enum => {
            let enum_name = normalize_type_name(field.type_name.as_deref().unwrap_or(""));
            if let Some(en) = index.enums.get(&enum_name) {
                let values: Vec<String> = en.value.iter().filter_map(|v| v.name.clone()).collect();
                json!({"type": "string", "enum": values})
            } else {
                json!({"type": "string"})
            }
        }
        Type::Message => {
            let type_name = normalize_type_name(field.type_name.as_deref().unwrap_or(""));
            json!({"$ref": format!("#/components/schemas/{type_name}")})
        }
        _ => json!({"type": "object"}),
    }
}

fn is_map_entry(message: &DescriptorProto) -> bool {
    message
        .options
        .as_ref()
        .and_then(|opt| opt.map_entry)
        .unwrap_or(false)
}

fn build_markdown(index: &DescriptorIndex) -> String {
    let mut lines = vec![
        "# gRPC Contract Documentation".to_string(),
        String::new(),
        "Generated from protobuf descriptor set (descriptor-based parser).".to_string(),
        String::new(),
        "## Packages".to_string(),
        String::new(),
    ];
    for pkg in index.packages.keys() {
        lines.push(format!(
            "- `{}`",
            if pkg.is_empty() { "<root>" } else { pkg }
        ));
    }
    lines.push(String::new());

    lines.push("## Services And Methods".to_string());
    lines.push(String::new());
    for (service_name, service) in &index.services {
        lines.push(format!("### `{service_name}`"));
        lines.push(String::new());
        for method in &service.method {
            let method_name = method.name.as_deref().unwrap_or("UnknownMethod");
            let input = normalize_type_name(method.input_type.as_deref().unwrap_or(""));
            let output = normalize_type_name(method.output_type.as_deref().unwrap_or(""));
            let stream = format!(
                "client_streaming={}, server_streaming={}",
                method.client_streaming.unwrap_or(false),
                method.server_streaming.unwrap_or(false)
            );
            lines.push(format!(
                "- `{method_name}`: `{input}` -> `{output}` ({stream})"
            ));
        }
        lines.push(String::new());
    }

    lines.push("## Messages".to_string());
    lines.push(String::new());
    for (message_name, message) in &index.messages {
        if is_map_entry(message) {
            continue;
        }
        lines.push(format!("### `{message_name}`"));
        lines.push(String::new());

        if message.field.is_empty() {
            lines.push("- (no fields)".to_string());
        }

        for field in &message.field {
            let name = field.name.as_deref().unwrap_or("unknown_field");
            let type_name = readable_field_type(field);
            let number = field.number.unwrap_or_default();
            let repeated = field.label == Some(Label::Repeated as i32);
            let repeated_flag = if repeated { ", repeated" } else { "" };
            lines.push(format!(
                "- `{name}` (`{type_name}`, field #{number}{repeated_flag})"
            ));
        }

        if !message.oneof_decl.is_empty() {
            lines.push(String::new());
            lines.push("Oneof groups:".to_string());
            for (idx, oneof) in message.oneof_decl.iter().enumerate() {
                let group_name = oneof.name.clone().unwrap_or_else(|| format!("oneof_{idx}"));
                let fields: Vec<String> = message
                    .field
                    .iter()
                    .filter(|f| f.oneof_index == Some(idx as i32))
                    .filter_map(|f| f.name.clone())
                    .collect();
                lines.push(format!("- `{group_name}`: {}", fields.join(", ")));
            }
        }

        lines.push(String::new());
    }

    lines.push("## Enums".to_string());
    lines.push(String::new());
    for (enum_name, en) in &index.enums {
        let values: Vec<String> = en.value.iter().filter_map(|v| v.name.clone()).collect();
        lines.push(format!("- `{enum_name}`: {}", values.join(", ")));
    }
    lines.push(String::new());

    lines.push("## gRPC Error Model".to_string());
    lines.push(String::new());
    lines.push("Common status codes exposed by the runtime:".to_string());
    lines.push(String::new());
    lines.push("- `INVALID_ARGUMENT` (3): validation failures".to_string());
    lines.push("- `INTERNAL` (13): unexpected server failures".to_string());
    lines.push(String::new());

    lines.push("## Artifacts".to_string());
    lines.push(String::new());
    lines.push("- Markdown: `docs/generated/grpc-contracts.md`".to_string());
    lines.push("- OpenAPI bridge: `docs/generated/grpc-openapi-bridge.json`".to_string());
    lines.push(String::new());

    lines.join("\n")
}

fn readable_field_type(field: &FieldDescriptorProto) -> String {
    let field_type = Type::try_from(field.r#type.unwrap_or_default()).unwrap_or(Type::String);
    match field_type {
        Type::Message | Type::Enum => normalize_type_name(field.type_name.as_deref().unwrap_or("")),
        Type::Double => "double".to_string(),
        Type::Float => "float".to_string(),
        Type::Int64 => "int64".to_string(),
        Type::Uint64 => "uint64".to_string(),
        Type::Int32 => "int32".to_string(),
        Type::Fixed64 => "fixed64".to_string(),
        Type::Fixed32 => "fixed32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Group => "group".to_string(),
        Type::Bytes => "bytes".to_string(),
        Type::Uint32 => "uint32".to_string(),
        Type::Sfixed32 => "sfixed32".to_string(),
        Type::Sfixed64 => "sfixed64".to_string(),
        Type::Sint32 => "sint32".to_string(),
        Type::Sint64 => "sint64".to_string(),
    }
}
