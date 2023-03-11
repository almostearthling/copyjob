/// copyjob
/// An utility to perform complex copy operations based on TOML files
/// (c) 2023, Francesco Garosi

use std::io::BufReader;
use std::io::Read;
use std::env;

use std::fs;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::metadata;

use lazy_static::lazy_static;

use std::collections::HashMap;
use std::path::PathBuf;

use regex::{Regex, RegexBuilder};

use dirs::home_dir;
use walkdir::WalkDir;

use toml;
use cfgmap::{CfgValue, CfgMap, Condition::*, Checkable};
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};



// Structures used for a copy job configuration and the global configuration:
// values provided in CopyJobConfig default to the ones provided globally in
// the CopyJobGlobalConfig object, and override them if different
#[derive(Debug)]
struct CopyJobConfig {
    job_name: String,               // the job name
    source_dir: PathBuf,            // source directory
    destination_dir: PathBuf,       // destination directory
    include_pattern: String,        // RE pattern of filenames to include
    exclude_pattern: String,        // RE pattern of filenames to exclude
    excludedir_pattern: String,     // RE pattern of directories to skip
    recursive: bool,                // recurse directories
    case_sensitive: bool,           // consider filenames as case sensitive
    follow_symlinks: bool,          // follow symlinks
    overwrite: bool,                // possibly overwrite destination
    skip_newer: bool,               // do not overwrite more recent files
    check_content: bool,            // check whether contents are the same
    remove_others_matching: bool,   // remove matching files not present in source
    create_directories: bool,       // create non-existing directories
    keep_structure: bool,           // keep directory structure as in source
    halt_on_errors: bool,           // exit job if an error occurs
}

#[derive(Debug)]
struct CopyJobGlobalConfig {
    active_jobs: Vec<String>,           // list of active jobs in config (names)
    job_list: Vec<String>,              // list of all job names found in config
    variables: HashMap<String, String>, // variables/values defined in config
    recursive: bool,                    // recurse directories
    case_sensitive: bool,               // consider filenames as case sensitive
    follow_symlinks: bool,              // follow symlinks
    overwrite: bool,                    // possibly overwrite destination
    skip_newer: bool,                   // do not overwrite more recent files
    check_content: bool,                // check whether contents are the same
    remove_others_matching: bool,       // remove matching files not present in source
    create_directories: bool,           // create non-existing directories
    keep_structure: bool,               // keep directory structure as in source
    halt_on_errors: bool,               // exit job if an error occurs

    // the following parameters are defined through CLI arguments only
    config_file: PathBuf,               // configuration file path
    verbose: bool,                      // provide output while running
    parsable_output: bool,              // provide machine-readable output
}


// Holds a result for file op to be choosen among the following ones: it
// does not contain the word Result in the definition as it is not related
// to the plethora of *::Result outcomes used in Rust (though it indicates
// either Success or a specified error)
#[derive(Debug)]
enum FileOpOutcome {
    Success,
    Error(u64),     // will report a code from the following list
}

// Values for FileOpOutcome::Error
const FOERR_GENERIC_FAILURE: u64 = 1001;
const FOERR_DESTINATION_IS_ITSELF: u64 = 1011;
const FOERR_DESTINATION_IS_DIR: u64 = 1012;
const FOERR_DESTINATION_IS_SYMLINK: u64 = 1013;
const FOERR_DESTINATION_IS_NEWER: u64 = 1014;
const FOERR_DESTINATION_IS_IDENTICAL: u64 = 1015;
const FOERR_DESTINATION_IS_READONLY: u64 = 1016;
const FOERR_DESTINATION_EXISTS: u64 = 1021;
const FOERR_DESTINATION_NOT_ACCESSIBLE: u64 = 1022;
const FOERR_CANNOT_CREATE_DIR: u64 = 1031;
const FOERR_CANNOT_CREATE_FILE: u64 = 1032;
const FOERR_SOURCE_NOT_EXISTS: u64 = 1041;
const FOERR_SOURCE_IS_DIR: u64 = 1042;
const FOERR_SOURCE_IS_SYMLINK: u64 = 1043;
const FOERR_SOURCE_NOT_ACCESSIBLE: u64 = 1044;


// Holds a result for a copy job operation
#[derive(Debug)]
enum CopyJobOutcome {
    Success,
    Error(u64),     // will report a code from the following
}

// values for CopyJobOutcome::Error
const CJERR_GENERIC_FAILURE: u64 = 2001;
const CJERR_SOURCE_DIR_NOT_EXISTS: u64 = 2011;
const CJERR_DESTINATION_DIR_NOT_EXISTS: u64 = 2012;
const CJERR_NO_SOURCE_FILES: u64 = 2013;
const CJERR_CANNOT_DETERMINE_DESTFILE: u64 = 2021;
const CJERR_HALT_ON_COPY_ERROR: u64 = 2041;


// value for generic outcomes
const ERR_OK: u64 = 0;
const ERR_GENERIC: u64 = 9999;
const ERR_INVALID_CONFIG_FILE: u64 = 9998;



// Some constants used within the code
lazy_static! {
    // directory markers: any of the values in respective lists, when
    // used in the source and destination directory specs, will expand
    // into the appropriate fully qualified path, namely:
    //  USER-HOME => $HOME / %USERPROFILE%
    //  CONFIG-FILE-DIR => where the current config file is located
    static ref DIR_MARKERS: HashMap<&'static str, Vec<&'static str>> = {
        let mut _tmap = HashMap::new();
        _tmap.insert("USER-HOME", vec![r"~/", r"~\"]);
        _tmap.insert("CONFIG-FILE-DIR", vec![r"@/", r"@\"]);
        _tmap
    };

    // error strings (parsable version)
    static ref ERRS_PARSABLE: HashMap<u64, &'static str> = {
        let mut _tmap = HashMap::new();
        _tmap.insert(FOERR_GENERIC_FAILURE, "FOERR_GENERIC_FAILURE");
        _tmap.insert(FOERR_DESTINATION_IS_ITSELF, "FOERR_DESTINATION_IS_ITSELF");
        _tmap.insert(FOERR_DESTINATION_IS_DIR, "FOERR_DESTINATION_IS_DIR");
        _tmap.insert(FOERR_DESTINATION_IS_SYMLINK, "FOERR_DESTINATION_IS_SYMLINK");
        _tmap.insert(FOERR_DESTINATION_IS_NEWER, "FOERR_DESTINATION_IS_NEWER");
        _tmap.insert(FOERR_DESTINATION_IS_IDENTICAL, "FOERR_DESTINATION_IS_IDENTICAL");
        _tmap.insert(FOERR_DESTINATION_IS_READONLY, "FOERR_DESTINATION_IS_READONLY");
        _tmap.insert(FOERR_DESTINATION_EXISTS, "FOERR_DESTINATION_EXISTS");
        _tmap.insert(FOERR_DESTINATION_NOT_ACCESSIBLE, "FOERR_DESTINATION_NOT_ACCESSIBLE");
        _tmap.insert(FOERR_CANNOT_CREATE_DIR, "FOERR_CANNOT_CREATE_DIR");
        _tmap.insert(FOERR_CANNOT_CREATE_FILE, "FOERR_CANNOT_CREATE_FILE");
        _tmap.insert(FOERR_SOURCE_NOT_EXISTS, "FOERR_SOURCE_NOT_EXISTS");
        _tmap.insert(FOERR_SOURCE_IS_DIR, "FOERR_SOURCE_IS_DIR");
        _tmap.insert(FOERR_SOURCE_IS_SYMLINK, "FOERR_SOURCE_IS_SYMLINK");
        _tmap.insert(FOERR_SOURCE_NOT_ACCESSIBLE, "FOERR_SOURCE_NOT_ACCESSIBLE");

        _tmap.insert(CJERR_GENERIC_FAILURE, "CJERR_GENERIC_FAILURE");
        _tmap.insert(CJERR_SOURCE_DIR_NOT_EXISTS, "CJERR_SOURCE_DIR_NOT_EXISTS");
        _tmap.insert(CJERR_DESTINATION_DIR_NOT_EXISTS, "CJERR_DESTINATION_DIR_NOT_EXISTS");
        _tmap.insert(CJERR_NO_SOURCE_FILES, "CJERR_NO_SOURCE_FILES");
        _tmap.insert(CJERR_CANNOT_DETERMINE_DESTFILE, "CJERR_CANNOT_DETERMINE_DESTFILE");
        _tmap.insert(CJERR_HALT_ON_COPY_ERROR, "CJERR_HALT_ON_COPY_ERROR");

        _tmap.insert(ERR_INVALID_CONFIG_FILE, "ERR_INVALID_CONFIG");
        _tmap.insert(ERR_GENERIC, "ERR_GENERIC");
        _tmap.insert(ERR_OK, "OK");
        _tmap
    };

    // error strings (verbose version)
    static ref ERRS_VERBOSE: HashMap<u64, &'static str> = {
        let mut _tmap = HashMap::new();
        _tmap.insert(FOERR_GENERIC_FAILURE, "file operation: generic failure");
        _tmap.insert(FOERR_DESTINATION_IS_ITSELF, "file operation: failed attempt to copy on self");
        _tmap.insert(FOERR_DESTINATION_IS_DIR, "file operation: destination is a directory");
        _tmap.insert(FOERR_DESTINATION_IS_SYMLINK, "file operation: destination is a symbolic link");
        _tmap.insert(FOERR_DESTINATION_IS_NEWER, "file operation: destination is more recent than source");
        _tmap.insert(FOERR_DESTINATION_IS_IDENTICAL, "file operation: destination is identical to source");
        _tmap.insert(FOERR_DESTINATION_IS_READONLY, "file operation: cannot overwrite destination");
        _tmap.insert(FOERR_DESTINATION_EXISTS, "file operation: destination exists");
        _tmap.insert(FOERR_DESTINATION_NOT_ACCESSIBLE, "file operation: destination is not accessible");
        _tmap.insert(FOERR_CANNOT_CREATE_DIR, "file operation: cannot create directory");
        _tmap.insert(FOERR_CANNOT_CREATE_FILE, "file operation: cannot create file");
        _tmap.insert(FOERR_SOURCE_NOT_EXISTS, "file operation: source file does not exist");
        _tmap.insert(FOERR_SOURCE_IS_DIR, "file operation: source file is a directory");
        _tmap.insert(FOERR_SOURCE_IS_SYMLINK, "file operation: source file is a symbolic link");
        _tmap.insert(FOERR_SOURCE_NOT_ACCESSIBLE, "file operation: source file is not accessible");

        _tmap.insert(CJERR_GENERIC_FAILURE, "copy job: generic failure");
        _tmap.insert(CJERR_SOURCE_DIR_NOT_EXISTS, "copy job: source directory does not exist");
        _tmap.insert(CJERR_DESTINATION_DIR_NOT_EXISTS, "copy job: destination does not exist");
        _tmap.insert(CJERR_NO_SOURCE_FILES, "copy job: no source files found");
        _tmap.insert(CJERR_CANNOT_DETERMINE_DESTFILE, "copy job: cannot determine source");
        _tmap.insert(CJERR_HALT_ON_COPY_ERROR, "copy job: ending job after copy error");

        _tmap.insert(ERR_INVALID_CONFIG_FILE, "application: invalid config file");
        _tmap.insert(ERR_GENERIC, "application: generic failure");
        _tmap.insert(ERR_OK, "application: operation succeeded");
        _tmap
    };

    static ref STR_MATCH_NO_FILE: String = String::from(r"^\*$");

    static ref RE_VARNAME: Regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    static ref RE_JOBNAME: Regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    static ref RE_MATCH_NO_FILE: Regex = Regex::new(&STR_MATCH_NO_FILE).unwrap();

    // variable mention expressions: *_LOC is the mention of a variable
    // defined in the configuration file, *_ENV is the mention of a variable
    // defined in the system environment
    static ref RE_VARMENTION_LOC: Regex = Regex::new(r"[%]\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
    static ref RE_VARMENTION_ENV: Regex = Regex::new(r"[\$]\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();

    // these must have a star corresponding to the internal group of the
    // corresponding RE_VARMENTION_* instance
    static ref FMT_VARMENTION_LOC: String = String::from("%{*}");
    static ref FMT_VARMENTION_ENV: String = String::from("${*}");
}



// helper to convert a list of regexp patterns into a single ORed regexp
fn _combine_regexp_patterns(res: &Vec<String>) -> String {
    format!("({})", String::from(res.join("|")))
}



// Helper to calculate hash for a single file
// see https://stackoverflow.com/a/71606608/5138770
fn sha256_digest(path: &PathBuf) -> std::io::Result<String> {
    let input = File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 4096];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 { break }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    Ok(HEXLOWER.encode(digest.as_ref()))
}



// Helpers to simply convert an error code to text
fn format_err_parsable(code: u64) -> String {
    if ERRS_PARSABLE.contains_key(&code) {
        String::from(ERRS_PARSABLE[&code])
    } else { String::from(ERRS_PARSABLE[&ERR_GENERIC]) }
}

fn format_err_verbose(code: u64) -> String {
    if ERRS_VERBOSE.contains_key(&code) {
        String::from(ERRS_VERBOSE[&code])
    } else { String::from(ERRS_VERBOSE[&ERR_GENERIC]) }
}



// helper to format a parsable output line consistently
fn format_parsable_output(
    context: &'static str,
    name: &String,
    code: u64,
    operation: &'static str,
    arg1: &String,
    arg2: &String,
) -> String {
    let mname: String;
    let mtype: String;
    let mresult: String;
    let marg1: String;
    let marg2: String;
    if code == 0 {
        mtype = String::from("INFO");
        mresult = String::from("OK");
    } else {
        mtype = String::from("ERROR");
        mresult = format_err_parsable(code);
    }

    if name.is_empty() {
        mname = String::from("<N/A>");
    } else {
        mname = String::from(name);
    }

    if arg1.is_empty() {
        marg1 = String::from("<N/A>");
    } else {
        marg1 = String::from(arg1);
    }

    if arg2.is_empty() {
        marg2 = String::from("<N/A>");
    } else {
        marg2 = String::from(arg2);
    }

    format!("{context}|{mtype}:{code}/{mresult}|{operation}:{mname}|{marg1}|{marg2}")
}


/// Extract the configuration from a TOML file, given the file name and the
/// pertaining arguments as resulting from the command line. A description of
/// the arguments follows:
///
///     config_file: the path to the configuration file (CLI argument)
///     verbose: turn verbosity on (goes into config), negation of CLI argument 'quiet'
///     parsable_output: produce machine readable output, CLI argument 'parsable-output'
///
/// Returns a tuple consisting in a global configuration and a list of job
/// configurations if successful, otherwise an error containing a string that
/// briefly describes the error and possibly where it occurred.
///
/// The following lines also include some utilities for string replacement
/// in the handled paths.
///

fn _ec_error_invalid_config(key: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput,
        String::from(format!("{}:{}", format_err_parsable(ERR_INVALID_CONFIG_FILE), key)).as_str())
}

fn _ec_replace_variables_in_string(pattern: &Regex, format: &String, source: &String, vars: &HashMap<String, String>) -> String {
    let mut result = String::from(source);

    // mimick shell by replacing undefined variables with the empty string:
    // since the same function is used for both local and environment vars,
    // this represents a difference with the Python version, that considered
    // mentioning an undefined local variable a fatal error
    // WARNING: this actually assumes that the regular expression pattern
    //          "[%$]\{[a-zA-Z_][a-zA-Z0-9_]*\}" cannot appear in the source or
    //          the destination directory within job definitions

    while let Some(caps) = pattern.captures(&result.as_str()) {
        let varname = caps.get(1).map_or("", |m| m.as_str());
        let occurrence = format.replace("*", &varname);
        if let Some(replacement) = vars.get(varname) {
            result = result.replace(&occurrence, replacement);
        } else {
            result = result.replace(&occurrence, &String::new());
        }
    }

    String::from(result)
}

fn _ec_replace_markers_in_string(source: &String, user_home: &PathBuf, config_file_dir: &PathBuf) -> String {
    let mut result = String::from(source);

    for (mkey, mlist) in DIR_MARKERS.clone().iter() {
        for marker in mlist {
            if result.starts_with(*marker) {
                match *mkey {
                    "USER-HOME" => {
                        result = String::from(&user_home
                            .clone()
                            .to_string_lossy()
                            .to_string()) + &String::from(&result[1..]); // to preserve the slash
                    }
                    "CONFIG-FILE-DIR" => {
                        result = String::from(&config_file_dir
                            .clone()
                            .to_string_lossy()
                            .to_string()) + &String::from(&result[1..]); // to preserve the slash
                    }
                    _ => { }
                }
            }
        }
    }

    String::from(result)
}

fn _ec_normalize_path_slashes(path: &String) -> String {
    if cfg!(windows) {
        Regex::new("\\[\\]+")
            .unwrap()   // cannot panic for we know the RE is correct
            .replace_all(&path.replace("/", "\\"), "\\")
            .to_string()
    } else {
        Regex::new("/[/]+")
            .unwrap()   // cannot panic for we know the RE is correct
            .replace_all(&path.replace("\\", "/"), "/")
            .to_string()
    }
}

fn _ec_add_trailing_slashes(path: &String) -> String {
    if cfg!(windows) {
        if path.ends_with("\\") || path.ends_with("/") {
            String::from(path)
        } else {
            String::from(path.to_owned() + "\\")
        }
    } else {
        if path.ends_with("/") {
            String::from(path)
        } else {
            String::from(path.to_owned() + "/")
        }
    }
}

// actual function
fn extract_config(config_file: &PathBuf, verbose: bool, parsable_output: bool) -> std::io::Result<(CopyJobGlobalConfig, Vec<CopyJobConfig>)> {
    // here we also set default values
    let mut global_config = CopyJobGlobalConfig {
        active_jobs: Vec::new(),
        job_list: Vec::new(),
        variables: HashMap::new(),
        recursive: false,
        case_sensitive: true,
        follow_symlinks: true,
        overwrite: true,
        skip_newer: true,
        check_content: false,
        remove_others_matching: false,
        create_directories: true,
        keep_structure: true,
        halt_on_errors: false,

        // the following parameters are defined through CLI arguments only
        config_file: PathBuf::from(
            _ec_normalize_path_slashes(
                &String::from(
                    config_file
                        .as_os_str()
                        .to_str()
                        .unwrap()))),
        verbose: verbose,
        parsable_output: parsable_output,
    };
    let mut job_configs: Vec<CopyJobConfig> = Vec::new();
    let mut check_active_jobs: Vec<String> = Vec::new();
    let allowed_globals: Vec<String> = vec!(
        String::from("active_jobs"),
        String::from("variables"),
        String::from("recursive"),
        String::from("case_sensitive"),
        String::from("follow_symlinks"),
        String::from("overwrite"),
        String::from("skip_newer"),
        String::from("check_content"),
        String::from("remove_others_matching"),
        String::from("create_directories"),
        String::from("keep_structure"),
        String::from("halt_on_errors"),
        String::from("job"),
    );

    let config_map: CfgMap;     // to be initialized below

    match toml::from_str(&fs::read_to_string(config_file)?.as_str()) {
        Ok(toml_text) => {
            config_map = CfgMap::from_toml(toml_text);
        }
        _ => {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format_err_parsable(ERR_INVALID_CONFIG_FILE)));
        }
    }

    // check that global keys are all known: if not report offending key
    for key in config_map.keys() {
        if !allowed_globals.contains(key) {
            return Err(_ec_error_invalid_config(key));
        }
    }

    // strings that will be used to build actal paths
    let var_user_home = PathBuf::from(home_dir().unwrap());
    let var_config_file_dir = PathBuf::from(config_file.clone().parent().unwrap());

    let mut sys_variables: HashMap<String, String> = HashMap::new();
    for (var, value) in env::vars_os() {
        sys_variables.insert(
            String::from(var.to_str().unwrap()),
            String::from(value.to_str().unwrap()),
        );
    }

    // collect globals:

    // 1. list of active jobs (will be checked later)
    let cur_key = "active_jobs";
    let cur_item = config_map.get(&cur_key);
    if !cur_item.check_that(IsList) {
        return Err(_ec_error_invalid_config(&cur_key));
    } else {
        match cur_item {
            Some(c) => {
                for item in c.as_list().unwrap() {
                    if !item.is_str() {
                        return Err(_ec_error_invalid_config(&cur_key));
                    }
                    global_config.active_jobs.push(String::from(item.as_str().unwrap()));
                    check_active_jobs.push(String::from(item.as_str().unwrap()));
                }
            }
            None => {
                return Err(_ec_error_invalid_config(&cur_key));
            }
        }
    }

    // 2. HashMap of local variables
    let cur_key = "variables";
    if config_map.contains_key(cur_key) {
        let cur_item = config_map.get(&cur_key);
        if !cur_item.check_that(IsMap) {
            return Err(_ec_error_invalid_config(&cur_key));
        } else {
            match cur_item {
                Some(c) => {
                    for (key, item) in c.as_map().unwrap().iter() {
                        if !item.is_str() {
                            return Err(_ec_error_invalid_config(&cur_key));
                        }
                        global_config.variables.insert(
                            String::from(key.as_str()),
                            String::from(item.as_str().unwrap()));
                    }
                }
                None => { /* OK to go, default already set */ }
            }
        }
    }

    // 3. recursive flag
    let cur_key = "recursive";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.recursive = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 4. case sensitivity
    let cur_key = "case_sensitive";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.case_sensitive = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 5. whether or not to follow symlinks
    let cur_key = "follow_symlinks";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.follow_symlinks = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 6. whether or not to overwrite destination
    let cur_key = "overwrite";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.overwrite = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 7. newer version skipping
    let cur_key = "skip_newer";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.skip_newer = *item.as_bool().unwrap();

        }
        None => { /* OK to go, default already set */ }
    }

    // 8. whether to check if source == destination
    let cur_key = "check_content";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.check_content = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 9. remove matching destination files
    let cur_key = "remove_others_matching";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.remove_others_matching = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 10. create directory structure if not found
    let cur_key = "create_directories";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.create_directories = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 11. keep source directory structure or flat
    let cur_key = "keep_structure";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.keep_structure = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // 12. halt on errors or continue
    let cur_key = "halt_on_errors";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(item) => {
            if !item.is_bool() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            global_config.halt_on_errors = *item.as_bool().unwrap();
        }
        None => { /* OK to go, default already set */ }
    }

    // collect job definitions
    // note that specific job flags are directly taken from the corresponding
    // global configuration values, so filling will not be needed later; jobs
    // with no name will be considered invalid, and patterns will be recorded
    // in their combined version (in fact allowing multiple patterns is only
    // a way to facilitate writing the configuration file)
    let cur_key = "job";
    let cur_item = config_map.get(&cur_key);
    match cur_item {
        Some(c) => {
            if !c.is_list() {
                return Err(_ec_error_invalid_config(&cur_key));
            }
            for elem in c.as_list().unwrap_or(&Vec::<CfgValue>::new()).into_iter() {
                if !elem.is_map() {
                    return Err(_ec_error_invalid_config(&cur_key));
                } else {
                    let mut job = CopyJobConfig {
                        job_name: String::new(),
                        source_dir: PathBuf::new(),
                        destination_dir: PathBuf::new(),
                        include_pattern: String::new(),
                        exclude_pattern: String::from(STR_MATCH_NO_FILE.as_str()),
                        excludedir_pattern: String::from(STR_MATCH_NO_FILE.as_str()),
                        recursive: global_config.recursive,
                        case_sensitive: global_config.case_sensitive,
                        follow_symlinks: global_config.follow_symlinks,
                        overwrite: global_config.overwrite,
                        skip_newer: global_config.skip_newer,
                        check_content: global_config.check_content,
                        remove_others_matching: global_config.remove_others_matching,
                        create_directories: global_config.create_directories,
                        keep_structure: global_config.keep_structure,
                        halt_on_errors: global_config.halt_on_errors,
                    };
                    for (key, item) in elem.as_map().unwrap().iter() {
                        // a note on variable and marker replacements: first we
                        // replace local variables, because they could mention
                        // environment variables and/or special markers to be
                        // replaced below, then the environment variables, as
                        // special markers (especially '~/') could be present
                        // therein; at last we replace the special markers
                        match key.as_str() {
                            "name" => {
                                let cur_key = "job/name";
                                if !item.is_str() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.job_name = String::from(item.as_str().unwrap());
                                if !RE_JOBNAME.is_match(&job.job_name) {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                            }
                            "source" => {
                                let cur_key = "job/source";
                                if !item.is_str() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                let mut s = String::from(item.as_str().unwrap());
                                s = _ec_replace_variables_in_string(
                                    &RE_VARMENTION_LOC,
                                    &FMT_VARMENTION_LOC,
                                    &String::from(s),
                                    &global_config.variables,
                                );
                                s = _ec_replace_variables_in_string(
                                    &RE_VARMENTION_ENV,
                                    &FMT_VARMENTION_ENV,
                                    &String::from(s),
                                    &sys_variables,
                                );
                                s = _ec_replace_markers_in_string(
                                    &s,
                                    &var_user_home,
                                    &var_config_file_dir
                                );
                                job.source_dir = PathBuf::from(
                                    _ec_add_trailing_slashes(&_ec_normalize_path_slashes(&s)));
                            }
                            "destination" => {
                                let cur_key = "job/destination";
                                if !item.is_str() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                let mut s = String::from(item.as_str().unwrap());
                                s = _ec_replace_variables_in_string(
                                    &RE_VARMENTION_LOC,
                                    &FMT_VARMENTION_LOC,
                                    &String::from(s),
                                    &global_config.variables,
                                );
                                s = _ec_replace_variables_in_string(
                                    &RE_VARMENTION_ENV,
                                    &FMT_VARMENTION_ENV,
                                    &String::from(s),
                                    &sys_variables,
                                );
                                s = _ec_replace_markers_in_string(
                                    &s,
                                    &var_user_home,
                                    &var_config_file_dir
                                );
                                job.destination_dir = PathBuf::from(
                                    _ec_add_trailing_slashes(&_ec_normalize_path_slashes(&s)));
                            }
                            "patterns_include" => {
                                let cur_key = "job/patterns_include";
                                if !item.is_list() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                let mut li: Vec<String> = Vec::new();
                                for i in item.as_list().unwrap() {
                                    if let Some(s) = i.as_str() {
                                        if s.len() > 0 { li.push(String::from(s)); }
                                    }
                                }
                                job.include_pattern = _combine_regexp_patterns(&li);
                            }
                            "patterns_exclude" => {
                                let cur_key = "job/patterns_exclude";
                                if !item.is_list() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                let mut li: Vec<String> = Vec::new();
                                for i in item.as_list().unwrap() {
                                    if let Some(s) = i.as_str() {
                                        if s.len() > 0 { li.push(String::from(s)); }
                                    }
                                }
                                job.exclude_pattern = _combine_regexp_patterns(&li);
                            }
                            "patterns_exclude_dir" => {
                                let cur_key = "job/patterns_exclude_dir";
                                if !item.is_list() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                let mut li: Vec<String> = Vec::new();
                                for i in item.as_list().unwrap() {
                                    if let Some(s) = i.as_str() {
                                        if s.len() > 0 { li.push(String::from(s)); }
                                    }
                                }
                                job.excludedir_pattern = _combine_regexp_patterns(&li);
                            }
                            "recursive" => {
                                let cur_key = "job/recursive";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.recursive = *item.as_bool().unwrap();
                            }
                            "case_sensitive" => {
                                let cur_key = "job/case_sensitive";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.case_sensitive = *item.as_bool().unwrap();
                            }
                            "follow_symlinks" => {
                                let cur_key = "job/follow_symlinks";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.follow_symlinks = *item.as_bool().unwrap();
                            }
                            "overwrite" => {
                                let cur_key = "job/overwrite";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.overwrite = *item.as_bool().unwrap();
                            }
                            "skip_newer" => {
                                let cur_key = "job/skip_newer";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.skip_newer = *item.as_bool().unwrap();
                            }
                            "check_content" => {
                                let cur_key = "job/check_content";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.check_content = *item.as_bool().unwrap();
                            }
                            "remove_others_matching" => {
                                let cur_key = "job/remove_others_matching";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.remove_others_matching = *item.as_bool().unwrap();
                            }
                            "create_directories" => {
                                let cur_key = "job/create_directories";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.create_directories = *item.as_bool().unwrap();
                            }
                            "keep_structure" => {
                                let cur_key = "job/keep_structure";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.keep_structure = *item.as_bool().unwrap();
                            }
                            "halt_on_errors" => {
                                let cur_key = "job/halt_on_errors";
                                if !item.is_bool() {
                                    return Err(_ec_error_invalid_config(&cur_key));
                                }
                                job.halt_on_errors = *item.as_bool().unwrap();
                            }
                            _ => {
                                return Err(_ec_error_invalid_config(&cur_key));
                            }
                        }
                    }
                    if job.job_name.len() == 0 {
                        return Err(_ec_error_invalid_config(&cur_key));
                    }
                    global_config.job_list.push(String::from(&job.job_name));
                    job_configs.push(job);
                }
            }
        }
        None => { /* job_configs remains empty */ }
    }

    // check that all active jobs that have been listed are actually defined
    let cur_key = "active_jobs";
    for item in Vec::from(check_active_jobs) {
        if !global_config.job_list.contains(&item) {
            return Err(_ec_error_invalid_config(&cur_key));
        }
    }

    // PHEW! Now the configuration is complete (unless this function panicked)
    Ok((global_config, job_configs))
}



/// Build a list of files in a directory matching/unmatching a pattern by
/// either listing the files in that directory or traversing it recursively.
/// A description of the accepted parameters follows:
///
///     search_dir: the full specification of search directory
///     include_pattern: file/dir names to be processed (regular expression)
///     exclude_pattern: file/dir names to be excluded (regular expression)
///     recursive: recursively traverse the directory structure
///     follow_symlinks: follow symbolic links
///     case_sensitive: consider provided patterns as case sensitive
///
/// this utility can be used both for determining which files to copy from
/// the source directory and what files to delete in the destination folder
/// if requested
///
/// NOTE: skip errors code, see: https://github.com/BurntSushi/walkdir/blob/master/README.md
///

fn list_files_matching(
    search_dir: &PathBuf,
    include_pattern: &String,
    exclude_pattern: &String,
    excludedir_pattern: &String,
    recursive: bool,
    follow_symlinks: bool,
    case_sensitive: bool,
) -> Option<Vec<PathBuf>> {
    // FIXME: for now erratic patterns only cause a no-match (acceptable?)
    //        in the release version they should actually return None
    let include_match = RegexBuilder::new(
        String::from(format!("^{include_pattern}$")).as_str())
            .case_insensitive(!case_sensitive)
            .build()
            .unwrap_or(RE_MATCH_NO_FILE.clone());
    let exclude_match = RegexBuilder::new(
        String::from(format!("^{exclude_pattern}$")).as_str())
            .case_insensitive(!case_sensitive)
            .build()
            .unwrap_or(RE_MATCH_NO_FILE.clone());
    let excludedir_match = RegexBuilder::new(
        String::from(format!("{excludedir_pattern}")).as_str())
            .case_insensitive(!case_sensitive)
            .build()
            .unwrap_or(RE_MATCH_NO_FILE.clone());

    let depth: usize = if recursive { usize::MAX } else { 1 };
    let mut result: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(&search_dir)
        .max_depth(depth)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok()) {    // skip errors
            if !entry.file_type().is_dir() && !(follow_symlinks && entry.file_type().is_symlink()) {
                let mut dir_pathbuf = PathBuf::from(entry.path());
                let dir_name = if dir_pathbuf.pop() {
                    dir_pathbuf.to_str().unwrap_or("*").replace(&search_dir.to_str().unwrap(), "")
                } else { String::from("*") };   // a value of '*' as directory reports an error (reserved on both Unix&Win)
                if dir_name != "*" && !excludedir_match.is_match(&dir_name.as_str()) {    // intentionally no ^$ wrapping
                    if let Some(file_name) = entry.path().file_name() {
                        if include_match.is_match(&file_name.to_str().unwrap_or(""))
                           && !exclude_match.is_match(&file_name.to_str().unwrap_or("")) {
                            result.push(PathBuf::from(entry.path()));
                        }
                    }
                }
            }
        }
    Some(result)
}



/// Attempt to copy a single file to a destination (provided as a path):
/// source and destination are full or relative to current FS position,
/// they must be both complete, in particular destination must include the
/// file name component. The implementation itself uses PathBuf based
/// variables for file name handling; also, the helper takes care to create
/// the destination path if instructed to, and to compare modification time
/// and contents of the destination file if it exists. A full description
/// of the required parameters follows:
///
///     source: the full specification of source file
///     destination: the full specification of destination file
///     overwrite: if false, never overwrite an existing destination
///     skip_newer: if overwrite, only overwrite when source is newer
///     check_content: if overwrite, only overwrite when contents differ
///     follow_symlinks: follow symbolic links
///     create_directories: create directory if it does not exist yet
///

fn copyfile (
    source: &PathBuf,
    destination: &PathBuf,
    overwrite: bool,
    skip_newer: bool,
    check_content: bool,
    follow_symlinks: bool,
    create_directories: bool,
) -> FileOpOutcome {
    // normalize paths
    let source_path = PathBuf::from(
        &source.canonicalize().unwrap_or(PathBuf::new()));
    let destination_path = PathBuf::from(
        &destination.canonicalize().unwrap_or(PathBuf::from(&destination)));
        // NOTE: https://doc.rust-lang.org/nightly/std/fs/fn.canonicalize.html#errors
        //       `canonicalize` returns an error if the target does not exist, thus
        //       we either get the canonicalied path or the original path.
        // TODO: consider assuming paths as already canonicalized by the caller

    // check source and destination metadata (and whether or not they exist)
    match metadata(&source_path) {
        Ok(s_stat) => {
            // first check that source <> destination
            if source_path == destination_path {
                return FileOpOutcome::Error(FOERR_DESTINATION_IS_ITSELF);
            }
            if s_stat.is_dir() {
                return FileOpOutcome::Error(FOERR_SOURCE_IS_DIR);
            }
            if s_stat.is_symlink() && !follow_symlinks {   // TODO: expected?
                return FileOpOutcome::Error(FOERR_SOURCE_IS_SYMLINK);
            }
            match metadata(&destination_path) {
                Ok(d_stat) => {
                    // if we are here, then the destination exists: check
                    // whether overwrite is false, compare s_stat, d_stat and
                    // possibly hashes
                    if !overwrite {
                        return FileOpOutcome::Error(FOERR_DESTINATION_EXISTS);
                    } else if d_stat.is_dir() {
                        return FileOpOutcome::Error(FOERR_DESTINATION_IS_DIR);
                    } else if d_stat.is_symlink() && !follow_symlinks {
                        return FileOpOutcome::Error(FOERR_DESTINATION_IS_SYMLINK);
                    }
                    if skip_newer {
                        match s_stat.modified() {
                            Ok(s_mtime) => {
                                match d_stat.modified() {
                                    Ok(d_mtime) => {
                                        if s_mtime <= d_mtime {
                                            return FileOpOutcome::Error(FOERR_DESTINATION_IS_NEWER);
                                        }
                                    }
                                    Err(_) => {
                                        return FileOpOutcome::Error(FOERR_DESTINATION_NOT_ACCESSIBLE);
                                    }
                                }
                            }
                            // should never be reached
                            Err(_) => {
                                return FileOpOutcome::Error(FOERR_SOURCE_NOT_ACCESSIBLE);
                            }
                        }
                    }
                    // only when asked perform content checking via SHA256
                    // and skip copy if the contents are the same
                    if check_content {
                        match sha256_digest(&source_path) {
                            Ok(source_hash) => {
                                match sha256_digest(&destination_path) {
                                    Ok(destination_hash) => {
                                        if destination_hash == source_hash {
                                            return FileOpOutcome::Error(
                                                FOERR_DESTINATION_IS_IDENTICAL);
                                        }
                                    }
                                    Err(_) => {
                                        return FileOpOutcome::Error(
                                            FOERR_DESTINATION_NOT_ACCESSIBLE);
                                    }
                                }
                            }
                            Err(_) => {
                                return FileOpOutcome::Error(FOERR_SOURCE_NOT_ACCESSIBLE);
                            }
                        }
                    }
                }
                Err(_) => {
                    // in case of error check whether or not the destination
                    // directory exists, and if not create it when instructed
                    // to do so
                    // TODO: double check this part!!! If destdir is found a CANNOT_CREATE_DIR error is propagated
                    let mut destination_dir = PathBuf::from(&destination_path);
                    if !destination_dir.pop() {
                        return FileOpOutcome::Error(FOERR_CANNOT_CREATE_DIR);
                    }
                    match metadata(&destination_dir) {
                        Ok(d_dirdata) => {
                            if !d_dirdata.is_dir() {
                                return FileOpOutcome::Error(FOERR_CANNOT_CREATE_DIR);
                            }
                        }
                        Err(_) => {
                            if !create_directories {
                                return FileOpOutcome::Error(FOERR_CANNOT_CREATE_DIR);
                            }
                            match create_dir_all(&destination_dir) {
                                Ok(_) => { /* safe to go */ }
                                Err(_) => {
                                    return FileOpOutcome::Error(FOERR_CANNOT_CREATE_DIR);
                                }
                            }
                        }
                    }
                }
            }
            // actually copy the file using OS API
            let res = fs::copy(source_path, destination_path);
            match res {
                Ok(_) => {
                    // FileOpOutcome::Success is returned only here, after an
                    // actually successful copy operation
                    return FileOpOutcome::Success
                }
                Err(res_err) => {
                    if res_err.kind() == std::io::ErrorKind::PermissionDenied {
                        return FileOpOutcome::Error(FOERR_DESTINATION_IS_READONLY);
                    }
                    return FileOpOutcome::Error(FOERR_GENERIC_FAILURE);
                }
            }
        }
        Err(_) => {
            return FileOpOutcome::Error(FOERR_SOURCE_NOT_ACCESSIBLE);
        }
    }
}



/// Attempt to remove a specified file if it exists and if allowed to. A
/// full description of the required parameters follows:
///
///     destination: the full specification of destination file
///     follow_symlinks: follow symbolic links
///

fn removefile (
    destination: &PathBuf,
    follow_symlinks: bool,
) -> FileOpOutcome {
    // normalize paths
    let destination_path = PathBuf::from(
        &destination.canonicalize().unwrap_or(PathBuf::new()));

    match metadata(&destination_path) {
        Ok(d_stat) => {
            // if we are here, then destination exists
            if d_stat.is_dir() {
                return FileOpOutcome::Error(FOERR_DESTINATION_IS_DIR);
            } else if d_stat.is_symlink() && !follow_symlinks {
                return FileOpOutcome::Error(FOERR_DESTINATION_IS_SYMLINK);
            }
            match fs::remove_file(destination_path) {
                Ok(_) => {
                    return FileOpOutcome::Success;
                }
                Err(_) => {
                    return FileOpOutcome::Error(FOERR_DESTINATION_NOT_ACCESSIBLE);
                }
            }
        }
        Err(_) => {
            return FileOpOutcome::Error(FOERR_DESTINATION_NOT_ACCESSIBLE);
        }
    }
}



/// Perform a single copy job, by building a list of files to copy and by
/// copying them if possible using `copyfile` seen above. To be noticed that:
///
///     1) the job name has to be provided for reporting/logging purposes
///     2) source_dir and destination_dir need to be passed to this function
///        *after* replacing shortcuts and variables with their values, as no
///        substitution is performed here
///
/// A description of the parameters follows:
///
///     job: &CopyJobConfig, containing all the job parameters
///     verbose: bool, provide output while running the job
///     parsable_output: bool, provide machine readable output if verbose
///
/// NOTE: writes to stdout/stderr
/// NOTE: machine readable prefix of this section is JOB
///
/// The following lines also include some simple formatters to ease up writing
/// suitable messages when needed.
///

fn _rsj_format_copy_feedback(parsable_output: bool, job: &String, source: &PathBuf, destination: &PathBuf) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, 0, "COPY",
            &String::from(source.to_str().unwrap_or("<unknown>")),
            &String::from(destination.to_str().unwrap_or("<unknown>")))
    } else {
        format!("copied in job {}: {} => {}", job, source.display(), destination.display())
    }
}

fn _rsj_format_copy_error(parsable_output: bool, job: &String, code: u64, source: &PathBuf, destination: &PathBuf) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, code, "COPY",
            &String::from(source.to_str().unwrap_or("<unknown>")),
            &String::from(destination.to_str().unwrap_or("<unknown>")))
    } else {
        format!("error in job {}: '{}' while copying {} => {}",
            job, format_err_verbose(code), source.display(), destination.display())
    }
}

fn _rsj_format_del_feedback(parsable_output: bool, job: &String, destination: &PathBuf) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, 0, "DEL",
            &String::new(),
            &String::from(destination.to_str().unwrap_or("<unknown>")),
        )
    } else {
        format!("removed in job {}: {}", job, destination.display())
    }
}

fn _rsj_format_del_error(parsable_output: bool, job: &String, code: u64, destination: &PathBuf) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, code, "DEL",
            &String::new(),
            &String::from(destination.to_str().unwrap_or("<unknown>")),
        )
    } else {
        format!("error in job {}: '{}' while removing {}",
            job, format_err_verbose(code), destination.display())
    }
}

fn _rsj_format_job_info_begin(parsable_output: bool, job: &String, to_copy: usize, to_delete: usize) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, 0, "BEG_CPDL",
            &String::from(format!("{to_copy}")),
            &String::from(format!("{to_delete}")),
        )
    } else {
        format!("tasks in job {}: {} file(s) to copy, {} to possibly remove on destination",
            job, to_copy, to_delete)
    }
}

fn _rsj_format_job_info_end(parsable_output: bool, job: &String, copied: usize, deleted: usize) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, 0, "END_CPDL",
            &String::from(format!("{copied}")),
            &String::from(format!("{deleted}")),
        )
    } else {
        format!("results for job {}: {} file(s) copied, {} removed on destination", job, copied, deleted)
    }
}

fn _rsj_format_job_error(parsable_output: bool, job: &String, code: u64) -> String {
    if parsable_output {
        format_parsable_output("JOB", job, code, "END_CPDL",
            &String::new(),
            &String::new(),
        )
    } else {
        format!("error in job {}: '{}'", job, format_err_verbose(code))
    }
}

// actual function
fn run_single_job(
    job: &CopyJobConfig,
    verbose: bool,
    parsable_output: bool,
) -> CopyJobOutcome {
    // source and destination must exist and be canonicalizeable
    let source_directory = PathBuf::from(
        &job.source_dir.canonicalize().unwrap_or(PathBuf::new()));
    if !source_directory.exists() {
        if verbose {
            eprintln!("{}", _rsj_format_job_error(
                parsable_output, &job.job_name, CJERR_DESTINATION_DIR_NOT_EXISTS));
        }
        return CopyJobOutcome::Error(CJERR_SOURCE_DIR_NOT_EXISTS);
    }
    if !job.destination_dir.exists() {
        if !job.create_directories {
            if verbose {
                eprintln!("{}", _rsj_format_job_error(
                    parsable_output, &job.job_name, CJERR_DESTINATION_DIR_NOT_EXISTS));
            }
            return CopyJobOutcome::Error(CJERR_DESTINATION_DIR_NOT_EXISTS);
        }
    }

    // build the list of files to be copied
    match list_files_matching(
        // use job.source_dir here to retrieve names that are not canonicalized
        // and thus usable for string operations as described in the config
        &job.source_dir,
        &job.include_pattern,
        &job.exclude_pattern,
        &job.excludedir_pattern,
        job.recursive,
        job.follow_symlinks,
        job.case_sensitive) {
            Some(files_to_copy) => {
                let mut num_files_copied: usize = 0;
                let mut num_files_deleted: usize = 0;
                let mut files_to_delete = if job.remove_others_matching {
                    list_files_matching(
                        &job.destination_dir,
                        &job.include_pattern,
                        &job.exclude_pattern,
                        &job.excludedir_pattern,
                        job.recursive,
                        job.follow_symlinks,
                        job.case_sensitive).unwrap_or(Vec::new())
                } else { Vec::new() };
                if verbose {
                    println!("{}", _rsj_format_job_info_begin(
                        parsable_output,
                        &job.job_name,
                        files_to_copy.len(),
                        files_to_delete.len(),
                    ));
                }
                for item in files_to_copy {
                    // here we also copy the file
                    let destination = PathBuf::from(&job.destination_dir);
                    let destfile_relative: PathBuf = if job.keep_structure {
                        PathBuf::from(&item)
                            .strip_prefix(&job.source_dir)
                            .unwrap_or(&PathBuf::from("")).to_path_buf()
                        } else {
                            PathBuf::from(&item.file_name().unwrap_or(
                                &std::ffi::OsStr::new(""))).to_path_buf()
                        };
                        if destfile_relative.as_os_str().len() > 0 {
                            let destfile_absolute = destination.join(destfile_relative);
                            // now that the destination path is known, check
                            // whether the list of matching files to delete
                            // contains it and remove it from the list: in
                            // this way the deletion process is selective and
                            // only deletes unwanted files in the target
                            // directory
                            if files_to_delete.contains(&destfile_absolute) {
                                files_to_delete.remove(
                                    files_to_delete.iter()
                                        .position(|x| x.as_path() == destfile_absolute.as_path())
                                        .unwrap());     // cannot panic here
                            }
                            match copyfile(
                                &item,
                                &destfile_absolute,
                                job.overwrite,
                                job.skip_newer,
                                job.check_content,
                                job.follow_symlinks,
                                job.create_directories) {
                                    FileOpOutcome::Success => {
                                        num_files_copied += 1;
                                        if verbose {
                                            println!("{}", _rsj_format_copy_feedback(
                                                parsable_output,
                                                &job.job_name,
                                                &item,
                                                &destfile_absolute,
                                            ));
                                        }
                                    }
                                    FileOpOutcome::Error(err) => {
                                        if verbose {
                                            eprintln!("{}", _rsj_format_copy_error(
                                                parsable_output,
                                                &job.job_name,
                                                err,
                                                &item,
                                                &destfile_absolute,
                                            ));
                                        }
                                        if job.halt_on_errors {
                                            return CopyJobOutcome::Error(CJERR_GENERIC_FAILURE);
                                        };
                                    }
                                };
                        } else {
                            if verbose {
                                eprintln!("{}", _rsj_format_copy_error(
                                    parsable_output,
                                    &job.job_name,
                                    CJERR_CANNOT_DETERMINE_DESTFILE,
                                    &item,
                                    &destination,
                                ));
                            }
                            if job.halt_on_errors {
                                return CopyJobOutcome::Error(CJERR_CANNOT_DETERMINE_DESTFILE)
                            }
                        }
                }
                // if not remove_other_matching the vector is empty
                for item in files_to_delete {
                    match removefile(&item, job.follow_symlinks) {
                        FileOpOutcome::Success => {
                            if verbose {
                                println!("{}", _rsj_format_del_feedback(
                                    parsable_output, &job.job_name, &item));
                            }
                            num_files_deleted += 1;
                        }
                        FileOpOutcome::Error(err) => {
                            if verbose {
                                eprintln!("{}", _rsj_format_del_error(
                                    parsable_output, &job.job_name, err, &item));
                            }
                            if job.halt_on_errors {
                                return CopyJobOutcome::Error(CJERR_GENERIC_FAILURE);
                            };
                        }
                    }
                }
                if verbose {
                    println!("{}", _rsj_format_job_info_end(
                        parsable_output,
                        &job.job_name,
                        num_files_copied,
                        num_files_deleted,
                    ));
                }
            }
            None => {
                if verbose {
                    eprintln!("{}", _rsj_format_job_error(
                        parsable_output, &job.job_name, CJERR_NO_SOURCE_FILES));
                }
                return CopyJobOutcome::Error(CJERR_NO_SOURCE_FILES);
            }
    }

    CopyJobOutcome::Success
}



/// Perform all jobs, according to the passed global config object and list
/// of job configuration objects, that is the result of extract_config as
/// defined above. A brief description of the arguments follows:
///
///     global_config: &CopyJobGlobalConfig, global configuration
///     job_configs: &Vec<CopyJobConfig>, full list of job configurations
///
/// This function selects the jobs to actually perform according to the
/// list of names provided in global_config.active_jobs, so the full list
/// of jobs found in the configuration file can be provided.
///
/// NOTE: writes to stdout/stderr
/// NOTE: machine readable prefix of this section is TASK
///
/// The following lines also include some simple formatters to ease up writing
/// suitable messages when needed.
///

fn _rj_format_copy_feedback(parsable_output: bool, job: &String) -> String {
    if parsable_output {
        format_parsable_output(
            "TASK", job, 0, "END_JOB",
            &String::new(),
            &String::new())
    } else {
        format!("job {} completed successfully", job)
    }
}

fn _rj_format_copy_error(parsable_output: bool, job: &String, code: u64) -> String {
    if parsable_output {
        format_parsable_output(
            "TASK", job, code, "END_JOB",
            &String::new(),
            &String::new())
    } else {
        format!("job {} failed with error '{}'", job, format_err_verbose(code))
    }
}

fn _rj_format_cfgfile_info(parsable_output: bool, cfgfile: &String) -> String {
    if parsable_output {
        format_parsable_output(
            "TASK", cfgfile, 0, "CONFIG",
            &String::new(),
            &String::new())
    } else {
        format!("info: using configuration file '{}'", cfgfile)
    }
}

// actual function
fn run_jobs(global_config: &CopyJobGlobalConfig, job_configs: &Vec<CopyJobConfig>) -> std::io::Result<()> {
    if global_config.verbose {
        println!("{}", _rj_format_cfgfile_info(
            global_config.parsable_output,
            &global_config.config_file.to_string_lossy().to_string()));
    }
    for job in job_configs {
        if global_config.active_jobs.contains(&job.job_name) {
            match run_single_job(job, global_config.verbose, global_config.parsable_output) {
                CopyJobOutcome::Success => {
                    if global_config.verbose {
                        println!("{}", _rj_format_copy_feedback(
                            global_config.parsable_output, &job.job_name));
                    }
                }
                CopyJobOutcome::Error(code) => {
                    if global_config.verbose {
                        println!("{}", _rj_format_copy_error(
                            global_config.parsable_output, &job.job_name, code));
                    }
                    if global_config.halt_on_errors {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted, format_err_parsable(ERR_GENERIC)));
                    }
                }
            }
        }
    }

    Ok(())
}


use clap::Parser;

// argument parsing and command execution: doc comments are used by clap

/// Perform complex copy jobs according to criteria provided in a TOML file
#[derive(Parser)]
#[command(name="copyjob", version, about)]
struct Args {
    /// Suppress all output
    #[arg(short, long)]
    quiet: bool,

    /// Generate machine readable output
    #[arg(short = 'p', long = "parsable-output")]
    parsable_output: bool,

    /// path to configuration file
    #[arg()]
    config: String,
}


// helpers to write a message from the main entry point
fn _main_format_error(parsable_output: bool, e: std::io::Error, msg_parsable: &String, msg_verbose: &String) -> String {
    if parsable_output {
        let code = u64::try_from(
            e.raw_os_error().unwrap_or(9999)).unwrap_or(9999);
        format_parsable_output(
            "MAIN", &String::new(), code, "END_MAIN",
            msg_parsable,
            &String::from(e.to_string()))
    } else {
        format!("error: {} / {}", msg_verbose, e.to_string())
    }
}

fn _main_format_output(parsable_output: bool, msg_parsable: &String, msg_verbose: &String) -> String {
    if parsable_output {
        format_parsable_output(
            "MAIN", &String::new(), ERR_OK, "END_MAIN",
            msg_parsable,
            &String::new())
    } else {
        format!("info: {} ", msg_verbose)
    }
}


// entry point: mandatory arguments are handled by the parser
fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // configuration file name is canonicalized in order to get a correct
    // UNICODE path that includes the prefix, so that substitutions in
    // destination file names can be performed without error; an empty
    // PathBuf is produced if the file path does not exist, and this will
    // cause an error while reading the configuration
    let config = extract_config(
        &PathBuf::from(args.config).canonicalize().unwrap_or(PathBuf::new()),
        !args.quiet,
        args.parsable_output);

    match config {
        Ok((global, jobs)) => {
            match run_jobs(&global, &jobs) {
                Ok(_) => {
                    if !args.quiet {
                        println!("{}", _main_format_output(
                            args.parsable_output,
                            &format_err_parsable(ERR_OK),
                            &format_err_verbose(ERR_OK)));
                        }
                    Ok(())
                }
                Err(e) => {
                    if !args.quiet {
                        eprintln!("{}", _main_format_error(
                            args.parsable_output,
                            e,
                            &format_err_parsable(ERR_GENERIC),
                            &format_err_verbose(ERR_GENERIC)));
                        }
                    std::process::exit(2);
                }
            }
        }
        Err(e) => {
            if !args.quiet {
                eprintln!("{}", _main_format_error(
                    args.parsable_output,
                    e,
                    &format_err_parsable(ERR_INVALID_CONFIG_FILE),
                    &format_err_verbose(ERR_INVALID_CONFIG_FILE)));
            }
            std::process::exit(2);
        }
    }
}


// end.
