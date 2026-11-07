// generic constants
#![allow(dead_code)]

// The application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ERR_INVALID_VALUE_FOR_ENTRY: &str = "invalid value for entry";
pub const ERR_INVALID_VALUE_FOR_LIST_ENTRY: &str = "invalid value for list entry";

pub const STR_UNDEFINED_VALUE: &str = "<undefined>";
pub const STR_UNKNOWN_VALUE: &str = "<unknown>";
pub const STR_INVALID_TYPE: &str = "<invalid_type>";
pub const STR_INVALID_VALUE: &str = "<invalid_value>";

// errors
pub const RES_OK: &str = "success";
pub const RES_FAILED: &str = "failed";

pub const ERR_INVALID_CONFIG: &str = "invalid configuration";
pub const ERR_INVALID_CFG_ENTRY: &str = "invalid configuration entry";
pub const ERR_INVALID_VARNAME: &str = "invalid variable name";
pub const ERR_INVALID_REGEX: &str = "invalid regular expression";
pub const ERR_MISSING_PARAMETER: &str = "missing parameter";
pub const ERR_INVALID_PARAMETER: &str = "invalid parameter";
pub const ERR_INVALID_PARAMETER_LIST: &str = "invalid list or list element";
pub const ERR_DUPLICATE_JOB_NAME: &str = "duplicate job name";
pub const ERR_LOGGER_NOT_INITIALIZED: &str = "could not initialize logger";
pub const ERR_RESULTING_DIR_EMPTY: &str = "resulting directory name is empty";
pub const ERR_GENERIC: &str = "generic failure";
pub const ERR_TOO_MANY_REPLACEMENTS: &str = "too many variable replacements";
pub const ERR_SOURCE_DEST_OVERLAP: &str = "source and destination overlap";
pub const ERR_LOCAL_VAR_UNDEFINED: &str = "local variable used but not defined";
pub const ERR_DESTINATION_IS_ITSELF: &str = "failed attempt to copy on self";
pub const ERR_DESTINATION_IS_DIR: &str = "destination is a directory";
pub const ERR_DESTINATION_IS_SYMLINK: &str = "destination is a symbolic link";
pub const ERR_DESTINATION_IS_NEWER: &str = "destination is more recent than source";
pub const ERR_DESTINATION_IS_IDENTICAL: &str = "destination is identical to source";
pub const ERR_CANNOT_WRITE_DESTINATION: &str = "cannot write to destination";
pub const ERR_DESTINATION_EXISTS: &str = "destination exists";
pub const ERR_DESTINATION_NOT_ACCESSIBLE: &str = "destination is not accessible";
pub const ERR_CANNOT_CREATE_DIR: &str = "cannot create directory";
pub const ERR_CANNOT_CREATE_FILE: &str = "cannot create file";
pub const ERR_SOURCE_NOT_EXISTS: &str = "source file does not exist";
pub const ERR_SOURCE_IS_DIR: &str = "source file is a directory";
pub const ERR_SOURCE_IS_SYMLINK: &str = "source file is a symbolic link";
pub const ERR_SOURCE_NOT_ACCESSIBLE: &str = "source file is not accessible";
pub const ERR_DESTINATION_DIR_NOT_EXISTS: &str = "destination directory does not exist";
pub const ERR_SOURCE_DIR_NOT_EXISTS: &str = "source directory does not exist";
pub const ERR_DESTINATION_NOT_DIR: &str = "destination is not a directory";
pub const ERR_CANNOT_DETERMINE_DESTFILE: &str = "cannot determine destination file";
pub const ERR_HALT_ON_ERROR: &str = "ending job after error";

// operation identifiers for output
pub const LOG_ACTION_COPY: &str = "COPY";
pub const LOG_ACTION_DEL: &str = "DEL";
pub const LOG_ACTION_JOB: &str = "JOB";
pub const LOG_ACTION_OTHER: &str = "OTHER";

pub const LOG_EMITTER_MAIN: &str = "MAIN";
pub const LOG_EMITTER_CONFIG: &str = "CONFIG";
pub const LOG_EMITTER_GLOBAL: &str = "GLOBAL";
pub const LOG_EMITTER_JOB: &str = "JOB";

pub const LOG_WHEN_INIT: &str = "INIT";
pub const LOG_WHEN_START: &str = "START";
pub const LOG_WHEN_END: &str = "END";
pub const LOG_WHEN_PROC: &str = "PROC";

pub const LOG_STATUS_OK: &str = "OK";
pub const LOG_STATUS_FAIL: &str = "FAIL";
pub const LOG_STATUS_IND: &str = "IND";
pub const LOG_STATUS_MSG: &str = "MSG";
pub const LOG_STATUS_ERR: &str = "ERR";

// end.
