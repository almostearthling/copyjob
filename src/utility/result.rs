//! A common result type: catching errors from modules used throughout
//! the entire code. The corresponding error carries some information
//! about what went wrong.
#![allow(dead_code)]

use std::{self, fmt};

use crate::constants::*;

/// Transform an error code into a parsable string
pub fn code_to_str_parsable(code: i64) -> &'static str {
    match code {
        ERR_CODE_OK => "OK",
        ERR_CODE_GENERIC => "ERR_GENERIC",
        ERR_CODE_INVALID_CONFIG_FILE => "ERR_INVALID_CONFIG",
        FOERR_GENERIC_FAILURE => "FOERR_GENERIC_FAILURE",
        FOERR_DESTINATION_IS_ITSELF => "FOERR_DESTINATION_IS_ITSELF",
        FOERR_DESTINATION_IS_DIR => "FOERR_DESTINATION_IS_DIR",
        FOERR_DESTINATION_IS_SYMLINK => "FOERR_DESTINATION_IS_SYMLINK",
        FOERR_DESTINATION_IS_NEWER => "FOERR_DESTINATION_IS_NEWER",
        FOERR_DESTINATION_IS_IDENTICAL => "FOERR_DESTINATION_IS_IDENTICAL",
        FOERR_DESTINATION_IS_READONLY => "FOERR_DESTINATION_IS_READONLY",
        FOERR_DESTINATION_EXISTS => "FOERR_DESTINATION_EXISTS",
        FOERR_DESTINATION_NOT_ACCESSIBLE => "FOERR_DESTINATION_NOT_ACCESSIBLE",
        FOERR_CANNOT_CREATE_DIR => "FOERR_CANNOT_CREATE_DIR",
        FOERR_CANNOT_CREATE_FILE => "FOERR_CANNOT_CREATE_FILE",
        FOERR_SOURCE_NOT_EXISTS => "FOERR_SOURCE_NOT_EXISTS",
        FOERR_SOURCE_IS_DIR => "FOERR_SOURCE_IS_DIR",
        FOERR_SOURCE_IS_SYMLINK => "FOERR_SOURCE_IS_SYMLINK",
        FOERR_SOURCE_NOT_ACCESSIBLE => "FOERR_SOURCE_NOT_ACCESSIBLE",
        CJERR_GENERIC_FAILURE => "CJERR_GENERIC_FAILURE",
        CJERR_SOURCE_DIR_NOT_EXISTS => "CJERR_SOURCE_DIR_NOT_EXISTS",
        CJERR_DESTINATION_DIR_NOT_EXISTS => "CJERR_DESTINATION_DIR_NOT_EXISTS",
        CJERR_NO_SOURCE_FILES => "CJERR_NO_SOURCE_FILES",
        CJERR_CANNOT_DETERMINE_DESTFILE => "CJERR_CANNOT_DETERMINE_DESTFILE",
        CJERR_HALT_ON_COPY_ERROR => "CJERR_HALT_ON_COPY_ERROR",
        _ => "ERR_GENERIC",
    }
}

/// Transform an error code into a human readable string
pub fn code_to_str_readable(code: i64) -> &'static str {
    match code {
        ERR_CODE_OK => "application: operation succeeded",
        ERR_CODE_GENERIC => "application: generic failure",
        ERR_CODE_INVALID_CONFIG_FILE => "application: invalid config file",
        FOERR_GENERIC_FAILURE => "file operation: generic failure",
        FOERR_DESTINATION_IS_ITSELF => "file operation: failed attempt to copy on self",
        FOERR_DESTINATION_IS_DIR => "file operation: destination is a directory",
        FOERR_DESTINATION_IS_SYMLINK => "file operation: destination is a symbolic link",
        FOERR_DESTINATION_IS_NEWER => "file operation: destination is more recent than source",
        FOERR_DESTINATION_IS_IDENTICAL => "file operation: destination is identical to source",
        FOERR_DESTINATION_IS_READONLY => "file operation: cannot overwrite destination",
        FOERR_DESTINATION_EXISTS => "file operation: destination exists",
        FOERR_DESTINATION_NOT_ACCESSIBLE => "file operation: destination is not accessible",
        FOERR_CANNOT_CREATE_DIR => "file operation: cannot create directory",
        FOERR_CANNOT_CREATE_FILE => "file operation: cannot create file",
        FOERR_SOURCE_NOT_EXISTS => "file operation: source file does not exist",
        FOERR_SOURCE_IS_DIR => "file operation: source file is a directory",
        FOERR_SOURCE_IS_SYMLINK => "file operation: source file is a symbolic link",
        FOERR_SOURCE_NOT_ACCESSIBLE => "file operation: source file is not accessible",
        CJERR_GENERIC_FAILURE => "copy job: generic failure",
        CJERR_SOURCE_DIR_NOT_EXISTS => "copy job: source directory does not exist",
        CJERR_DESTINATION_DIR_NOT_EXISTS => "copy job: destination does not exist",
        CJERR_NO_SOURCE_FILES => "copy job: no source files found",
        CJERR_CANNOT_DETERMINE_DESTFILE => "copy job: cannot determine source",
        CJERR_HALT_ON_COPY_ERROR => "copy job: ending job after copy error",
        _ => "application: generic failure",
    }
}

// types of specific errors: coming from another crate, all this variety might
// be overkill
// TODO: optimize after refactoring
#[non_exhaustive]
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
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    Native,
    Unit,
    StdIo,
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
                Origin::Unknown => "unknown",
            }
        )
    }
}

/// The error type that is used throughout the application: implementations
/// of the `From` trait are used to implicitly convert from other error
/// types, which in turn set the `origin` property.
#[derive(Debug, Clone)]
pub struct Error {
    kind: Kind,
    origin: Origin,
    code: i64,
    message: String, // freeform message: owned in order to avoid lifetime management
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.origin != Origin::Native {
            write!(
                f,
                "{} ({}): {} / {}",
                self.kind, self.origin, self.code, self.message
            )
        } else {
            write!(f, "{}: {} / {}", self.kind, self.code, self.message)
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
            code: e.raw_os_error().unwrap_or(ERR_CODE_GENERIC as i32) as i64,
            message: e.to_string(),
        }
    }
}

// errors based on the unit type
impl From<()> for Error {
    fn from(_: ()) -> Self {
        Self {
            kind: Kind::Failed,
            origin: Origin::Unit,
            code: ERR_CODE_GENERIC,
            message: ERR_FAILED.to_owned(),
        }
    }
}

// implements `Error` and provides access to properties
impl Error {
    // this is used only to natively create an instance of `Error`: only
    // conversions set the `origin` property to something different
    pub fn new(kind: Kind, code: i64, message: &str) -> Self {
        Self {
            kind,
            origin: Origin::Native,
            code: code,
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

    pub fn code(&self) -> i64 {
        self.code
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
            code: ERR_CODE_GENERIC,
            message: e.to_string(),
        }
    }
}

/// Specific `Result` type that assumes `wres::Error` as its Err variant
pub type Result<T> = std::result::Result<T, Error>;
