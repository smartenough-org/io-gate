use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IOConfig {
    pub id: u8,
    #[serde(rename = "type")]
    pub io_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceConfig {
    pub addr: u8,
    pub outputs: HashMap<String, IOConfig>,
    pub inputs: HashMap<String, IOConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub devices: HashMap<String, DeviceConfig>,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(filename: P) -> anyhow::Result<Self> {
        let handle = File::open(filename)?;
        let data: Config = serde_yaml::from_reader(handle)?;

        Ok(data)
    }
}
