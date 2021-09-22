use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;

/// Configuration of reposerve
#[derive(Deserialize, Clone)]
pub struct Config {
	/// root dir of the repository
	pub dir: PathBuf,
	/// token for uploading packages
	// pub token: String,
	/// use tls
	pub tls: bool,
	/// certificate chain
	pub crt: Option<PathBuf>,
	/// key
	pub key: Option<PathBuf>,
	/// webhooks configuration
	pub webhooks: BTreeMap<String, String>,
	/// jwks endpoint
	pub jwks: String,
	/// claims
	pub claims: BTreeMap<String, String>
}

impl Config {
	pub fn read(config: &str) -> Result<Config> {
		// open configuration file
		let file = File::open(&config).with_context(|| format!("Can't open {}", &config))?;
		// deserialize configuration
		let config: Config =
			serde_yaml::from_reader(file).with_context(|| format!("Can't read {}", &config))?;
		Ok(config)
	}
}
