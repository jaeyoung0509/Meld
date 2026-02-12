#!/usr/bin/env python3
"""Generate a unified REST+gRPC contract bundle with explicit link validation."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ModuleNotFoundError:
        tomllib = None  # type: ignore[assignment]

HTTP_METHODS = {"get", "put", "post", "delete", "patch", "options", "head", "trace"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Generate docs/generated/contracts-bundle.json from REST OpenAPI, "
            "gRPC bridge OpenAPI, and contracts/links.toml."
        )
    )
    parser.add_argument(
        "--rest-openapi",
        default="docs/generated/rest-openapi.json",
        help="Path to REST OpenAPI JSON",
    )
    parser.add_argument(
        "--grpc-bridge",
        default="docs/generated/grpc-openapi-bridge.json",
        help="Path to gRPC OpenAPI bridge JSON",
    )
    parser.add_argument(
        "--links",
        default="contracts/links.toml",
        help="Path to explicit mapping file",
    )
    parser.add_argument(
        "--out",
        default="docs/generated/contracts-bundle.json",
        help="Output path for generated bundle JSON",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        payload = json.load(handle)
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must be a JSON object")
    return payload


def collect_rest_operations(openapi: dict, errors: list[str]) -> list[dict]:
    operations: list[dict] = []
    paths = openapi.get("paths")
    if not isinstance(paths, dict):
        errors.append("REST OpenAPI is missing object field: paths")
        return operations

    for path_key in sorted(paths):
        path_item = paths.get(path_key)
        if not isinstance(path_item, dict):
            errors.append(f"REST path item must be an object: {path_key}")
            continue

        for method_key in sorted(path_item):
            method_lower = method_key.lower()
            if method_lower not in HTTP_METHODS:
                continue
            operation = path_item.get(method_key)
            if not isinstance(operation, dict):
                errors.append(
                    f"REST operation must be an object: {method_key.upper()} {path_key}"
                )
                continue
            operation_id = operation.get("operationId")
            if not isinstance(operation_id, str) or not operation_id:
                errors.append(
                    f"REST operation is missing operationId: {method_key.upper()} {path_key}"
                )
                continue

            operations.append(
                {
                    "operation_id": operation_id,
                    "method": method_key.upper(),
                    "path": path_key,
                    "summary": operation.get("summary"),
                }
            )

    operations.sort(key=lambda item: item["operation_id"])
    return operations


def collect_grpc_methods(grpc_openapi: dict, errors: list[str]) -> list[dict]:
    methods: list[dict] = []
    paths = grpc_openapi.get("paths")
    if not isinstance(paths, dict):
        errors.append("gRPC bridge OpenAPI is missing object field: paths")
        return methods

    for path_key in sorted(paths):
        path_item = paths.get(path_key)
        if not isinstance(path_item, dict):
            errors.append(f"gRPC bridge path item must be an object: {path_key}")
            continue

        post = path_item.get("post")
        if not isinstance(post, dict):
            errors.append(f"gRPC bridge path must define post operation: {path_key}")
            continue

        grpc_method = path_key[1:] if path_key.startswith("/") else path_key
        request_schema = (
            post.get("requestBody", {})
            .get("content", {})
            .get("application/grpc+proto", {})
            .get("schema", {})
            .get("$ref")
        )
        response_schema = (
            post.get("responses", {})
            .get("200", {})
            .get("content", {})
            .get("application/grpc+proto", {})
            .get("schema", {})
            .get("$ref")
        )

        metadata = post.get("x-meld-grpc")
        if isinstance(metadata, dict):
            package = metadata.get("package")
            service = metadata.get("service")
            method = metadata.get("method")
            if (
                isinstance(package, str)
                and isinstance(service, str)
                and isinstance(method, str)
            ):
                from_metadata = f"{package}.{service}/{method}"
                if from_metadata != grpc_method:
                    errors.append(
                        "gRPC bridge method mismatch between path and x-meld-grpc metadata: "
                        f"path={grpc_method}, metadata={from_metadata}"
                    )

        methods.append(
            {
                "grpc_method": grpc_method,
                "http_method": "POST",
                "path": path_key,
                "summary": post.get("summary"),
                "request_schema_ref": request_schema,
                "response_schema_ref": response_schema,
            }
        )

    methods.sort(key=lambda item: item["grpc_method"])
    return methods


def parse_toml_value(raw: str, line_number: int, errors: list[str]) -> object:
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        if raw.isdigit():
            return int(raw)
        errors.append(f"links.toml line {line_number}: unsupported value: {raw}")
        return None


def parse_links_toml(links_path: Path, errors: list[str]) -> dict:
    root: dict = {}
    coverage: dict = {}
    links: list[dict] = []
    section = "root"
    current_link: dict | None = None

    with links_path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            stripped = line.split("#", 1)[0].strip()
            if not stripped:
                continue
            if stripped == "[coverage]":
                section = "coverage"
                current_link = None
                continue
            if stripped == "[[links]]":
                section = "links"
                current_link = {}
                links.append(current_link)
                continue
            if "=" not in stripped:
                errors.append(f"links.toml line {line_number}: expected key = value")
                continue

            key, value_raw = stripped.split("=", 1)
            key = key.strip()
            value_raw = value_raw.strip()
            value = parse_toml_value(value_raw, line_number, errors)

            if section == "coverage":
                coverage[key] = value
            elif section == "links":
                if current_link is None:
                    errors.append(
                        f"links.toml line {line_number}: key outside [[links]] block"
                    )
                    continue
                current_link[key] = value
            else:
                root[key] = value

    root["coverage"] = coverage
    root["links"] = links
    return root


def load_links(links_path: Path, errors: list[str]) -> tuple[dict, list[dict]]:
    raw: dict | None = None
    if tomllib is not None:
        with links_path.open("rb") as handle:
            parsed = tomllib.load(handle)
        if isinstance(parsed, dict):
            raw = parsed
        else:
            errors.append("links.toml root must be a TOML table")
            raw = {}
    else:
        # Fallback parser for environments without tomllib/tomli (keeps zero-deps local flow).
        raw = parse_links_toml(links_path, errors)

    coverage = raw.get("coverage", {})
    if not isinstance(coverage, dict):
        errors.append("[coverage] must be a table in links.toml")
        coverage = {}

    links = raw.get("links", [])
    if not isinstance(links, list):
        errors.append("links.toml [[links]] entries must be an array")
        links = []

    for idx, link in enumerate(links):
        if not isinstance(link, dict):
            errors.append(f"links[{idx}] must be a table")
            continue
        for key in ("rest_operation_id", "grpc_method"):
            value = link.get(key)
            if not isinstance(value, str) or not value:
                errors.append(f"links[{idx}] requires non-empty string: {key}")

    return coverage, links


def as_sorted_string_set(value: object, field_name: str, errors: list[str]) -> list[str]:
    if value is None:
        return []
    if not isinstance(value, list) or not all(
        isinstance(item, str) and item for item in value
    ):
        errors.append(f"{field_name} must be an array of non-empty strings")
        return []
    return sorted(set(value))


def sanitize_repo_relative_path(
    raw_path: str,
    repo_root: Path,
    *,
    arg_name: str,
    allowed_root: Path,
    must_exist: bool,
) -> Path:
    candidate = Path(raw_path)
    if str(candidate) == "":
        raise ValueError(f"{arg_name} path cannot be empty")
    if candidate.is_absolute():
        raise ValueError(f"{arg_name} must be a repo-relative path")
    if ".." in candidate.parts:
        raise ValueError(f"{arg_name} cannot contain parent-directory traversal ('..')")

    resolved = (repo_root / candidate).resolve(strict=False)
    repo_root_resolved = repo_root.resolve()
    allowed_root_resolved = allowed_root.resolve()

    try:
        resolved.relative_to(repo_root_resolved)
    except ValueError as err:
        raise ValueError(f"{arg_name} escapes repository root") from err

    try:
        resolved.relative_to(allowed_root_resolved)
    except ValueError as err:
        raise ValueError(
            f"{arg_name} must stay under {allowed_root_resolved.relative_to(repo_root_resolved)}"
        ) from err

    if must_exist and not resolved.is_file():
        raise ValueError(f"{arg_name} file does not exist: {resolved}")

    return resolved


def to_repo_relative(path: Path, repo_root: Path) -> str:
    return path.relative_to(repo_root).as_posix()


def main() -> int:
    args = parse_args()
    errors: list[str] = []

    repo_root = Path(__file__).resolve().parent.parent
    generated_root = repo_root / "docs/generated"
    contracts_root = repo_root / "contracts"

    try:
        rest_path = sanitize_repo_relative_path(
            args.rest_openapi,
            repo_root,
            arg_name="--rest-openapi",
            allowed_root=generated_root,
            must_exist=True,
        )
        grpc_path = sanitize_repo_relative_path(
            args.grpc_bridge,
            repo_root,
            arg_name="--grpc-bridge",
            allowed_root=generated_root,
            must_exist=True,
        )
        links_path = sanitize_repo_relative_path(
            args.links,
            repo_root,
            arg_name="--links",
            allowed_root=contracts_root,
            must_exist=True,
        )
        out_path = sanitize_repo_relative_path(
            args.out,
            repo_root,
            arg_name="--out",
            allowed_root=generated_root,
            must_exist=False,
        )
    except ValueError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2

    rest_openapi = load_json(rest_path)
    grpc_openapi = load_json(grpc_path)

    rest_operations = collect_rest_operations(rest_openapi, errors)
    grpc_methods = collect_grpc_methods(grpc_openapi, errors)
    coverage_cfg, raw_links = load_links(links_path, errors)

    rest_by_id = {item["operation_id"]: item for item in rest_operations}
    grpc_by_method = {item["grpc_method"]: item for item in grpc_methods}

    allow_unmapped_rest = as_sorted_string_set(
        coverage_cfg.get("allow_unmapped_rest_operation_ids"),
        "coverage.allow_unmapped_rest_operation_ids",
        errors,
    )
    allow_unmapped_grpc = as_sorted_string_set(
        coverage_cfg.get("allow_unmapped_grpc_methods"),
        "coverage.allow_unmapped_grpc_methods",
        errors,
    )

    for operation_id in allow_unmapped_rest:
        if operation_id not in rest_by_id:
            errors.append(
                "coverage.allow_unmapped_rest_operation_ids contains unknown operationId: "
                f"{operation_id}"
            )
    for grpc_method in allow_unmapped_grpc:
        if grpc_method not in grpc_by_method:
            errors.append(
                "coverage.allow_unmapped_grpc_methods contains unknown method: "
                f"{grpc_method}"
            )

    normalized_links: list[dict] = []
    seen_rest: set[str] = set()
    seen_grpc: set[str] = set()

    for index, link in enumerate(raw_links):
        if not isinstance(link, dict):
            continue
        operation_id = link.get("rest_operation_id")
        grpc_method = link.get("grpc_method")
        if not isinstance(operation_id, str) or not isinstance(grpc_method, str):
            continue

        if operation_id in seen_rest:
            errors.append(f"duplicate rest_operation_id mapping: {operation_id}")
            continue
        if grpc_method in seen_grpc:
            errors.append(f"duplicate grpc_method mapping: {grpc_method}")
            continue
        seen_rest.add(operation_id)
        seen_grpc.add(grpc_method)

        rest_operation = rest_by_id.get(operation_id)
        if rest_operation is None:
            errors.append(f"links[{index}] references unknown rest_operation_id: {operation_id}")
            continue
        grpc_operation = grpc_by_method.get(grpc_method)
        if grpc_operation is None:
            errors.append(f"links[{index}] references unknown grpc_method: {grpc_method}")
            continue

        declared_rest_method = link.get("rest_method")
        if declared_rest_method is not None:
            if (
                not isinstance(declared_rest_method, str)
                or declared_rest_method.upper() != rest_operation["method"]
            ):
                errors.append(
                    f"links[{index}] rest_method mismatch for "
                    f"{operation_id}: expected {rest_operation['method']}"
                )
        declared_rest_path = link.get("rest_path")
        if declared_rest_path is not None:
            if (
                not isinstance(declared_rest_path, str)
                or declared_rest_path != rest_operation["path"]
            ):
                errors.append(
                    f"links[{index}] rest_path mismatch for "
                    f"{operation_id}: expected {rest_operation['path']}"
                )

        normalized = {
            "rest_operation_id": operation_id,
            "rest_method": rest_operation["method"],
            "rest_path": rest_operation["path"],
            "grpc_method": grpc_method,
            "grpc_http_method": grpc_operation["http_method"],
            "grpc_path": grpc_operation["path"],
        }
        notes = link.get("notes")
        if isinstance(notes, str) and notes:
            normalized["notes"] = notes
        normalized_links.append(normalized)

    normalized_links.sort(key=lambda item: item["rest_operation_id"])
    linked_rest = {item["rest_operation_id"] for item in normalized_links}
    linked_grpc = {item["grpc_method"] for item in normalized_links}

    unmapped_rest = sorted(
        operation_id
        for operation_id in rest_by_id
        if operation_id not in linked_rest and operation_id not in allow_unmapped_rest
    )
    unmapped_grpc = sorted(
        grpc_method
        for grpc_method in grpc_by_method
        if grpc_method not in linked_grpc and grpc_method not in allow_unmapped_grpc
    )

    if unmapped_rest:
        errors.append(
            "missing REST mappings (add links or allow_unmapped_rest_operation_ids): "
            + ", ".join(unmapped_rest)
        )
    if unmapped_grpc:
        errors.append(
            "missing gRPC mappings (add links or allow_unmapped_grpc_methods): "
            + ", ".join(unmapped_grpc)
        )

    if errors:
        for message in errors:
            print(f"error: {message}", file=sys.stderr)
        return 1

    bundle = {
        "version": 1,
        "sources": {
            "rest_openapi": to_repo_relative(rest_path, repo_root),
            "grpc_openapi_bridge": to_repo_relative(grpc_path, repo_root),
            "links_toml": to_repo_relative(links_path, repo_root),
        },
        "rest": {
            "operation_count": len(rest_operations),
            "operations": rest_operations,
        },
        "grpc": {
            "method_count": len(grpc_methods),
            "methods": grpc_methods,
        },
        "links": normalized_links,
        "coverage": {
            "allow_unmapped_rest_operation_ids": allow_unmapped_rest,
            "allow_unmapped_grpc_methods": allow_unmapped_grpc,
            "unmapped_rest_operation_ids": unmapped_rest,
            "unmapped_grpc_methods": unmapped_grpc,
        },
    }

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as handle:
        json.dump(bundle, handle, indent=2, sort_keys=True)
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
