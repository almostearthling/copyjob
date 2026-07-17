// generic constants

pub const ERR_FAILED: &str = "failed";

pub const ERR_INVALID_CONFIG: &str = "invalid configuration";

pub const ERR_INVALID_CFG_ENTRY: &str = "invalid configuration entry";
pub const ERR_MISSING_PARAMETER: &str = "missing parameter";
pub const ERR_INVALID_PARAMETER: &str = "invalid parameter";
pub const ERR_INVALID_PARAMETER_LIST: &str = "invalid list or list element";

// pub const ERR_INVALID_VALUE_FOR: &str = "invalid value for";
pub const ERR_INVALID_VALUE_FOR_ENTRY: &str = "invalid value for entry";
pub const ERR_INVALID_VALUE_FOR_LIST_ENTRY: &str = "invalid value for list entry";

pub const STR_UNKNOWN_VALUE: &str = "<unknown>";
pub const STR_INVALID_TYPE: &str = "<invalid_type>";
pub const STR_INVALID_VALUE: &str = "<invalid_value>";


// specific constants

// Values for Outcome::Error (copy_file, remove_file)
pub const FOERR_GENERIC_FAILURE: u64 = 1001;
pub const FOERR_DESTINATION_IS_ITSELF: u64 = 1011;
pub const FOERR_DESTINATION_IS_DIR: u64 = 1012;
pub const FOERR_DESTINATION_IS_SYMLINK: u64 = 1013;
pub const FOERR_DESTINATION_IS_NEWER: u64 = 1014;
pub const FOERR_DESTINATION_IS_IDENTICAL: u64 = 1015;
pub const FOERR_DESTINATION_IS_READONLY: u64 = 1016;
pub const FOERR_DESTINATION_EXISTS: u64 = 1021;
pub const FOERR_DESTINATION_NOT_ACCESSIBLE: u64 = 1022;
pub const FOERR_CANNOT_CREATE_DIR: u64 = 1031;
pub const FOERR_CANNOT_CREATE_FILE: u64 = 1032;
pub const FOERR_SOURCE_NOT_EXISTS: u64 = 1041;
pub const FOERR_SOURCE_IS_DIR: u64 = 1042;
pub const FOERR_SOURCE_IS_SYMLINK: u64 = 1043;
pub const FOERR_SOURCE_NOT_ACCESSIBLE: u64 = 1044;

// values for Outcome::Error (run_single_job, run_jobs)
pub const CJERR_GENERIC_FAILURE: u64 = 2001;
pub const CJERR_SOURCE_DIR_NOT_EXISTS: u64 = 2011;
pub const CJERR_DESTINATION_DIR_NOT_EXISTS: u64 = 2012;
pub const CJERR_NO_SOURCE_FILES: u64 = 2013;
pub const CJERR_CANNOT_DETERMINE_DESTFILE: u64 = 2021;
pub const CJERR_HALT_ON_COPY_ERROR: u64 = 2041;

// values for generic outcomes
pub const ERR_CODE_OK: u64 = 0;
pub const ERR_CODE_GENERIC: u64 = 9999;
pub const ERR_CODE_INVALID_CONFIG_FILE: u64 = 9998;

// context identifiers for output
pub const CONTEXT_MAIN: &str = "MAIN";
pub const CONTEXT_JOB: &str = "JOB";
pub const CONTEXT_TASK: &str = "TASK";

// operation identifiers for output
pub const OPERATION_JOB_COPY: &str = "COPY";
pub const OPERATION_JOB_DEL: &str = "DEL";
pub const OPERATION_JOB_BEGIN: &str = "BEGIN_JOB";
pub const OPERATION_JOB_END: &str = "END_JOB";
pub const OPERATION_MAIN_END: &str = "END_MAIN";
pub const OPERATION_CONFIG: &str = "CONFIG";

// end.
