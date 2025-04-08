# nix-upgrade

An elegant solution for updating NixOS systems during shutdown.

## Description

`nix-upgrade` is a tool that automatically updates your NixOS system during shutdown or reboot. Unlike the standard `system.autoUpgrade` module which performs periodic updates via a systemd timer, `nix-upgrade` works when the system shuts down, ensuring your server will boot with the latest version on its next start.

## Features

- 🔄 Automatic updates during system shutdown or reboot
- 🔒 Network availability check before updating
- 🌟 Support for both Nix flakes and traditional channels
- ⚙️ Flexible configuration via JSON
- 📝 Detailed logging of update operations
- 🔧 Compatible with NixOS via a dedicated module

## Installation

TODO

## Usage

Once configured in your NixOS system, `nix-upgrade` works automatically during system shutdown or reboot. No manual intervention is needed.

To temporarily disable shutdown updates:

```nix
system.shutdownUpgrade.enable = false;
```

## Configuration

The NixOS module offers the following configuration options:

| Option | Type | Default | Description |
|--------|------|--------|-------------|
| `enable` | boolean | `false` | Enables shutdown upgrade |
| `operation` | enum (`"switch"` or `"boot"`) | `"boot"` | Type of operation for nixos-rebuild |
| `flake` | string or null | `null` | Nix flake URI to use for the upgrade |
| `channel` | string or null | `null` | NixOS channel to use (if not using flake) |
| `flags` | [string] | `["--no-build-output"]` | Additional options for nixos-rebuild |

### Configuration Example

```nix
system.shutdownUpgrade = {
  enable = true;
  operation = "boot";
  flags = [
    "--update-input" "nixpkgs"
    "--commit-lock-file"
  ];
};
```

## How It Works

The tool operates through a systemd service configured to run before shutdown targets (`shutdown.target`, `reboot.target`, `halt.target`). During shutdown, the Rust application:

1. Checks network availability
2. Loads configuration
3. Executes `nixos-rebuild` with appropriate options
4. Logs results to the system journal

## Development

### Prerequisites

- Rust 1.70.0 or higher
- Cargo
- Nix 2.4 or higher

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```
