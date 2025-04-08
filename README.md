# nix-upgrade

A reliable tool for NixOS system upgrades during shutdown or reboot.

## Description

`nix-upgrade` enhances NixOS's auto-upgrade functionality by adding the ability to perform upgrades during system shutdown or reboot. While the standard module performs upgrades on a timer, this enhancement ensures your system will boot with the latest version on its next start, which is particularly valuable for servers and critical systems.

## Features

- 🔄 Upgrade NixOS during shutdown or reboot
- 🔒 Network availability check before upgrading
- 🌟 Support for both Nix flakes and traditional channels
- ⚙️ Seamless integration with the standard auto-upgrade module
- 📝 Detailed logging of upgrade operations
- 🛡️ Support for controlled reboots with reboot windows

## How It Works

The tool consists of two parts:
1. A Rust application that performs the actual upgrade, handling network checks and config parsing
2. An extension to the standard `auto-upgrade.nix` module that adds the new `onShutdown` option

## Installation

### Step 1: Install the nix-upgrade package

Add the package to your configuration:

```nix
nixpkgs.overlays = [
  (self: super: {
    nix-upgrade = self.callPackage /path/to/nix-upgrade-package.nix {};
  })
];
```

### Step 2: Replace or extend the auto-upgrade module

Either replace the standard module:

```nix
disabledModules = [ "tasks/auto-upgrade.nix" ];
imports = [ ./path/to/modified-auto-upgrade.nix ];
```

Or use it as an extension if you're developing outside the main NixOS repository.

## Configuration

The module uses the standard `system.autoUpgrade` namespace with an additional `onShutdown` option:

```nix
system.autoUpgrade = {
  # Standard options
  enable = true;                 # Enable periodic upgrades (standard functionality)
  dates = "04:40";               # When to run periodic upgrades

  # New option
  onShutdown = true;             # Enable upgrades during shutdown or reboot

  # Shared options (used by both types of upgrades)
  operation = "boot";            # Use "boot" for shutdown upgrades (recommended)
  flake = "github:user/config";  # Flake URI (if using flakes)
  # channel = "...";             # Or channel (if not using flakes)
  allowReboot = true;            # Whether to reboot if needed

  # Additional options
  flags = [                      # Additional flags for nixos-rebuild
    "--update-input" "nixpkgs"
    "--commit-lock-file"
  ];
  rebootWindow = {               # Time window when reboots are allowed
    lower = "01:00";
    upper = "05:00";
  };
};
```

### Upgrade Strategies

You can choose one of the following strategies:

1. **Periodic upgrades only**: Set `enable = true` and `onShutdown = false`
2. **Shutdown upgrades only**: Set `enable = false` and `onShutdown = true`
3. **Both types of upgrades**: Set both `enable = true` and `onShutdown = true`

## Operation Modes

For `system.autoUpgrade.operation`, there are two options:

- `switch`: Changes take effect immediately (better for periodic upgrades)
- `boot`: Changes take effect on next boot (recommended for shutdown upgrades)

## Advanced Configuration

### Reboot Control

If `allowReboot` is set to `true`, the system will reboot automatically if a kernel, initrd, or module change is detected during the upgrade. You can restrict when these reboots occur using the `rebootWindow` option.

```nix
rebootWindow = {
  lower = "01:00";  # 1:00 AM
  upper = "05:00";  # 5:00 AM
};
```

### Network Requirements

The shutdown upgrade service checks for network connectivity before proceeding. If the network is not available, the upgrade will be skipped.

## Development

### Prerequisites

- Rust 1.70.0 or higher
- Cargo
- Nix 2.4 or higher

### Building the Rust application

```bash
cargo build --release
```

### Testing

For testing the shutdown upgrade without actually shutting down, you can manually run:

```bash
sudo nix-upgrade --config /etc/nix-upgrade.json
```
