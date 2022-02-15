# cargo-ament-build

This cargo plugin is a wrapper around `cargo build` which installs build artifacts in a layout expected by ament and ROS 2 tools.

It can be used standalone or through `colcon-ros-cargo`. Its command line interface is `cargo ament-build --install-base <install base> -- <cargo build args>`.

What does this plugin do?
- It builds or checks the package, depending on whether it contains any binaries
- It copies the source code and binaries to appropriate locations in the install base
- It places marker files in the ament index