use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_time() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("gen_cgroup_name failed")
        .as_millis()
        .to_string()
}
