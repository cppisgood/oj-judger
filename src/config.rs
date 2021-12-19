use config::{Config, File};
use lazy_static::lazy_static;
use std::{
    error::Error,
    sync::{RwLock, RwLockReadGuard},
};

lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}

pub fn config_init() -> Result<(), Box<dyn Error>> {
    let mut settings = SETTINGS.write()?;
    settings.merge(vec![File::with_name("config/config.toml")])?;

    Ok(())
}

pub fn get_config<'a>() -> RwLockReadGuard<'a, Config> {
    SETTINGS.read().expect("get RwLockReadGuard failed")
}

#[test]
fn test_config_init() {
    config_init().unwrap();
}
