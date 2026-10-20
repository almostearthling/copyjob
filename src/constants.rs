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
pub const ERR_INVALID_REGEX: &str = "invalid regular expression";
pub const ERR_MISSING_PARAMETER: &str = "missing parameter";
pub const ERR_INVALID_PARAMETER: &str = "invalid parameter";
pub const ERR_INVALID_PARAMETER_LIST: &str = "invalid list or list element";
pub const ERR_LOGGER_NOT_INITIALIZED: &str = "could not initialize logger";

// values for errors in copy_file, remove_file
pub const FOERR_GENERIC_FAILURE: i64 = 3001;
pub const FOERR_DESTINATION_IS_ITSELF: i64 = 3011;
pub const FOERR_DESTINATION_IS_DIR: i64 = 3012;
pub const FOERR_DESTINATION_IS_SYMLINK: i64 = 3013;
pub const FOERR_DESTINATION_IS_NEWER: i64 = 3014;
pub const FOERR_DESTINATION_IS_IDENTICAL: i64 = 3015;
pub const FOERR_DESTINATION_IS_READONLY: i64 = 3016;
pub const FOERR_DESTINATION_EXISTS: i64 = 3021;
pub const FOERR_DESTINATION_NOT_ACCESSIBLE: i64 = 3022;
pub const FOERR_CANNOT_CREATE_DIR: i64 = 3031;
pub const FOERR_CANNOT_CREATE_FILE: i64 = 3032;
pub const FOERR_SOURCE_NOT_EXISTS: i64 = 3041;
pub const FOERR_SOURCE_IS_DIR: i64 = 3042;
pub const FOERR_SOURCE_IS_SYMLINK: i64 = 3043;
pub const FOERR_SOURCE_NOT_ACCESSIBLE: i64 = 3044;

// values for errors in run_single_job, run_jobs
pub const CJERR_GENERIC_FAILURE: i64 = 2001;
pub const CJERR_SOURCE_DIR_NOT_EXISTS: i64 = 2011;
pub const CJERR_DESTINATION_DIR_NOT_EXISTS: i64 = 2012;
pub const CJERR_NO_SOURCE_FILES: i64 = 2013;
pub const CJERR_CANNOT_DETERMINE_DESTFILE: i64 = 2021;
pub const CJERR_HALT_ON_ERROR: i64 = 2041;

// values for generic errors
pub const ERR_CODE_OK: i64 = 0;
pub const ERR_CODE_NONE: i64 = -1;
pub const ERR_CODE_GENERIC: i64 = 9999;
pub const ERR_CODE_INVALID_CONFIG_FILE: i64 = 9998;
pub const ERR_CODE_TOO_MANY_REPLACEMENTS: i64 = 9997;
pub const ERR_CODE_SOURCE_DEST_OVERLAP: i64 = 9996;

// machine readable codes
pub const SMR_ERR_CODE_OK: &str = "OK";
pub const SMR_ERR_CODE_NONE: &str = "MSG";
pub const SMR_ERR_CODE_GENERIC: &str = "ERR_GENERIC";
pub const SMR_ERR_CODE_INVALID_CONFIG_FILE: &str = "ERR_INVALID_CONFIG";
pub const SMR_ERR_CODE_TOO_MANY_REPLACEMENTS: &str = "ERR_TOO_MANY_REPLACEMENTS";
pub const SMR_ERR_CODE_SOURCE_DEST_OVERLAP: &str = "ERR_SOURCE_DEST_OVERLAP";
pub const SMR_FOERR_GENERIC_FAILURE: &str = "FOERR_GENERIC_FAILURE";
pub const SMR_FOERR_DESTINATION_IS_ITSELF: &str = "FOERR_DESTINATION_IS_ITSELF";
pub const SMR_FOERR_DESTINATION_IS_DIR: &str = "FOERR_DESTINATION_IS_DIR";
pub const SMR_FOERR_DESTINATION_IS_SYMLINK: &str = "FOERR_DESTINATION_IS_SYMLINK";
pub const SMR_FOERR_DESTINATION_IS_NEWER: &str = "FOERR_DESTINATION_IS_NEWER";
pub const SMR_FOERR_DESTINATION_IS_IDENTICAL: &str = "FOERR_DESTINATION_IS_IDENTICAL";
pub const SMR_FOERR_DESTINATION_IS_READONLY: &str = "FOERR_DESTINATION_IS_READONLY";
pub const SMR_FOERR_DESTINATION_EXISTS: &str = "FOERR_DESTINATION_EXISTS";
pub const SMR_FOERR_DESTINATION_NOT_ACCESSIBLE: &str = "FOERR_DESTINATION_NOT_ACCESSIBLE";
pub const SMR_FOERR_CANNOT_CREATE_DIR: &str = "FOERR_CANNOT_CREATE_DIR";
pub const SMR_FOERR_CANNOT_CREATE_FILE: &str = "FOERR_CANNOT_CREATE_FILE";
pub const SMR_FOERR_SOURCE_NOT_EXISTS: &str = "FOERR_SOURCE_NOT_EXISTS";
pub const SMR_FOERR_SOURCE_IS_DIR: &str = "FOERR_SOURCE_IS_DIR";
pub const SMR_FOERR_SOURCE_IS_SYMLINK: &str = "FOERR_SOURCE_IS_SYMLINK";
pub const SMR_FOERR_SOURCE_NOT_ACCESSIBLE: &str = "FOERR_SOURCE_NOT_ACCESSIBLE";
pub const SMR_CJERR_GENERIC_FAILURE: &str = "CJERR_GENERIC_FAILURE";
pub const SMR_CJERR_DESTINATION_DIR_NOT_EXISTS: &str = "CJERR_DESTINATION_DIR_NOT_EXISTS";
pub const SMR_CJERR_SOURCE_DIR_NOT_EXISTS: &str = "CJERR_SOURCE_DIR_NOT_EXISTS";
pub const SMR_CJERR_NO_SOURCE_FILES: &str = "CJERR_NO_SOURCE_FILES";
pub const SMR_CJERR_CANNOT_DETERMINE_DESTFILE: &str = "CJERR_CANNOT_DETERMINE_DESTFILE";
pub const SMR_CJERR_HALT_ON_ERROR: &str = "CJERR_HALT_ON_ERROR";

// human readable codes
pub const SHR_ERR_CODE_OK: &str = "success";
pub const SHR_ERR_CODE_NONE: &str = "message";
pub const SHR_ERR_CODE_GENERIC: &str = "generic failure";
pub const SHR_ERR_CODE_INVALID_CONFIG_FILE: &str = "invalid config file";
pub const SHR_ERR_CODE_TOO_MANY_REPLACEMENTS: &str = "too many variable replacements";
pub const SHR_ERR_CODE_SOURCE_DEST_OVERLAP: &str = "source and destination overlap";
pub const SHR_FOERR_GENERIC_FAILURE: &str = "generic failure";
pub const SHR_FOERR_DESTINATION_IS_ITSELF: &str = "failed attempt to copy on self";
pub const SHR_FOERR_DESTINATION_IS_DIR: &str = "destination is a directory";
pub const SHR_FOERR_DESTINATION_IS_SYMLINK: &str = "destination is a symbolic link";
pub const SHR_FOERR_DESTINATION_IS_NEWER: &str = "destination is more recent than source";
pub const SHR_FOERR_DESTINATION_IS_IDENTICAL: &str = "destination is identical to source";
pub const SHR_FOERR_DESTINATION_IS_READONLY: &str = "cannot overwrite destination";
pub const SHR_FOERR_DESTINATION_EXISTS: &str = "destination exists";
pub const SHR_FOERR_DESTINATION_NOT_ACCESSIBLE: &str = "destination is not accessible";
pub const SHR_FOERR_CANNOT_CREATE_DIR: &str = "cannot create directory";
pub const SHR_FOERR_CANNOT_CREATE_FILE: &str = "cannot create file";
pub const SHR_FOERR_SOURCE_NOT_EXISTS: &str = "source file does not exist";
pub const SHR_FOERR_SOURCE_IS_DIR: &str = "source file is a directory";
pub const SHR_FOERR_SOURCE_IS_SYMLINK: &str = "source file is a symbolic link";
pub const SHR_FOERR_SOURCE_NOT_ACCESSIBLE: &str = "source file is not accessible";
pub const SHR_CJERR_GENERIC_FAILURE: &str = "generic failure";
pub const SHR_CJERR_DESTINATION_DIR_NOT_EXISTS: &str = "destination directory does not exist";
pub const SHR_CJERR_SOURCE_DIR_NOT_EXISTS: &str = "source directory does not exist";
pub const SHR_CJERR_NO_SOURCE_FILES: &str = "no source files found";
pub const SHR_CJERR_CANNOT_DETERMINE_DESTFILE: &str = "cannot determine destination file";
pub const SHR_CJERR_HALT_ON_ERROR: &str = "ending job after error";

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
