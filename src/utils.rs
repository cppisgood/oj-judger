use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

pub fn unix_time() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("gen_cgroup_name failed")
        .as_millis()
        .to_string()
}

pub fn gen_response<T: Serialize>(code: u32, msg: T) -> Json<Value> {
    Json(json!({
        "code": code,
        "msg": json!(msg)
    }))
}