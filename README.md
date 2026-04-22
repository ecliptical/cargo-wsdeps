# cargo wsdeps

Manage dependencies that are shared among crates in a workspace.

## Installation

Using homebrew:

```shell
brew install ecliptical/cargo-wsdeps/cargo-wsdeps
```

Using `cargo binstall`:

```shell
cargo binstall --locked cargo-wsdeps
```

Using `cargo install`:

```shell
cargo install --locked cargo-wsdeps
```

## Usage

`cargo wsdeps` suggests three kinds of changes to a workspace:

1. consolidate dependencies declared inline by 2+ member crates into shared
   `[workspace.dependencies]` entries (and rewrite the members to use
   `workspace = true`),
2. remove `[workspace.dependencies]` entries that no member references, and
3. (with `--aggressive`) move a workspace dependency back into the sole member
   that uses it, when only one member inherits it via `workspace = true` and
   no other member references it inline.

To print a summary of suggested changes:

```shell
cargo wsdeps show [--aggressive]
```

To generate a unified diff with the suggested changes:

```shell
cargo wsdeps diff [--dotted] [--aggressive]
```

Copyright (c) 2025 - 2026 Ecliptical Software Inc. All rights reserved.
