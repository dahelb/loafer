use std::path::PathBuf;

use config::{Config, File, FileFormat};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoaferConfig {
    pub base_dir: PathBuf,
    pub index_file: PathBuf,
    pub max_connections: usize,
}

pub fn get_config() -> LoaferConfig {
    let builder = Config::builder().add_source(File::new("config.toml", FileFormat::Toml));

    let config = builder.build().unwrap();

    config.try_deserialize().unwrap()
}
