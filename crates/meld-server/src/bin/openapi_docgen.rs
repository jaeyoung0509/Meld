use std::{env, fs, path::PathBuf};

fn print_usage(binary_name: &str) {
    eprintln!("Usage: {binary_name} [--out <path>]");
}

fn parse_out_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut out = PathBuf::from("docs/generated/rest-openapi.json");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--out requires a path value".to_string())?;
                out = PathBuf::from(value);
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
    let out_path = match parse_out_path() {
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
    println!("{}", out_path.display());
    Ok(())
}
