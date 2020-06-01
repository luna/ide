#![deny(unconditional_recursion)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_import_braces)]

//! This crate contains implementation of logging interface.

pub mod disabled;
pub mod enabled;

use enso_prelude::*;



// ==============
// === LogMsg ===
// ==============

/// Message that can be logged.
pub trait LogMsg {
    /// Turns message into `&str` and passes it to input function.
    fn with_log_msg<F: FnOnce(&str) -> T, T>(&self, f:F) -> T;
}

impl LogMsg for &str {
    fn with_log_msg<F: FnOnce(&str) -> T, T>(&self, f:F) -> T {
        f(self)
    }
}

impl<F: Fn() -> S, S:Str> LogMsg for F {
    fn with_log_msg<G: FnOnce(&str) -> T, T>(&self, f:G) -> T {
        f(self().as_ref())
    }
}



// ==============
// === Logger ===
// ==============

/// Interface common to all loggers.
pub trait AnyLogger {
    /// Path that is used as an unique identifier of this logger.
    fn path(&self) -> &str;

    /// Creates a new logger. Path should be a unique identifier for this logger.
    fn new(path:impl Str) -> Self;

    /// Creates a new logger with this logger as a parent.
    fn sub(logger:&impl AnyLogger, path:impl Str) -> Self where Self:Sized {
        if logger.path().is_empty() {Self::new(path)} else {
            Self::new(format!("{}.{}", logger.path(), path.as_ref()))
        }
    }

    /// Evaluates function `f` and visually groups all logs will occur during its execution.
    fn group<M:LogMsg,T,F:FnOnce() -> T>(&self, msg: M, f:F) -> T {
        self.group_begin(msg);
        let out = f();
        self.group_end();
        out
    }

    /// Log with stacktrace and level:info.
    fn trace<M:LogMsg>(&self, msg:M);
    /// Log with level:debug
    fn debug<M:LogMsg>(&self, msg:M);
    /// Log with level:info.
    fn info<M:LogMsg>(&self, msg:M);
    /// Log with level:warning.
    fn warning<M:LogMsg>(&self, msg:M);
    /// Log with level:error.
    fn error<M:LogMsg>(&self, msg:M);
    /// Visually groups all logs between group_begin and group_end.
    fn group_begin<M:LogMsg>(&self, msg:M);
    /// Visually groups all logs between group_begin and group_end.
    fn group_end(&self);
}



// ==============
// === Macros ===
// ==============

/// Shortcut for `|| format!(..)`.
#[macro_export]
macro_rules! fmt {
    ($($arg:tt)*) => (||(format!($($arg)*)))
}

/// Evaluates expression and visually groups all logs will occur during its execution.
#[macro_export]
macro_rules! group {
    ($logger:expr, $message:tt, {$($body:tt)*}) => {{
        let __logger = $logger.clone();
        __logger.group_begin(|| iformat!{$message});
        let out = {$($body)*};
        __logger.group_end();
        out
    }};
}

/// Logs a message on on given level.
#[macro_export]
macro_rules! log_template {
    ($method:ident $logger:expr, $message:tt $($rest:tt)*) => {
        $crate::log_template_impl! {$method $logger, iformat!($message) $($rest)*}
    };
}

/// Logs a message on on given level.
#[macro_export]
macro_rules! log_template_impl {
    ($method:ident $logger:expr, $expr:expr) => {{
        $logger.$method(|| $expr);
    }};
    ($method:ident $logger:expr, $expr:expr, $body:tt) => {{
        let __logger = $logger.clone();
        __logger.group_begin(|| $expr);
        let out = $body;
        __logger.group_end();
        out
    }};
}

/// Logs an internal error with descriptive message.
#[macro_export]
macro_rules! with_internal_bug_message { ($f:ident $($args:tt)*) => { $crate::$f! {
"This is a bug. Please report it and and provide us with as much information as \
possible at https://github.com/luna/enso/issues. Thank you!"
$($args)*
}};}

/// Logs an internal error.
#[macro_export]
macro_rules! log_internal_bug_template {
    ($($toks:tt)*) => {
        $crate::with_internal_bug_message! { log_internal_bug_template_impl $($toks)* }
    };
}

/// Logs an internal error.
#[macro_export]
macro_rules! log_internal_bug_template_impl {
    ($note:tt $method:ident $logger:expr, $message:tt $($rest:tt)*) => {
        $crate::log_template_impl! {$method $logger,
            format!("Internal Error. {}\n\n{}",iformat!($message),$note) $($rest)*
        }
    };
}

/// Log with stacktrace and level:info.
#[macro_export]
macro_rules! trace {
    ($($toks:tt)*) => {
        $crate::log_template! {trace $($toks)*}
    };
}

/// Log with level:debug
#[macro_export]
macro_rules! debug {
    ($($toks:tt)*) => {
        $crate::log_template! {debug $($toks)*}
    };
}

/// Log with level:info.
#[macro_export]
macro_rules! info {
    ($($toks:tt)*) => {
        $crate::log_template! {info $($toks)*}
    };
}

/// Log with level:warning.
#[macro_export]
macro_rules! warning {
    ($($toks:tt)*) => {
        $crate::log_template! {warning $($toks)*}
    };
}

/// Log with level:error.
#[macro_export]
macro_rules! error {
    ($($toks:tt)*) => {
        $crate::log_template! {error $($toks)*}
    };
}

/// Logs an internal warning.
#[macro_export]
macro_rules! internal_warning {
    ($($toks:tt)*) => {
        $crate::log_internal_bug_template! {warning $($toks)*}
    };
}
