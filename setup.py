import os
import shlex

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib
from setuptools import setup

from setuptools_rust import RustBin

# Force the wheel to be platform specific
# https://stackoverflow.com/a/45150383/3549270
# There's also the much more concise solution in
# https://stackoverflow.com/a/53463910/3549270,
# but that would require python-dev
try:
    # noinspection PyPackageRequirements,PyUnresolvedReferences
    from wheel.bdist_wheel import bdist_wheel as _bdist_wheel

    # noinspection PyPep8Naming,PyAttributeOutsideInit
    class bdist_wheel(_bdist_wheel):
        def finalize_options(self):
            _bdist_wheel.finalize_options(self)
            self.root_is_pure = False

except ImportError:
    bdist_wheel = None

with open("Cargo.toml", "rb") as fp:
    cargo_data = tomllib.load(fp)
    version = cargo_data["package"]["version"]
    description = cargo_data["package"]["description"]

# Use `--no-default-features` by default for a minimal build to support PEP 517.
# `MATURIN_SETUP_ARGS` env var can be used to pass customized arguments to cargo.
cargo_args = ["--no-default-features"]
long_description = description

setup(
    version=version,
    cmdclass={"bdist_wheel": bdist_wheel},
    rust_extensions=[RustBin("cargo-ament-build", args=cargo_args, cargo_manifest_args=["--locked"])],
    zip_safe=False,
    description=description,
    long_description=long_description,
)
