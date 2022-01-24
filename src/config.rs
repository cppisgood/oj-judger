use config::{Config, File};
use once_cell::sync::OnceCell;
use std::sync::{Once, RwLock, RwLockReadGuard};

static CONFIG: OnceCell<RwLock<Config>> = OnceCell::new();
static CONFIG_INIT: Once = Once::new();

pub fn get_config() -> RwLockReadGuard<'static, Config> {
    CONFIG_INIT.call_once(|| {
        let mut config = Config::default();
        config
            .merge(vec![File::with_name("config/config.toml")])
            .unwrap();
        CONFIG.set(RwLock::new(config)).unwrap();
    });

    CONFIG.get().unwrap().read().unwrap()
}

#[test]
fn test_config_init() {
    println!("{:?}", get_config().get_table("sandbox"));
    println!("{:?}", get_config().get_str("sandbox.jail_path").unwrap());
}
