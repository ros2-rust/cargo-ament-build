// Licensed under the Apache License, Version 2.0

use anyhow::{anyhow, Context, Result};

use cargo_ament_build::*;
use cargo_manifest::Manifest;

fn main() {
    let exitcode = match fallible_main().context("Error in cargo-ament-build") {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => {
            eprintln!("{:?}", e);
            1
        }
    };
    // No destructors left to run, so it's fine to exit.
    std::process::exit(exitcode);
}

/// Returns Ok when there was no error in this plugin itself (even though cargo
/// build/check may have failed), and a boolean indicating the cargo build/check
/// status.
fn fallible_main() -> Result<bool> {
    let args = match ArgsOrHelp::parse()? {
        ArgsOrHelp::Args(args) => args,
        ArgsOrHelp::Help => {
            ArgsOrHelp::print_help();
            return Ok(true);
        }
    };
    let mut manifest = Manifest::from_path(&args.manifest_path)?;
    manifest.complete_from_path(&args.manifest_path)?;

    // Unwrap is safe since complete_from_path() has been called
    let is_pure_library = {
        let no_binaries = manifest.bin.as_ref().unwrap().is_empty();
        let no_exported_libraries = if let Some(crate_types) = manifest
            .lib
            .as_ref()
            .and_then(|lib| lib.crate_type.as_ref())
        {
            crate_types.as_slice() == [String::from("rlib")]
        } else {
            true
        };
        no_binaries && no_exported_libraries
    };
    let verb = if is_pure_library { "check" } else { "build" };
    let exitcode = cargo(&args.forwarded_args, verb)?
        .ok_or_else(|| anyhow!("'cargo {}' was terminated by signal.", verb))?;
    if exitcode != 0 {
        return Ok(false);
    }
    let package = manifest
        .package
        .as_ref()
        .ok_or(anyhow!("Cargo manifest has no package section."))?;
    let package_name = &package.name;
    let package_path = args
        .manifest_path
        .parent()
        .ok_or(anyhow!("Manifest path must have a parent."))?;
    // Putting marker file creation after the actual build command means that
    // we create less garbage if the build command failed.
    create_package_marker(&args.install_base, "packages", package_name)?;
    // This marker is used by colcon-ros-cargo when looking for dependencies
    create_package_marker(&args.install_base, "rust_packages", package_name)?;
    install_package(
        &args.install_base,
        package_path,
        &args.manifest_path,
        package_name,
        &manifest,
    )?;
    install_binaries(
        &args.install_base,
        &args.build_base,
        package_name,
        &args.profile,
        // Unwrap is safe since complete_from_path() has been called
        &manifest.bin.unwrap(),
    )?;
    install_to_share(
        &args.install_base,
        package_path,
        package_name,
        package.metadata.as_ref(),
    )?;
    Ok(true)
}
