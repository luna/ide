//! Utilities for DOM events.

pub mod listener;

use crate::prelude::*;

use js_sys::Function;
use web_sys::EventTarget;



// =============
// === Event ===
// =============

/// This trait represents a type of event that may fire from some specific JS `EventTarget`.
///
/// For example, `WebSocket.close` is such an event, where `close` is event type and `WebSocket` is
/// the `EventTarget`.
pub trait Type {
    /// The type of the event -- it will be the type of value passed to the event listeners.
    /// For example `web_sys::CloseEvent`.
    type Interface : AsRef<web_sys::Event>;

    /// The type of the EventTarget object that fires this type of event, e.g. `web_sys::WebSocket`.
    type Target : AsRef<EventTarget> + AsRef<JsValue> + Clone + PartialEq;

    /// The type of the event as a string. For example `"close"`.
    const NAME:&'static str;

    /// Add a given function to the event's target as an event listener. It will be called each
    /// time event fires until listener is removed through `remove_listener`.
    fn add_listener(target:&Self::Target, listener:&Function) {
        EventTarget::add_event_listener_with_callback(target.as_ref(), Self::NAME, listener).unwrap()
    }

    /// Remove the event listener. The `add_listener` method should have been called before with
    /// the very same function argument.
    fn remove_listener(target:&Self::Target, listener:&Function) {
        EventTarget::remove_event_listener_with_callback(target.as_ref(), Self::NAME, listener).unwrap()
    }
}