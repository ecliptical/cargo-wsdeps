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

To generate a diff with suggested changes for a workspace in the current directory:

```shell
cargo wsdeps --diff
```

Copyright (c) 2025 Ecliptical Software Inc. All rights reserved.
