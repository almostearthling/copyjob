//! A common result type: catching errors from modules used throughout
//! the entire code. The corresponding error carries some information
//! about what went wrong.
#![allow(dead_code)]

use bstr::*;
use std::{self, fmt};

use crate::constants::*;

// types of specific errors: coming from another crate, all this variety might
// be overkill
// TODO: optimize after refactoring
#[derive(Debug, Clone)]
pub enum Kind {
    Forbidden,
    Unsupported,
    Unavailable,
    Unconverted,
    Unparsed,
    Busy,
    Invalid,
    Failed,
    Empty,
    // ...
    Unknown,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Kind::Forbidden => "not permitted",
                Kind::Unsupported => "not supported",
                Kind::Unavailable => "not available",
                Kind::Unconverted => "not converted",
                Kind::Unparsed => "not parsed",
                Kind::Busy => "resource busy",
                Kind::Invalid => "invalid",
                Kind::Failed => "failed",
                Kind::Empty => "empty",
                Kind::Unknown => "unknown",
            }
        )
    }
}

/// Describes the origin of the error: if `Native` the error was originated
/// natively, otherwise the field is set by another error that is converted
/// into `Error` via a dedicated `From` trait implementation.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    Native,
    Unit,
    StdIo,
    Utf8,
    Unknown,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Origin::Native => "self",
                Origin::Unit => "unit",
                Origin::StdIo => "io",
                Origin::Utf8 => "utf8",
                Origin::Unknown => "unknown",
            }
        )
    }
}

/// The error type that is used throughout the implementations
/// of the `From` trait are used to implicitly convert from other error
/// types, which in turn set the `origin` property.
#[derive(Debug, Clone)]
pub struct Error {
    kind: Kind,
    origin: Origin,
    message: String, // freeform message: owned in order to avoid lifetime management
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.origin != Origin::Native {
            write!(
                f,
                "{} ({}): {}",
                self.kind, self.origin, self.message
            )
        } else {
            write!(f, "{}: {}", self.kind, self.message)
        }
    }
}

// maybe the most important: From<std::io::Error>
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: match e.kind() {
                std::io::ErrorKind::Unsupported => Kind::Unsupported,
                std::io::ErrorKind::PermissionDenied => Kind::Forbidden,
                std::io::ErrorKind::InvalidData => Kind::Invalid,
                std::io::ErrorKind::InvalidInput => Kind::Invalid,
                _ => Kind::Unknown,
            },
            origin: Origin::StdIo,
            message: e.to_string(),
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Self {
            kind: Kind::Unconverted,
            origin: Origin::Utf8,
            message: value.to_string(),
        }
    }
}

// errors based on the unit type
impl From<()> for Error {
    fn from(_: ()) -> Self {
        Self {
            kind: Kind::Failed,
            origin: Origin::Unit,
            message: RES_FAILED.to_owned(),
        }
    }
}

// implements `Error` and provides access to properties
impl Error {
    // this is used only to natively create an instance of `Error`: only
    // conversions set the `origin` property to something different
    pub fn new(kind: Kind, message: &str) -> Self {
        Self {
            kind,
            origin: Origin::Native,
            message: message.to_string(),
        }
    }

    // property access
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

// possible last resort to allow conversions from pointers to errors
impl<T: std::error::Error> From<Box<T>> for Error {
    fn from(e: Box<T>) -> Self {
        Self {
            kind: Kind::Unknown,
            origin: Origin::Unknown,
            message: e.to_string(),
        }
    }
}

/// Specific `Result` type that assumes `result::Error` as its Err variant
pub type Result<T> = std::result::Result<T, Error>;
