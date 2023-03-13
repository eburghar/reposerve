use actix_schemeredirect_middleware::data::{Redirect, StrictTransportSecurity};
use actix_token_middleware::data::Jwt;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, path::PathBuf};

#[derive(Deserialize, Clone)]
/// Tls configuration
pub struct Tls {
	/// crt path
	pub crt: PathBuf,
	/// key path
	pub key: PathBuf,
	/// redirect configuration
	pub redirect: Option<Redirect>,
	/// hsts configuration
	pub hsts: Option<StrictTransportSecurity>,
}

/// Configuration of reposerve
#[derive(Deserialize, Clone)]
pub struct Config {
	/// root dir of the repository
	pub dir: PathBuf,
	/// use tls
	pub tls: Option<Tls>,
	/// webhooks configuration
	pub webhooks: Option<HashMap<String, String>>,
	/// jwt configuration
	pub jwt: Option<Jwt>,
}

impl Config {
	pub fn read(config: &str) -> Result<Config> {
		// open configuration file
		let file = File::open(config).with_context(|| format!("Can't open {}", &config))?;
		// deserialize configuration
		let config: Config =
			serde_yaml::from_reader(file).with_context(|| format!("Can't read {}", &config))?;
		Ok(config)
	}
}
