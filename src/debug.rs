use wasm_bindgen::JsValue;
use web_sys::console;

pub fn log(msg: &str) {
    if !msg.is_empty() {
        console::log_1(&JsValue::from_str(msg));
    };
}
