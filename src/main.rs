use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
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
struct NixosUpgradeConfig {
    #[serde(default)]
    operation: String,

    #[serde(default)]
    flake: Option<String>,

    #[serde(default)]
    channel: Option<String>,

    #[serde(default)]
    flags: Vec<String>,
}

impl Default for NixosUpgradeConfig {
    fn default() -> Self {
        Self {
            operation: "boot".to_string(),
            flake: None,
            channel: None,
            flags: vec!["--no-build-output".to_string()],
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
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .map_err(NixosUpgradeError::NetworkCheck)?;

    Ok(!output.stdout.is_empty())
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
