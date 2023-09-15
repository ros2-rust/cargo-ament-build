// Licensed under the Apache License, Version 2.0

use anyhow::{anyhow, bail, Context, Result};
use cargo_manifest::{Manifest, Product, Value, StringOrBool};
use safe_drive_msg::SafeDrive;

use std::ffi::OsString;
use std::fs::{DirBuilder, File};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Arguments for both the wrapper and for `cargo build`.
pub struct Args {
    /// The install base for this package (i.e. directory containing `lib`, `share` etc.)
    pub install_base: PathBuf,
    /// The build base for this package, corresponding to the --target-dir option
    pub build_base: PathBuf,
    /// Arguments to be forwarded to `cargo build`.
    pub forwarded_args: Vec<OsString>,
    /// "debug", "release" etc.
    pub profile: String,
    /// The absolute path to the Cargo.toml file. Currently the --manifest-path option is not implemented.
    pub manifest_path: PathBuf,
}

/// Wrapper around [`Args`] that can also indicate the --help flag.
pub enum ArgsOrHelp {
    Args(Args),
    Help,
}

impl ArgsOrHelp {
    /// This binary not only reads arguments before the --, but also selected arguments after
    /// the --, so that it knows where the resulting binaries will be located.
    pub fn parse() -> Result<Self> {
        let mut args: Vec<_> = std::env::args_os().collect();
        args.remove(0); // Remove the executable path.

        // Find and process `--`.
        let forwarded_args = if let Some(dash_dash) = args.iter().position(|arg| arg == "--") {
            // Store all arguments following ...
            let later_args: Vec<_> = args[dash_dash + 1..].to_vec();
            // .. then remove the `--`
            args.remove(dash_dash);
            later_args
        } else {
            Vec::new()
        };

        // Now pass all the arguments (without `--`) through to `pico_args`.
        let mut args = pico_args::Arguments::from_vec(args);
        if args.contains("--help") {
            return Ok(ArgsOrHelp::Help);
        }
        let profile = if args.contains("--release") {
            String::from("release")
        } else if let Ok(p) = args.value_from_str("--profile") {
            p
        } else {
            String::from("debug")
        };

        let build_base = args
            .opt_value_from_str("--target-dir")?
            .unwrap_or_else(|| "target".into());
        let install_base = args.value_from_str("--install-base")?;

        let manifest_path = if let Ok(p) = args.value_from_str("--manifest-path") {
            p
        } else {
            PathBuf::from("Cargo.toml")
                .canonicalize()
                .context("Package manifest does not exist")?
        };

        let res = Args {
            install_base,
            build_base,
            forwarded_args,
            profile,
            manifest_path,
        };

        Ok(ArgsOrHelp::Args(res))
    }

    pub fn print_help() {
        println!("cargo-ament-build");
        println!("Wrapper around cargo-build that installs compilation results and extra files to an ament/ROS 2 install space.\n");
        println!("USAGE:");
        println!("    cargo ament-build --install-base <INSTALL_DIR> -- <CARGO-BUILD-OPTIONS>");
    }
}

/// Run a certain cargo verb
pub fn cargo(args: &[OsString], verb: &str) -> Result<Option<i32>> {
    let mut cmd = Command::new("cargo");
    // "check" and "build" have compatible arguments
    cmd.arg(verb);
    for arg in args {
        cmd.arg(arg);
    }
    let exit_status = cmd
        .status()
        .context("Failed to spawn 'cargo build' subprocess")?;
    Ok(exit_status.code())
}

/// This is comparable to ament_index_register_resource() in CMake
pub fn create_package_marker(
    install_base: impl AsRef<Path>,
    marker_dir: &str,
    package_name: &str,
) -> Result<()> {
    let mut path = install_base
        .as_ref()
        .join("share/ament_index/resource_index");
    path.push(marker_dir);
    DirBuilder::new()
        .recursive(true)
        .create(&path)
        .with_context(|| {
            format!(
                "Failed to create package marker directory '{}'",
                path.display()
            )
        })?;
    path.push(package_name);
    File::create(&path)
        .with_context(|| format!("Failed to create package marker '{}'", path.display()))?;
    Ok(())
}

pub fn generate_msg(metadata: Option<&Value>) -> Result<()> {
    let (libs, msg_dir, safe_drive) = if let Some(metadata_table) = metadata {
        if let Some(ros_table) = metadata_table.get("ros") {
            let libs = match ros_table.get("msg") {
                Some(Value::Array(arr)) => arr,
                Some(_) => bail!("The [package.metadata.ros.msg] entry is not an array"),
                _ => return Ok(()),
            };

            let msg_dir = match ros_table.get("msg_dir") {
                Some(Value::String(dir)) => Path::new(dir),
                Some(_) => bail!("The [package.metadata.ros.msg_dir] entry is not an astring"),
                _ => bail!("[package.metadata.ros.msg_dir] is required"),
            };

            let safe_drive = match ros_table.get("safe_drive_path") {
                Some(Value::String(safe_drive_path)) => SafeDrive::Path(safe_drive_path),
                Some(_) => {
                    bail!("The [package.metadata.ros.safe_drive_path] entry is not an string")
                }
                _ => match ros_table.get("safe_drive_version") {
                    Some(Value::String(safe_drive_path)) => SafeDrive::Version(safe_drive_path),
                    Some(_) => bail!(
                        "The [package.metadata.ros.safe_drive_version] entry is not an string"
                    ),
                    _ => bail!("[package.metadata.ros.safe_drive_path] or [package.metadata.ros.safe_drive_version] is required"),
                },
            };

            (libs, msg_dir, safe_drive)
        } else {
            return Ok(());
        }
    } else {
        return Ok(());
    };

    let libs: Vec<_> = libs.iter().filter_map(|v| v.as_str()).collect();

    if let Err(e) = safe_drive_msg::depends(msg_dir, &libs, safe_drive) {
        Err(anyhow!("failed to generate Rust's projects: {e}"))
    } else {
        Ok(())
    }
}

/// Copies files or directories.
fn copy(src: impl AsRef<Path>, dest_dir: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dest = dest_dir.as_ref().join(src.file_name().unwrap());
    if src.is_dir() {
        std::fs::create_dir_all(&dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                copy(entry.path(), &dest)?;
            } else {
                std::fs::copy(entry.path(), dest.join(entry.file_name()))?;
            }
        }
    } else if src.is_file() {
        std::fs::copy(&src, &dest).with_context(|| {
            format!(
                "Failed to copy '{}' to '{}'.",
                src.display(),
                dest.display()
            )
        })?;
    } else {
        bail!("File or dir '{}' does not exist", src.display())
    }
    Ok(())
}

/// Copy the source code of the package to the install space
///
/// Specifically, `${install_base}/share/${package}/rust`.
pub fn install_package(
    install_base: impl AsRef<Path>,
    package_path: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    package_name: &str,
    manifest: &Manifest,
) -> Result<()> {
    // Install source code
    // This is special-cased (and not simply added to the list of things to install below)
    let mut dest_dir = install_base.as_ref().to_owned();
    dest_dir.push("share");
    dest_dir.push(package_name);
    dest_dir.push("rust");
    if dest_dir.is_dir() {
        std::fs::remove_dir_all(&dest_dir)?;
    }
    DirBuilder::new().recursive(true).create(&dest_dir)?;
    // unwrap is ok since it has been validated in main
    let package = manifest.package.as_ref().unwrap();
    // The entry for the build script can be empty (in which case build.rs is implicitly used if it
    // exists), or a path, or false (in which case build.rs is not implicitly used).
    let build = match &package.build {
        Some(StringOrBool::Bool(false)) => None,
        Some(StringOrBool::String(path)) => Some(path.as_str()),
        Some(_) => bail!("Value of 'build' is not a string or boolean"),
        None => None,
    };
    if let Some(filename) = build {
        let src = package_path.as_ref().join(filename);
        copy(src, &dest_dir)?;
    }

    copy(package_path.as_ref().join("src"), &dest_dir)?;
    copy(manifest_path.as_ref(), &dest_dir)?;

    // Copy the workspace's lock file
    let mut is_workspace = false;
    if let Some(parent) = manifest_path.as_ref().parent().map_or(None, |p| p.parent()) {
        if copy(parent.join("Cargo.lock"), &dest_dir).is_ok() {
            is_workspace = true;
        }
    }

    if !is_workspace {
        copy(manifest_path.as_ref().with_extension("lock"), &dest_dir)?;
    }

    // unwrap is ok since we pushed to the path before
    copy(
        package_path.as_ref().join("package.xml"),
        dest_dir.parent().unwrap(),
    )?;
    Ok(())
}

/// Copy the binaries to a location where they will be found by ROS 2 tools (the lib dir)
pub fn install_binaries(
    install_base: impl AsRef<Path>,
    build_base: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    package_name: &str,
    profile: &str,
    binaries: &[Product],
) -> Result<()> {
    let src_dir = build_base.as_ref().join(profile);
    let dest_dir = install_base.as_ref().join("lib").join(package_name);
    if dest_dir.is_dir() {
        std::fs::remove_dir_all(&dest_dir)?;
    }
    // Copy binaries
    for binary in binaries {
        let mut name = binary
            .name
            .clone()
            .ok_or(anyhow!("Binary without name found"))?;
        if cfg!(target_os = "windows") {
            name.push_str(".exe");
        }
        let src = src_dir.join(&name);
        let dest = dest_dir.join(&name);
        // Create destination directory
        DirBuilder::new().recursive(true).create(&dest_dir)?;

        let mut is_workspace = false;
        if let Some(parent) = manifest_path.as_ref().parent().map_or(None, |p| p.parent()) {
            if copy(parent.join("target").join(profile).join(&name), &dest_dir).is_ok() {
                is_workspace = true;
            }
        }

        if !is_workspace {
            std::fs::copy(&src, &dest)
                .context(format!("Failed to copy binary from '{}'", src.display()))?;
        }
    }
    // If there is a shared or static library, copy it too
    // See https://doc.rust-lang.org/reference/linkage.html for an explanation of suffixes
    let prefix_suffix_combinations = [
        ("lib", "so"),
        ("lib", "dylib"),
        ("lib", "a"),
        ("", "dll"),
        ("", "lib"),
    ];
    for (prefix, suffix) in prefix_suffix_combinations {
        let filename = String::from(prefix) + package_name + "." + suffix;
        let src = src_dir.join(&filename);
        let dest = dest_dir.join(filename);
        if src.is_file() {
            // Create destination directory
            DirBuilder::new().recursive(true).create(&dest_dir)?;
            std::fs::copy(&src, &dest)
                .context(format!("Failed to copy library from '{}'", src.display()))?;
        }
    }
    Ok(())
}

/// Copy selected files/directories to the share dir.
pub fn install_files_from_metadata(
    install_base: impl AsRef<Path>,
    package_path: impl AsRef<Path>,
    package_name: &str,
    metadata: Option<&Value>,
) -> Result<()> {
    // Unpack the metadata entry
    let metadata_table = match metadata {
        Some(Value::Table(tab)) => tab,
        _ => return Ok(()),
    };
    let metadata_ros_table = match metadata_table.get("ros") {
        Some(Value::Table(tab)) => tab,
        _ => return Ok(()),
    };
    for subdir in ["share", "include", "lib"] {
        let dest = install_base.as_ref().join(subdir).join(package_name);
        DirBuilder::new().recursive(true).create(&dest)?;
        let key = format!("install_to_{subdir}");
        let install_array = match metadata_ros_table.get(&key) {
            Some(Value::Array(arr)) => arr,
            Some(_) => bail!("The [package.metadata.ros.{key}] entry is not an array"),
            _ => return Ok(()),
        };
        let install_entries = install_array
            .iter()
            .map(|entry| match entry {
                Value::String(dir) => Ok(dir.clone()),
                _ => {
                    bail!("The elements of the [package.metadata.ros.{key}] array must be strings")
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        for rel_path in install_entries {
            let src = package_path.as_ref().join(&rel_path);
            copy(&src, &dest).with_context(|| {
                format!("Could not process [package.metadata.ros.{key}] entry '{rel_path}'",)
            })?;
        }
    }
    Ok(())
}
