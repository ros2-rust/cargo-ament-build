# Releasing cargo-ament-build

This document describes how to create new Debian releases of cargo-ament-build.

## Overview

Debian packages are built automatically via GitHub Actions. When changes are pushed to the `main` branch, the workflow builds packages for multiple distributions and architectures. No special tools are needed locally for most releases.

## Creating a New Release

### Step 1: Update version in Cargo.toml

Edit `Cargo.toml` and update the version number on line 3:

```toml
[package]
name = "cargo-ament-build"
version = "X.Y.Z"
```

### Step 2: Update contrib/debian/changelog

Add a new entry at the **top** of `contrib/debian/changelog`. The newest entry must always be first.

Use this template:

```
cargo-ament-build (X.Y.Z-1) focal jammy noble bookworm trixie; urgency=low

  * Description of changes

 -- Your Name <your.email@example.com>  Day, DD Mon YYYY HH:MM:SS +ZZZZ

```

**Important notes:**
- The version format is `X.Y.Z-N` where:
  - `X.Y.Z` is the upstream version (must match Cargo.toml)
  - `N` is the Debian revision (start with `1` for new upstream versions)
- Target distributions: `focal jammy noble bookworm trixie`
- There must be exactly **two spaces** before the maintainer line
- There must be exactly **two spaces** between the email and the date

### Step 4: GitHub Actions builds the packages

The workflow runs automatically on push to `main`. Once complete:
1. Go to the repository's Actions tab
2. Find the "Build Debian Package" workflow run
3. Download the packages from the Artifacts section

## Debian Changelog Format Reference

Each changelog entry has this structure:

```
package-name (version) distributions; urgency=level

  * Change description
  * Another change

 -- Maintainer Name <email@example.com>  Day, DD Mon YYYY HH:MM:SS +ZZZZ

```

| Field | Description |
|-------|-------------|
| `package-name` | Always `cargo-ament-build` |
| `version` | Format: `X.Y.Z-N` (upstream version - debian revision) |
| `distributions` | Space-separated list of target distros |
| `urgency` | Usually `low` for regular releases |
| `Day` | Three-letter day abbreviation (Mon, Tue, etc.) |
| `+ZZZZ` | Timezone offset (e.g., `+0100` for CET) |

### Generating the date

Use the `date -R` command to generate a properly formatted date:

```bash
$ date -R
Wed, 28 Jan 2026 10:30:00 +0100
```

