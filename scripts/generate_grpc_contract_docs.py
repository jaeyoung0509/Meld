#!/usr/bin/env python3
import json
import re
from pathlib import Path


PROTO_PATH = Path("crates/alloy-rpc/proto/service.proto")
OUT_MD = Path("docs/generated/grpc-contracts.md")
OUT_OPENAPI = Path("docs/generated/grpc-openapi-bridge.json")


def parse_proto(text: str):
    package_match = re.search(r"^\s*package\s+([A-Za-z0-9_.]+)\s*;", text, re.MULTILINE)
    package = package_match.group(1) if package_match else "unknown"

    services = []
    for svc_match in re.finditer(
        r"service\s+([A-Za-z0-9_]+)\s*\{(.*?)\}", text, re.DOTALL | re.MULTILINE
    ):
        name = svc_match.group(1)
        body = svc_match.group(2)
        methods = []
        for rpc_match in re.finditer(
            r"rpc\s+([A-Za-z0-9_]+)\s*\(\s*([A-Za-z0-9_]+)\s*\)\s*returns\s*\(\s*([A-Za-z0-9_]+)\s*\)\s*;",
            body,
        ):
            methods.append(
                {
                    "name": rpc_match.group(1),
                    "request": rpc_match.group(2),
                    "response": rpc_match.group(3),
                }
            )
        services.append({"name": name, "methods": methods})

    messages = {}
    for msg_match in re.finditer(
        r"message\s+([A-Za-z0-9_]+)\s*\{(.*?)\}", text, re.DOTALL | re.MULTILINE
    ):
        msg_name = msg_match.group(1)
        body = msg_match.group(2)
        fields = []
        for field_match in re.finditer(
            r"^\s*([A-Za-z0-9_.]+)\s+([A-Za-z0-9_]+)\s*=\s*([0-9]+)\s*;",
            body,
            re.MULTILINE,
        ):
            fields.append(
                {
                    "type": field_match.group(1),
                    "name": field_match.group(2),
                    "number": int(field_match.group(3)),
                }
            )
        messages[msg_name] = fields

    return package, services, messages


def to_openapi_schema_type(proto_type: str):
    mapping = {
        "string": ("string", None),
        "bool": ("boolean", None),
        "bytes": ("string", "byte"),
        "double": ("number", "double"),
        "float": ("number", "float"),
        "int32": ("integer", "int32"),
        "sint32": ("integer", "int32"),
        "sfixed32": ("integer", "int32"),
        "uint32": ("integer", "int32"),
        "fixed32": ("integer", "int32"),
        "int64": ("integer", "int64"),
        "sint64": ("integer", "int64"),
        "sfixed64": ("integer", "int64"),
        "uint64": ("integer", "int64"),
        "fixed64": ("integer", "int64"),
    }
    return mapping.get(proto_type, ("object", None))


def build_openapi(package, services, messages):
    paths = {}
    components = {"schemas": {}}

    for msg_name, fields in messages.items():
        props = {}
        required = []
        for field in fields:
            type_name, fmt = to_openapi_schema_type(field["type"])
            prop = {"type": type_name}
            if fmt:
                prop["format"] = fmt
            props[field["name"]] = prop
            required.append(field["name"])
        components["schemas"][f"{package}.{msg_name}"] = {
            "type": "object",
            "properties": props,
            "required": required,
        }

    for service in services:
        for method in service["methods"]:
            path = f"/{package}.{service['name']}/{method['name']}"
            req_ref = f"#/components/schemas/{package}.{method['request']}"
            res_ref = f"#/components/schemas/{package}.{method['response']}"
            paths[path] = {
                "post": {
                    "summary": f"{service['name']}.{method['name']}",
                    "description": "Swagger-compatible bridge for gRPC method contract discovery.",
                    "requestBody": {
                        "required": True,
                        "content": {"application/grpc+proto": {"schema": {"$ref": req_ref}}},
                    },
                    "responses": {
                        "200": {
                            "description": "gRPC success response payload shape",
                            "content": {
                                "application/grpc+proto": {"schema": {"$ref": res_ref}}
                            },
                        }
                    },
                    "x-alloy-grpc": {
                        "package": package,
                        "service": service["name"],
                        "method": method["name"],
                    },
                }
            }

    return {
        "openapi": "3.0.3",
        "info": {
            "title": "Alloy gRPC Contract Bridge",
            "version": "0.1.0",
            "description": "Swagger-compatible contract view generated from .proto for discovery.",
        },
        "paths": paths,
        "components": components,
    }


def build_markdown(package, services, messages):
    lines = []
    lines.append("# gRPC Contract Documentation")
    lines.append("")
    lines.append("Generated from `crates/alloy-rpc/proto/service.proto`.")
    lines.append("")
    lines.append("## Package")
    lines.append("")
    lines.append(f"- `{package}`")
    lines.append("")
    lines.append("## Services And Methods")
    lines.append("")
    for service in services:
        lines.append(f"### `{service['name']}`")
        lines.append("")
        for method in service["methods"]:
            lines.append(
                f"- `{method['name']}`: `{method['request']}` -> `{method['response']}`"
            )
        lines.append("")

    lines.append("## Messages")
    lines.append("")
    for msg_name, fields in messages.items():
        lines.append(f"### `{msg_name}`")
        lines.append("")
        if not fields:
            lines.append("- (no fields)")
        for field in fields:
            lines.append(
                f"- `{field['name']}` (`{field['type']}`, field #{field['number']})"
            )
        lines.append("")

    lines.append("## gRPC Error Model")
    lines.append("")
    lines.append("Common status codes exposed by the runtime:")
    lines.append("")
    lines.append("- `INVALID_ARGUMENT` (3): validation failures")
    lines.append("- `INTERNAL` (13): unexpected server failures")
    lines.append("")
    lines.append("## Artifacts")
    lines.append("")
    lines.append("- Markdown: `docs/generated/grpc-contracts.md`")
    lines.append("- OpenAPI bridge: `docs/generated/grpc-openapi-bridge.json`")
    lines.append("")
    return "\n".join(lines)


def main():
    text = PROTO_PATH.read_text(encoding="utf-8")
    package, services, messages = parse_proto(text)
    openapi_bridge = build_openapi(package, services, messages)
    markdown = build_markdown(package, services, messages)

    OUT_MD.parent.mkdir(parents=True, exist_ok=True)
    OUT_OPENAPI.parent.mkdir(parents=True, exist_ok=True)

    OUT_MD.write_text(markdown + "\n", encoding="utf-8")
    OUT_OPENAPI.write_text(
        json.dumps(openapi_bridge, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
