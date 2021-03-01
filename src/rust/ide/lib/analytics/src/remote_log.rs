//! Provides an API to send data to our remote logging service. Requires the remote logging
//! to be already set up on the JS side. That means, there needs to exist a `window.enso.remoteLog`
//! method that takes a string and does the actual logging.

use crate::data::*;

pub use wasm_bindgen::prelude::*;



#[wasm_bindgen(inline_js="
export function _remote_log(msg, data) {
    if (window !== undefined && window.enso !== undefined && window.enso.remoteLog !== undefined) {
        try {
            window.enso.remoteLog(msg,{data})
        } catch (error) {
            console.error(\"Error while logging message. \" + error );
        }

    } else {
        console.warn(\"Failed to send log message.\")
    }
}

export function _register(data) {
    if (window !== undefined && window.enso !== undefined && window.enso.remoteLog !== undefined) {
        try {
            window.enso.register(msg,data)
        } catch (error) {
            console.error(\"Error while logging message. \" + error );
        }

    } else {
        console.warn(\"Failed to send log message.\")
    }
}
")]
extern "C" {
     #[allow(unsafe_code)]
     fn _remote_log(msg:JsValue,data:JsValue);

    #[allow(unsafe_code)]
    fn _register(msg:JsValue);
}

/// Send the provided public event to our logging service.
pub fn remote_log(message:&str) {
    _remote_log(JsValue::from(message.to_string()), JsValue::NULL);
}

/// Send the provided public event with data to our logging service.
pub fn remote_log_data
<T:Loggable>(message:&str, data:AnonymousData<T>) {
    _remote_log(JsValue::from(message.to_string()), data.0.get());
}

/// Send the provided public event to our logging service.
pub fn register(message:&str) {
    _register(JsValue::from(message.to_string()));
}

