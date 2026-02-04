# Releasing cargo-ament-build

This document describes how to create new Debian releases of cargo-ament-build.

## Overview

Debian packages are built automatically via GitHub Actions. When changes are pushed to the `main` branch, the workflow builds packages for multiple distributions and architectures. No special tools are needed locally for most releases.

The Debian changelog is auto-generated from metadata in `Cargo.toml`, so you only need to update the version there.

## Creating a New Release

### Step 1: Update version in Cargo.toml

Edit `Cargo.toml` and update the version number:

```toml
[package]
name = "cargo-ament-build"
version = "X.Y.Z"
```

### Step 2: Commit and push to main

Commit your changes and push to the `main` branch. The GitHub Actions workflow will:

1. Extract version and metadata from `Cargo.toml`
2. Auto-generate the Debian changelog with the git commit message
3. Build packages for all configured distributions and architectures

### Step 3: Download the packages

Once the workflow completes:

1. Go to the repository's Actions tab
2. Find the "Build Debian Package" workflow run
3. Download the packages from the Artifacts section

## Debian Metadata Configuration

The Debian-specific metadata is stored in `Cargo.toml` under `[package.metadata.deb]`:

```toml
[package.metadata.deb]
maintainer = "Esteve Fernandez <esteve@apache.org>"
distributions = "focal jammy noble bookworm trixie"
```

| Field | Description |
|-------|-------------|
| `maintainer` | Package maintainer name and email |
| `distributions` | Space-separated list of target distros |

The changelog is generated with:
- Version from `[package].version` in Cargo.toml
- Maintainer and distributions from `[package.metadata.deb]`
- Commit message from the latest git commit
- Current timestamp in RFC 2822 format
