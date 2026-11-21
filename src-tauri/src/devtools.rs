use crate::customization::{plugin, Error};
use serde::Serialize;
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Serialize)]
struct Failure {
    ok: bool,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize)]
struct Validation {
    ok: bool,
    package: plugin::PackageInspection,
}

#[derive(Serialize)]
struct Packed {
    ok: bool,
    output: String,
    package: plugin::PackageInspection,
}

#[derive(Serialize)]
struct Version {
    ok: bool,
    protocol: u32,
}

fn diagnostic(error: Error) -> Diagnostic {
    Diagnostic {
        code: error.code,
        message: error.message,
        path: error.path,
    }
}

fn fail(code: &'static str, message: impl ToString) -> i32 {
    print_failure(vec![Diagnostic {
        code,
        message: message.to_string(),
        path: None,
    }]);
    1
}

fn print_failure(diagnostics: Vec<Diagnostic>) -> i32 {
    let payload = Failure {
        ok: false,
        diagnostics,
    };
    println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    1
}

fn validate(path: &Path) -> i32 {
    match plugin::inspect_path(path) {
        Ok(package) => {
            println!(
                "{}",
                serde_json::to_string(&Validation { ok: true, package }).unwrap_or_default()
            );
            0
        }
        Err(error) => print_failure(vec![diagnostic(error)]),
    }
}

fn pack(path: &Path, output: Option<&Path>) -> i32 {
    let inspection = match plugin::inspect_path(path) {
        Ok(inspection) => inspection,
        Err(error) => return print_failure(vec![diagnostic(error)]),
    };
    let target = match output {
        Some(output)
            if output
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip")) =>
        {
            output.to_path_buf()
        }
        Some(output) => output.join(format!(
            "{}-{}.luma-extension.zip",
            inspection.id, inspection.version
        )),
        None => PathBuf::from(format!(
            "{}-{}.luma-extension.zip",
            inspection.id, inspection.version
        )),
    };
    match plugin::pack_path(path, &target) {
        Ok(package) => {
            println!(
                "{}",
                serde_json::to_string(&Packed {
                    ok: true,
                    output: target.display().to_string(),
                    package
                })
                .unwrap_or_default()
            );
            0
        }
        Err(error) => print_failure(vec![diagnostic(error)]),
    }
}

fn usage() -> i32 {
    eprintln!("Usage:");
    eprintln!("  luma-plugin-host validate <package-directory-or-zip>");
    eprintln!("  luma-plugin-host pack <package-directory> [--out <directory-or-zip>]");
    eprintln!("  luma-plugin-host version");
    2
}

pub fn run_cli(args: impl IntoIterator<Item = OsString>) -> i32 {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return usage();
    };
    match command.to_string_lossy().as_ref() {
        "validate" => {
            let Some(path) = args.next() else {
                return fail(
                    "usage",
                    "validate requires a package directory or ZIP path.",
                );
            };
            if args.next().is_some() {
                return fail("usage", "validate accepts one package path.");
            }
            validate(Path::new(&path))
        }
        "pack" => {
            let Some(path) = args.next() else {
                return fail("usage", "pack requires a package directory.");
            };
            let mut output = None;
            while let Some(argument) = args.next() {
                if argument == "--out" {
                    let Some(value) = args.next() else {
                        return fail("usage", "pack --out requires a path.");
                    };
                    output = Some(PathBuf::from(value));
                } else {
                    return fail(
                        "usage",
                        format!("Unknown pack argument: {}", argument.to_string_lossy()),
                    );
                }
            }
            pack(Path::new(&path), output.as_deref())
        }
        "version" => {
            println!(
                "{}",
                serde_json::to_string(&Version {
                    ok: true,
                    protocol: 1
                })
                .unwrap_or_default()
            );
            0
        }
        _ => usage(),
    }
}

pub fn main() -> i32 {
    run_cli(env::args_os().skip(1))
}
