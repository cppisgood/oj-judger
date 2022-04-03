use config::{Config, File, FileFormat};
use once_cell::sync::OnceCell;
use std::sync::{Once, RwLock, RwLockReadGuard};

static CONFIG: OnceCell<RwLock<Config>> = OnceCell::new();
static CONFIG_INIT: Once = Once::new();

pub fn get_config() -> RwLockReadGuard<'static, Config> {
    CONFIG_INIT.call_once(|| {
        let config = Config::builder()
            .add_source(File::new("config/config.toml", FileFormat::Toml))
            .build()
            .unwrap();
        CONFIG.set(RwLock::new(config)).unwrap();
    });

    CONFIG.get().unwrap().read().unwrap()
}

#[test]
fn test_config_init() {
    println!("{:?}", get_config().get_table("sandbox"));
    // println!("{:?}", get_config().get_str("sandbox.jail_path").unwrap());
}
