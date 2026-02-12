use std::{
    env, fs,
    path::{Component, Path, PathBuf},
};

fn print_usage(binary_name: &str) {
    eprintln!(
        "Usage: {binary_name} [--out <path>]\n\
         Note: --out must be a repo-relative path under docs/generated/"
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should be resolvable")
}

fn docs_generated_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("docs/generated")
}

fn resolve_output_path(raw: &str, workspace_root: &Path) -> Result<PathBuf, String> {
    let candidate = Path::new(raw);
    if candidate.as_os_str().is_empty() {
        return Err("--out path cannot be empty".to_string());
    }
    if candidate.is_absolute() {
        return Err("--out must be a repo-relative path".to_string());
    }
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("--out cannot contain parent-directory traversal ('..')".to_string());
    }

    let resolved = workspace_root.join(candidate);
    let allowed_root = docs_generated_dir(workspace_root);
    if !resolved.starts_with(&allowed_root) {
        return Err("--out must stay under docs/generated/".to_string());
    }

    Ok(resolved)
}

fn parse_out_path(workspace_root: &Path) -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut out = resolve_output_path("docs/generated/rest-openapi.json", workspace_root)?;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--out requires a path value".to_string())?;
                out = resolve_output_path(&value, workspace_root)?;
            }
            "--help" | "-h" => {
                let binary = env::args()
                    .next()
                    .unwrap_or_else(|| "openapi_docgen".to_string());
                print_usage(&binary);
                std::process::exit(0);
            }
            unknown => {
                return Err(format!("unknown argument: {unknown}"));
            }
        }
    }

    Ok(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root();
    let out_path = match parse_out_path(&workspace_root) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("error: {message}");
            let binary = env::args()
                .next()
                .unwrap_or_else(|| "openapi_docgen".to_string());
            print_usage(&binary);
            std::process::exit(2);
        }
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let document = meld_server::rest_openapi_json_pretty();
    fs::write(&out_path, document)?;
    let display_path = out_path
        .strip_prefix(&workspace_root)
        .unwrap_or(out_path.as_path());
    println!("{}", display_path.display());
    Ok(())
}
