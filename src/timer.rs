use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("gen_cgroup_name failed")
        .as_millis()
}
