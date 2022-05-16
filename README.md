# cargo-ament-build

This cargo plugin is a wrapper around `cargo build` which installs build artifacts in a layout expected by ament and ROS 2 tools.

It can be used standalone or through `colcon-ros-cargo`. Its command line interface is `cargo ament-build --install-base <install base> -- <cargo build args>`.

What does this plugin do?
- It builds or checks the package, depending on whether it contains any binaries
- It copies the source code and binaries to appropriate locations in the install base
- It places marker files in the ament index

It is possible to specify additional files or directories to be installed in the `metadata` section of `Cargo.toml` like this:
```
[package.metadata.ros]
install_to_share = ["launch", "config"]
```
These paths are relative to the directory containing the `Cargo.toml` file and will be copied to the appropriate location in `share`.

The same mechanism applies with `install_to_include` and `install_to_lib`.

Target types other than libraries and binaries (i.e. benches, tests) are not yet installed.