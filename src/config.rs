use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use anyhow::{Result, Context};

#[derive(Deserialize, Clone)]
pub struct Config {
    pub token: String,
    pub webhooks: BTreeMap<String, String>,
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
