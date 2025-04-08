use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
enum NixosUpgradeError {
    #[error("Failed to check network connectivity: {0}")]
    NetworkCheck(#[source] std::io::Error),

    #[error("Network is not available")]
    NetworkUnavailable,

    #[error("Failed to execute nixos-rebuild: {0}")]
    NixosRebuild(#[source] std::io::Error),

    #[error("nixos-rebuild failed with exit code: {0}")]
    NixosRebuildFailed(ExitStatus),

    #[error("Failed to read config file: {0}")]
    ConfigRead(#[source] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ConfigParse(#[source] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
struct RebootWindow {
    lower: String,
    upper: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NixosUpgradeConfig {
    #[serde(default)]
    operation: String,

    #[serde(default)]
    flake: Option<String>,

    #[serde(default)]
    channel: Option<String>,

    #[serde(default)]
    flags: Vec<String>,

    #[serde(default, rename = "allowReboot")]
    allow_reboot: bool,

    #[serde(default, rename = "rebootWindow")]
    reboot_window: Option<RebootWindow>,
}

impl Default for NixosUpgradeConfig {
    fn default() -> Self {
        Self {
            operation: "boot".to_string(),
            flake: None,
            channel: None,
            flags: vec!["--no-build-output".to_string()],
            allow_reboot: false,
            reboot_window: None,
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[clap(short, long, default_value = "/etc/nix-upgrade.json")]
    config: PathBuf,

    #[clap(short, long)]
    verbose: bool,
}

fn check_network_available() -> Result<bool, NixosUpgradeError> {
    let dns_servers = ["8.8.8.8:53", "1.1.1.1:53"];

    let mut last_error = None;

    for server in dns_servers {
        match server.parse::<SocketAddr>() {
            Ok(addr) => match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
                Ok(_) => {
                    info!("Network connectivity confirmed via {}", server);
                    return Ok(true);
                }
                Err(e) => {
                    debug!("Failed to connect to {}: {}", server, e);
                    last_error = Some(e);
                }
            },
            Err(e) => debug!("Failed to parse address {}: {}", server, e),
        }
    }

    warn!("No network connectivity detected");

    if let Some(err) = last_error {
        return Err(NixosUpgradeError::NetworkCheck(err));
    }

    Ok(false)
}

fn is_within_reboot_window(window: &RebootWindow) -> Result<bool> {
    let output = Command::new("date")
        .args(["+%H:%M"])
        .output()
        .context("Failed to get current time")?;

    let current_time = String::from_utf8(output.stdout)
        .context("Failed to parse current time")?
        .trim()
        .to_string();

    let lower = &window.lower;
    let upper = &window.upper;

    if lower < upper {
        Ok(current_time > *lower && current_time < *upper)
    } else {
        Ok(current_time < *upper || current_time > *lower)
    }
}

fn run_nixos_upgrade(config: &NixosUpgradeConfig) -> Result<(), NixosUpgradeError> {
    let mut cmd = Command::new("nixos-rebuild");

    cmd.arg(&config.operation);

    if config.flake.is_none() {
        cmd.arg("--upgrade");
    }

    if let Some(flake) = &config.flake {
        cmd.args(["--refresh", "--flake", flake]);
    }

    if let Some(channel) = &config.channel {
        cmd.args(["-I", &format!("nixpkgs={}/nixexprs.tar.xz", channel)]);
    }

    for flag in &config.flags {
        cmd.arg(flag);
    }

    debug!("Running command: {:?}", cmd);

    let status = cmd.status().map_err(NixosUpgradeError::NixosRebuild)?;

    if !status.success() {
        return Err(NixosUpgradeError::NixosRebuildFailed(status));
    }

    if config.allow_reboot && config.operation == "boot" {
        check_and_reboot_if_needed(config)?;
    }

    Ok(())
}

fn check_and_reboot_if_needed(config: &NixosUpgradeConfig) -> Result<(), NixosUpgradeError> {
    let booted = Command::new("readlink")
        .args([
            "-f",
            "/run/booted-system/kernel",
            "/run/booted-system/initrd",
            "/run/booted-system/kernel-modules",
        ])
        .output()
        .map_err(NixosUpgradeError::NixosRebuild)?;

    let built = Command::new("readlink")
        .args([
            "-f",
            "/nix/var/nix/profiles/system/kernel",
            "/nix/var/nix/profiles/system/initrd",
            "/nix/var/nix/profiles/system/kernel-modules",
        ])
        .output()
        .map_err(NixosUpgradeError::NixosRebuild)?;

    if booted.stdout != built.stdout {
        if let Some(window) = &config.reboot_window {
            if let Ok(can_reboot) = is_within_reboot_window(window) {
                if !can_reboot {
                    info!("Outside of configured reboot window, skipping reboot.");
                    return Ok(());
                }
            } else {
                warn!("Failed to check reboot window, proceeding with reboot.");
            }
        }

        info!("Initiating reboot since kernel, initrd or modules have changed");
        Command::new("shutdown")
            .args(["-r", "+1", "NixOS upgrade requires reboot"])
            .status()
            .map_err(NixosUpgradeError::NixosRebuild)?;
    }

    Ok(())
}

fn load_config(path: &PathBuf) -> Result<NixosUpgradeConfig, NixosUpgradeError> {
    if !path.exists() {
        warn!("Config file not found at {:?}, using defaults", path);
        return Ok(NixosUpgradeConfig::default());
    }

    let content = fs::read_to_string(path).map_err(NixosUpgradeError::ConfigRead)?;

    let config = serde_json::from_str(&content).map_err(NixosUpgradeError::ConfigParse)?;

    Ok(config)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(if cli.verbose { "debug" } else { "info" }),
    )
    .init();

    info!("Starting NixOS upgrade on shutdown");

    let config = load_config(&cli.config).context("Failed to load configuration")?;

    debug!("Using configuration: {:?}", config);

    if !check_network_available()? {
        warn!("Network is not available, skipping upgrade");
        return Err(NixosUpgradeError::NetworkUnavailable.into());
    }

    info!("Running NixOS upgrade with operation: {}", config.operation);
    run_nixos_upgrade(&config).context("Failed to upgrade NixOS")?;

    info!("NixOS upgrade completed successfully");
    Ok(())
}
