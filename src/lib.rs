// Licensed under the Apache License, Version 2.0

use anyhow::{anyhow, Context, Result};
use cargo_manifest::Product;

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

impl Args {
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
                .context("Package manifest does not exist.")?
        };

        let res = Args {
            install_base,
            build_base,
            forwarded_args,
            profile,
            manifest_path,
        };

        Ok(res)
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
        .context("Failed to spawn 'cargo build' subprocess.")?;
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
                "Failed to create package marker directory '{}'.",
                path.display()
            )
        })?;
    path.push(package_name);
    File::create(&path)
        .with_context(|| format!("Failed to create package marker '{}'.", path.display()))?;
    Ok(())
}

/// Copy the source code of the package to the install space
pub fn install_package(
    install_base: impl AsRef<Path>,
    package_path: impl AsRef<Path>,
    package_name: &str,
) -> Result<()> {
    let mut dest = install_base.as_ref().to_owned();
    dest.push("share");
    dest.push(package_name);
    dest.push("rust");
    fs_extra::dir::remove(&dest)?;
    DirBuilder::new().recursive(true).create(&dest)?;
    let mut opt = fs_extra::dir::CopyOptions::new();
    opt.overwrite = true;
    for dir_entry in std::fs::read_dir(package_path)? {
        let dir_entry = dir_entry?;
        let src = dir_entry.path();
        let filename = dir_entry.file_name();
        // There might be a target directory after a manual build with cargo
        if filename == "target" {
            continue;
        }
        if src.is_dir() {
            fs_extra::dir::copy(&src, &dest, &opt).context("Failed to install package.")?;
        } else {
            let dest_file = dest.join(filename);
            std::fs::copy(&src, &dest_file).context("Failed to install package.")?;
        }
    }
    Ok(())
}

/// Copy the binaries to a location where they will be found by ROS 2 tools (the lib dir)
pub fn install_binaries(
    install_base: impl AsRef<Path>,
    build_base: impl AsRef<Path>,
    package_name: &str,
    profile: &str,
    binaries: &[Product],
) -> Result<()> {
    let src_dir = build_base.as_ref().join(profile);
    let dest_dir = install_base.as_ref().join("lib").join(package_name);
    // Copy binaries
    for binary in binaries {
        let name = binary
            .name
            .as_ref()
            .ok_or(anyhow!("Binary without name found."))?;
        let src = src_dir.join(name);
        let dest = dest_dir.join(name);
        // Create destination directory
        DirBuilder::new().recursive(true).create(&dest_dir)?;
        std::fs::copy(&src, &dest)
            .context(format!("Failed to copy binary from '{}'.", src.display()))?;
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
                .context(format!("Failed to copy library from '{}'.", src.display()))?;
        }
    }
    Ok(())
}
