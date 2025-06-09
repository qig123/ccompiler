// src/common_ids.rs
use once_cell::sync::Lazy;
use std::sync::Mutex;

static SHARED_COUNTER: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

fn next_id() -> usize {
    let mut counter_guard = SHARED_COUNTER.lock().expect("Mutex poisoned");
    let id = *counter_guard;
    *counter_guard += 1;
    id
}

pub fn generate_analysis_variable_name() -> String {
    format!("v_{}", next_id())
}

pub fn generate_translator_temp_name() -> String {
    format!("tmp.{}", next_id())
}

pub fn generate_translator_label_name() -> String {
    format!("abel.{}", next_id())
}
