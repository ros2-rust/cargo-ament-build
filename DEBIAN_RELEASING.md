# Releasing cargo-ament-build debians

This document describes how to create new Debian releases of cargo-ament-build.

## Debian Package Generation

Debian packages are built automatically via GitHub Actions when a release tag is pushed. The workflow uses the same semver-style tag pattern as the existing release workflow, so packages are built for published releases instead of every push to `main`. No special tools are needed locally for most releases.

The Debian changelog is auto-generated from metadata in `Cargo.toml`, so you only need to update the version there.

## Debian Metadata Configuration

The Debian-specific metadata is stored in `Cargo.toml` under `[package.metadata.deb]`:

```toml
[package.metadata.deb]
maintainer = "Esteve Fernandez <esteve@apache.org>"
distributions = "jammy noble"
```

| Field | Description |
|-------|-------------|
| `maintainer` | Package maintainer name and email |
| `distributions` | Space-separated list of target distros |

The changelog is generated with:
- Version from `[package].version` in Cargo.toml
- Maintainer and distributions from `[package.metadata.deb]`
- A generated release message for the tagged version
- Current timestamp in RFC 2822 format
