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
pub const ERR_FAILED: &str = "failed";

pub const ERR_INVALID_CONFIG: &str = "invalid configuration";
pub const ERR_INVALID_CFG_ENTRY: &str = "invalid configuration entry";
pub const ERR_MISSING_PARAMETER: &str = "missing parameter";
pub const ERR_INVALID_PARAMETER: &str = "invalid parameter";
pub const ERR_INVALID_PARAMETER_LIST: &str = "invalid list or list element";
pub const ERR_LOGGER_NOT_INITIALIZED: &str = "could not initialize logger";

// values for Outcome::Error (copy_file, remove_file)
pub const FOERR_GENERIC_FAILURE: i64 = 1001;
pub const FOERR_DESTINATION_IS_ITSELF: i64 = 1011;
pub const FOERR_DESTINATION_IS_DIR: i64 = 1012;
pub const FOERR_DESTINATION_IS_SYMLINK: i64 = 1013;
pub const FOERR_DESTINATION_IS_NEWER: i64 = 1014;
pub const FOERR_DESTINATION_IS_IDENTICAL: i64 = 1015;
pub const FOERR_DESTINATION_IS_READONLY: i64 = 1016;
pub const FOERR_DESTINATION_EXISTS: i64 = 1021;
pub const FOERR_DESTINATION_NOT_ACCESSIBLE: i64 = 1022;
pub const FOERR_CANNOT_CREATE_DIR: i64 = 1031;
pub const FOERR_CANNOT_CREATE_FILE: i64 = 1032;
pub const FOERR_SOURCE_NOT_EXISTS: i64 = 1041;
pub const FOERR_SOURCE_IS_DIR: i64 = 1042;
pub const FOERR_SOURCE_IS_SYMLINK: i64 = 1043;
pub const FOERR_SOURCE_NOT_ACCESSIBLE: i64 = 1044;

// values for Outcome::Error (run_single_job, run_jobs)
pub const CJERR_GENERIC_FAILURE: i64 = 2001;
pub const CJERR_SOURCE_DIR_NOT_EXISTS: i64 = 2011;
pub const CJERR_DESTINATION_DIR_NOT_EXISTS: i64 = 2012;
pub const CJERR_NO_SOURCE_FILES: i64 = 2013;
pub const CJERR_CANNOT_DETERMINE_DESTFILE: i64 = 2021;
pub const CJERR_HALT_ON_ERROR: i64 = 2041;

// values for generic outcomes
pub const ERR_CODE_OK: i64 = 0;
pub const ERR_CODE_NONE: i64 = -1;
pub const ERR_CODE_GENERIC: i64 = 9999;
pub const ERR_CODE_INVALID_CONFIG_FILE: i64 = 9998;

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
