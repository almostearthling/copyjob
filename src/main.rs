//! copyjob
//! An utility to perform complex copy operations based on TOML files
//! (c) 2023-2026, Francesco Garosi

use std::fs;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::metadata;

use std::env;
use std::io::BufReader;
use std::io::Read;

use lazy_static::lazy_static;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::{Regex, RegexBuilder};

use dirs::home_dir;
use walkdir::WalkDir;

use cfgmap::{CfgMap, CfgValue, Checkable, Condition::*};
use data_encoding::HEXLOWER;
use serde_json::json;
use sha2::{Digest, Sha256};

mod constants;
mod utility;

use constants::*;
use utility::cfghelp::*;
use utility::result::*;

// Structures used for a copy job configuration and the global configuration:
// values provided in CopyJobConfig default to the ones provided globally in
// the CopyJobGlobalConfig object, and override them if different
#[derive(Debug)]
struct CopyJobConfig {
    job_name: String,             // the job name
    source_dir: PathBuf,          // source directory
    destination_dir: PathBuf,     // destination directory
    include_pattern: String,      // RE pattern of filenames to include
    exclude_pattern: String,      // RE pattern of filenames to exclude
    excludedir_pattern: String,   // RE pattern of directories to skip
    recursive: bool,              // recurse directories
    case_sensitive: bool,         // consider filenames as case sensitive
    follow_symlinks: bool,        // follow symlinks
    overwrite: bool,              // possibly overwrite destination
    skip_newer: bool,             // do not overwrite more recent files
    check_content: bool,          // check whether contents are the same
    remove_others_matching: bool, // remove matching files not present in source
    create_directories: bool,     // create non-existing directories
    keep_structure: bool,         // keep directory structure as in source
    trash_on_delete: bool,        // use garbage bin instead of deleting
    trash_on_overwrite: bool,     // send to garbage bin before overwrite
    halt_on_errors: bool,         // exit job if an error occurs
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
    trash_on_delete: bool,              // use garbage bin instead of deleting
    trash_on_overwrite: bool,           // send to garbage bin before overwrite
    halt_on_errors: bool,               // exit job if an error occurs

    // the following parameters are defined through CLI arguments only
    config_file: PathBuf,  // configuration file path
    verbose: bool,         // provide output while running
    parsable_output: bool, // provide machine-readable output
}

// Some constants used within the code
lazy_static! {
    // directory markers: any of the values in respective lists, when
    // used in the source and destination directory specs, will expand
    // into the appropriate fully qualified path, namely:
    //  USER-HOME => $HOME / %USERPROFILE%
    //  CONFIG-FILE-DIR => where the current config file is located

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
fn combine_regexp_patterns(res: &[String]) -> String {
    format!("({})", res.join("|"))
}

// Helper to calculate hash for a single file
// see https://stackoverflow.com/a/71606608/5138770
fn sha256_digest(path: &Path) -> std::io::Result<String> {
    let input = File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 4096];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    Ok(HEXLOWER.encode(digest.as_ref()))
}

// helper to format a parsable output line consistently
fn format_output_parsable(
    context: &'static str,
    name: &str,
    code: i64,
    operation: &str,
    arg1: &str,
    arg2: &str,
) -> String {
    let mresult = code_to_str_parsable(code).to_string();
    let mtype = if code == 0 {
        String::from("INFO")
    } else {
        String::from("ERROR")
    };

    let mname = if name.is_empty() {
        String::from("<N/A>")
    } else {
        String::from(name)
    };

    let marg1 = if arg1.is_empty() {
        String::from("<N/A>")
    } else {
        String::from(arg1)
    };

    let marg2 = if arg2.is_empty() {
        String::from("<N/A>")
    } else {
        String::from(arg2)
    };

    // construct a JSON message that reports the context, the type of message,
    // the result both as an integer (see the *ERR_* constants above) and as a
    // short string, the operation (point in the context) being performed when
    // the message is issued, and two (optional) arguments that may or may not
    // contain a value, and whose value depends on the current context and/or
    // the current operation; then return it as a String
    json!({
        "context": context,
        "message_type": mtype,
        "result": [code, mresult],
        "operation": [operation, mname],
        "args": [marg1, marg2]
    })
    .to_string()
}

/// This make variables and markers replacement easier
///
/// Note: it only works with strings, which can be converted to `PathBuf`
trait ReplaceableVarString {
    fn replace_start(&self, vars: &HashMap<&str, &str>) -> Self;
    fn replace_vars(&self, pattern: &Regex, format: &str, vars: &HashMap<&str, &str>) -> Self;
}

impl ReplaceableVarString for String {
    fn replace_start(&self, vars: &HashMap<&str, &str>) -> Self {
        let mut s = String::from(self);
        for (k, v) in vars {
            if s.starts_with(k) {
                s = s.replace(k, v);
                break;
            }
        }
        s
    }

    fn replace_vars(&self, pattern: &Regex, format: &str, vars: &HashMap<&str, &str>) -> Self {
        let mut result = String::from(self);
        // mimick shell by replacing undefined variables with the empty string:
        // since the same function is used for both local and environment vars,
        // this represents a difference with the Python version, that considered
        // mentioning an undefined local variable a fatal error
        // WARNING: this actually assumes that the regular expression pattern
        //          "[%$]\{[a-zA-Z_][a-zA-Z0-9_]*\}" cannot appear in the source or
        //          the destination directory within job definitions
        while let Some(caps) = pattern.captures(result.as_str()) {
            let varname = caps.get(1).map_or("", |m| m.as_str());
            let occurrence = format.replace("*", varname);
            if let Some(replacement) = vars.get(varname) {
                result = result.replace(&occurrence, replacement);
            } else {
                result = result.replace(&occurrence, "");
            }
        }
        result
    }
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
/// As internal functions it also includes utilities for string replacement
/// in the handled paths.
fn extract_config(
    config_file: &PathBuf,
    verbose: bool,
    parsable_output: bool,
) -> Result<(CopyJobGlobalConfig, Vec<CopyJobConfig>)> {
    // local helpers:

    // l1. create a specific error
    fn _ec_error_invalid_config(key: &str) -> Error {
        Error::new(
            Kind::Invalid,
            ERR_CODE_INVALID_CONFIG_FILE,
            format!(
                "{}:{key}",
                code_to_str_parsable(ERR_CODE_INVALID_CONFIG_FILE).to_string()
            )
            .as_str(),
        )
    }

    // l4. normalize path slashes (forward+back & multiple)
    fn _ec_normalize_path_slashes(path: &str) -> String {
        if cfg!(windows) {
            Regex::new("\\[\\]+")
                .unwrap() // cannot panic for we know the RE is correct
                .replace_all(&path.replace("/", "\\"), "\\")
                .to_string()
        } else {
            Regex::new("/[/]+")
                .unwrap() // cannot panic for we know the RE is correct
                .replace_all(&path.replace("\\", "/"), "/")
                .to_string()
        }
    }

    // l5. add trailing slashes
    fn _ec_add_trailing_slashes(path: &str) -> String {
        if cfg!(windows) {
            if path.ends_with("\\") || path.ends_with("/") {
                String::from(path)
            } else {
                path.to_owned() + "\\"
            }
        } else {
            if path.ends_with("/") {
                String::from(path)
            } else {
                path.to_owned() + "/"
            }
        }
    }

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
        trash_on_delete: true,
        trash_on_overwrite: false,
        halt_on_errors: false,

        // the following parameters are defined through CLI arguments only
        config_file: PathBuf::from(_ec_normalize_path_slashes(&String::from(
            config_file.as_os_str().to_str().unwrap(),
        ))),
        verbose,
        parsable_output,
    };
    let mut job_configs: Vec<CopyJobConfig> = Vec::new();
    let allowed_globals = vec![
        "active_jobs",
        "variables",
        "recursive",
        "case_sensitive",
        "follow_symlinks",
        "overwrite",
        "skip_newer",
        "check_content",
        "remove_others_matching",
        "create_directories",
        "keep_structure",
        "trash_on_delete",
        "trash_on_overwrite",
        "halt_on_errors",
        "job",
    ];

    let config_map = match toml::from_str(fs::read_to_string(config_file)?.as_str()) {
        Ok(toml_text) => CfgMap::from_toml(toml_text),
        _ => {
            return Err(Error::new(
                Kind::Invalid,
                ERR_CODE_INVALID_CONFIG_FILE,
                code_to_str_parsable(ERR_CODE_INVALID_CONFIG_FILE),
            ));
        }
    };

    // check that global keys are all known: if not report offending key
    for key in config_map.keys() {
        if !allowed_globals.contains(&key.as_str()) {
            return Err(_ec_error_invalid_config(key));
        }
    }

    // strings that will be used to build actal paths
    let var_user_home = home_dir().unwrap();
    let var_config_file_dir = PathBuf::from(config_file.clone().parent().unwrap());

    let separator = if cfg!(windows) { "\\" } else { "/" };
    let mut markers: HashMap<&str, String> = HashMap::new();
    if cfg!(windows) {
        markers.insert(
            "~/",
            format!("{}{separator}", var_user_home.to_string_lossy()),
        );
        markers.insert(
            "@/",
            format!("{}{separator}", var_config_file_dir.to_string_lossy()),
        );
        markers.insert(
            "~\\",
            format!("{}{separator}", var_user_home.to_string_lossy()),
        );
        markers.insert(
            "@\\",
            format!("{}{separator}", var_config_file_dir.to_string_lossy()),
        );
    } else {
        markers.insert(
            "~/",
            format!("{}{separator}", var_user_home.to_string_lossy()),
        );
        markers.insert(
            "@/",
            format!("{}{separator}", var_config_file_dir.to_string_lossy()),
        );
    }

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
    global_config.active_jobs = cfg_mandatory!(cfg_vec_string_check_regex(
        &config_map,
        cur_key,
        &RE_JOBNAME
    ))?
    .unwrap();

    // 2. HashMap of local variables (being a map this case has no shortcut)
    let cur_key = "variables";
    let cur_item = config_map.get(cur_key);
    if !cur_item.check_that(IsMap) {
        return Err(_ec_error_invalid_config(cur_key));
    } else {
        match cur_item {
            Some(c) => {
                for (key, item) in c.as_map().unwrap().iter() {
                    if !item.is_str() {
                        return Err(_ec_error_invalid_config(cur_key));
                    }
                    global_config.variables.insert(
                        String::from(key.as_str()),
                        String::from(item.as_str().unwrap()),
                    );
                }
            }
            None => { /* OK to go, default already set */ }
        }
    }

    // 3. flags: booleans are retrieved using shortcuts, defaults are set above
    // therefore they are just kept in case of a missing configuration entry
    global_config.recursive =
        cfg_bool(&config_map, "recursive")?.unwrap_or(global_config.recursive);
    global_config.case_sensitive =
        cfg_bool(&config_map, "case_sensitive")?.unwrap_or(global_config.case_sensitive);
    global_config.follow_symlinks =
        cfg_bool(&config_map, "follow_symlinks")?.unwrap_or(global_config.follow_symlinks);
    global_config.overwrite =
        cfg_bool(&config_map, "overwrite")?.unwrap_or(global_config.overwrite);
    global_config.skip_newer =
        cfg_bool(&config_map, "skip_newer")?.unwrap_or(global_config.skip_newer);
    global_config.check_content =
        cfg_bool(&config_map, "check_content")?.unwrap_or(global_config.check_content);
    global_config.remove_others_matching = cfg_bool(&config_map, "remove_others_matching")?
        .unwrap_or(global_config.remove_others_matching);
    global_config.create_directories =
        cfg_bool(&config_map, "create_directories")?.unwrap_or(global_config.create_directories);
    global_config.keep_structure =
        cfg_bool(&config_map, "keep_structure")?.unwrap_or(global_config.keep_structure);
    global_config.trash_on_delete =
        cfg_bool(&config_map, "trash_on_delete")?.unwrap_or(global_config.trash_on_delete);
    global_config.trash_on_overwrite =
        cfg_bool(&config_map, "trash_on_overwrite")?.unwrap_or(global_config.trash_on_delete);
    global_config.halt_on_errors =
        cfg_bool(&config_map, "halt_on_errors")?.unwrap_or(global_config.halt_on_errors);

    // collect job definitions
    // note that specific job flags are directly taken from the corresponding
    // global configuration values, so filling will not be needed later; jobs
    // with no name will be considered invalid, and patterns will be recorded
    // in their combined version (in fact allowing multiple patterns is only
    // a way to facilitate writing the configuration file)
    let cur_key = "job";
    let cur_item = config_map.get(cur_key);
    match cur_item {
        Some(c) => {
            if !c.is_list() {
                return Err(_ec_error_invalid_config(cur_key));
            }
            for job_entry in c.as_list().unwrap_or(&Vec::<CfgValue>::new()).iter() {
                if !job_entry.is_map() {
                    return Err(_ec_error_invalid_config(cur_key));
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
                        trash_on_delete: global_config.trash_on_delete,
                        trash_on_overwrite: global_config.trash_on_overwrite,
                        halt_on_errors: global_config.halt_on_errors,
                    };
                    let job_map = job_entry.as_map().unwrap();
                    job.job_name =
                        cfg_mandatory!(cfg_string_check_regex(&job_map, "name", &RE_JOBNAME))?
                            .unwrap();
                    job.source_dir = PathBuf::from({
                        _ec_normalize_path_slashes(&_ec_add_trailing_slashes(
                            &(cfg_mandatory!(cfg_string(&job_map, "source"))?
                                .unwrap()
                                .replace_start(
                                    &markers.iter().map(|(k, v)| (*k, v.as_str())).collect(),
                                )
                                .replace_vars(
                                    &RE_VARMENTION_LOC,
                                    &FMT_VARMENTION_LOC,
                                    &global_config
                                        .variables
                                        .iter()
                                        .map(|(k, v)| (k.as_str(), v.as_str()))
                                        .collect(),
                                )
                                .replace_vars(
                                    &RE_VARMENTION_ENV,
                                    &FMT_VARMENTION_ENV,
                                    &sys_variables
                                        .iter()
                                        .map(|(k, v)| (k.as_str(), v.as_str()))
                                        .collect(),
                                )
                                + separator),
                        ))
                    });
                    job.destination_dir = PathBuf::from({
                        _ec_normalize_path_slashes(&_ec_add_trailing_slashes(
                            &(cfg_mandatory!(cfg_string(&job_map, "destination"))?
                                .unwrap()
                                .replace_start(
                                    &markers.iter().map(|(k, v)| (*k, v.as_str())).collect(),
                                )
                                .replace_vars(
                                    &RE_VARMENTION_LOC,
                                    &FMT_VARMENTION_LOC,
                                    &global_config
                                        .variables
                                        .iter()
                                        .map(|(k, v)| (k.as_str(), v.as_str()))
                                        .collect(),
                                )
                                .replace_vars(
                                    &RE_VARMENTION_ENV,
                                    &FMT_VARMENTION_ENV,
                                    &sys_variables
                                        .iter()
                                        .map(|(k, v)| (k.as_str(), v.as_str()))
                                        .collect(),
                                )
                                + separator),
                        ))
                    });
                    job.include_pattern = combine_regexp_patterns(
                        &cfg_mandatory!(cfg_vec_string(&job_map, "patterns_include"))?.unwrap(),
                    );
                    job.exclude_pattern = cfg_vec_string(&job_map, "patterns_exclude")?
                        .map_or(job.exclude_pattern, |v| combine_regexp_patterns(&v));
                    job.excludedir_pattern = cfg_vec_string(&job_map, "patterns_exclude_dir")?
                        .map_or(job.excludedir_pattern, |v| combine_regexp_patterns(&v));
                    job.recursive = cfg_bool(job_map, "recursive")?.unwrap_or(job.recursive);
                    job.case_sensitive =
                        cfg_bool(job_map, "case_sensitive")?.unwrap_or(job.case_sensitive);
                    job.follow_symlinks =
                        cfg_bool(job_map, "follow_symlinks")?.unwrap_or(job.follow_symlinks);
                    job.overwrite = cfg_bool(job_map, "overwrite")?.unwrap_or(job.overwrite);
                    job.skip_newer = cfg_bool(job_map, "skip_newer")?.unwrap_or(job.skip_newer);
                    job.check_content =
                        cfg_bool(job_map, "check_content")?.unwrap_or(job.check_content);
                    job.remove_others_matching = cfg_bool(job_map, "remove_others_matching")?
                        .unwrap_or(job.remove_others_matching);
                    job.create_directories =
                        cfg_bool(job_map, "create_directories")?.unwrap_or(job.create_directories);
                    job.keep_structure =
                        cfg_bool(job_map, "keep_structure")?.unwrap_or(job.keep_structure);
                    job.trash_on_delete =
                        cfg_bool(job_map, "trash_on_delete")?.unwrap_or(job.trash_on_delete);
                    job.trash_on_overwrite =
                        cfg_bool(job_map, "trash_on_overwrite")?.unwrap_or(job.trash_on_overwrite);
                    job.halt_on_errors = cfg_bool(job_map, "overwrite")?.unwrap_or(job.overwrite);

                    if job.job_name.is_empty() {
                        return Err(_ec_error_invalid_config("job_name"));
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
    for item in global_config.active_jobs.clone() {
        if !global_config.job_list.contains(&item) {
            return Err(_ec_error_invalid_config(cur_key));
        }
    }

    // now the configuration is complete (unless this function panicked)
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
fn list_files_matching(
    search_dir: &PathBuf,
    include_pattern: &str,
    exclude_pattern: &str,
    excludedir_pattern: &str,
    recursive: bool,
    follow_symlinks: bool,
    case_sensitive: bool,
) -> Option<Vec<PathBuf>> {
    // FIXME: for now erratic patterns only cause a no-match (acceptable?)
    //        in the release version they should actually return None
    let include_match = RegexBuilder::new(format!("^{include_pattern}$").as_str())
        .case_insensitive(!case_sensitive)
        .build()
        .unwrap_or(RE_MATCH_NO_FILE.clone());
    let exclude_match = RegexBuilder::new(format!("^{exclude_pattern}$").as_str())
        .case_insensitive(!case_sensitive)
        .build()
        .unwrap_or(RE_MATCH_NO_FILE.clone());

    // excluded directory is not matched as ^$, to also ignore subdirectories
    // of the excluded directory (this solution is working for now); since the
    // startup directory is already canonicalized we can use specific REs for
    // path separators on Windows and UNIX
    let ps = if cfg!(windows) { "\\" } else { "/" };
    let psre = format!("\\{ps}");
    let excludedir_match = RegexBuilder::new(format!("{psre}{excludedir_pattern}{psre}").as_str())
        .case_insensitive(!case_sensitive)
        .build()
        .unwrap_or(RE_MATCH_NO_FILE.clone());

    let depth: usize = if recursive { usize::MAX } else { 1 };
    let mut result: Vec<PathBuf> = Vec::new();
    let search_dir_strlen = search_dir.to_str().unwrap_or("").len() - 1;

    for entry in WalkDir::new(search_dir)
        .max_depth(depth)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        // skip errors
        if !(entry.file_type().is_dir() || (follow_symlinks && entry.file_type().is_symlink())) {
            let mut dir_pathbuf = PathBuf::from(entry.path());
            let subdir_name = if dir_pathbuf.pop() {
                let mut res = String::new();
                for (idx, c) in dir_pathbuf
                    .to_str()
                    .unwrap_or("")
                    .to_string()
                    .chars()
                    .enumerate()
                {
                    if idx >= search_dir_strlen {
                        res.push(c);
                    }
                }
                res.push_str(ps);
                res
            } else {
                // a value of '*' as directory reports an error ('reserved' on both Unix&Win)
                String::from("*")
            };
            if subdir_name != "*" && !excludedir_match.is_match(&subdir_name) {
                if let Some(file_name) = entry.path().file_name() {
                    if include_match.is_match(file_name.to_str().unwrap_or(""))
                        && !exclude_match.is_match(file_name.to_str().unwrap_or(""))
                    {
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
///     trash_on_overwrite: to send to garbage bin instead of overwriting
fn copy_file(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    skip_newer: bool,
    check_content: bool,
    follow_symlinks: bool,
    create_directories: bool,
    trash_on_overwrite: bool,
) -> Result<()> {
    // normalize paths
    let source_path = PathBuf::from(&source.canonicalize().unwrap_or_default());
    let destination_path = PathBuf::from(
        &destination
            .canonicalize()
            .unwrap_or(PathBuf::from(&destination)),
    );
    // NOTE: https://doc.rust-lang.org/nightly/std/fs/fn.canonicalize.html#errors
    //       `canonicalize` returns an error if the target does not exist, thus
    //       we either get the canonicalied path or the original path.

    // a flag that keeps track of whether we are overwriting or not
    let mut overwriting = false;

    // check source and destination metadata (and whether or not they exist)
    match metadata(&source_path) {
        Ok(s_stat) => {
            // first check that source <> destination
            if source_path == destination_path {
                return Err(Error::new(
                    Kind::Invalid,
                    FOERR_DESTINATION_IS_ITSELF,
                    code_to_str_readable(FOERR_DESTINATION_IS_ITSELF),
                ));
            }
            if s_stat.is_dir() {
                return Err(Error::new(
                    Kind::Invalid,
                    FOERR_SOURCE_IS_DIR,
                    code_to_str_readable(FOERR_SOURCE_IS_DIR),
                ));
            }
            if s_stat.is_symlink() && !follow_symlinks {
                // TODO: is it expected?
                return Err(Error::new(
                    Kind::Forbidden,
                    FOERR_SOURCE_IS_SYMLINK,
                    code_to_str_readable(FOERR_SOURCE_IS_SYMLINK),
                ));
            }
            match metadata(&destination_path) {
                Ok(d_stat) => {
                    // if we are here, then the destination exists: check
                    // whether overwrite is false, compare s_stat, d_stat and
                    // possibly hashes
                    if !overwrite {
                        return Err(Error::new(
                            Kind::Forbidden,
                            FOERR_DESTINATION_EXISTS,
                            code_to_str_readable(FOERR_DESTINATION_EXISTS),
                        ));
                    } else if d_stat.is_dir() {
                        return Err(Error::new(
                            Kind::Invalid,
                            FOERR_DESTINATION_IS_DIR,
                            code_to_str_readable(FOERR_DESTINATION_IS_DIR),
                        ));
                    } else if d_stat.is_symlink() && !follow_symlinks {
                        return Err(Error::new(
                            Kind::Forbidden,
                            FOERR_DESTINATION_IS_SYMLINK,
                            code_to_str_readable(FOERR_DESTINATION_IS_SYMLINK),
                        ));
                    }
                    if skip_newer {
                        match s_stat.modified() {
                            Ok(s_mtime) => match d_stat.modified() {
                                Ok(d_mtime) => {
                                    if s_mtime <= d_mtime {
                                        return Err(Error::new(
                                            Kind::Invalid,
                                            FOERR_DESTINATION_IS_NEWER,
                                            code_to_str_readable(FOERR_DESTINATION_IS_NEWER),
                                        ));
                                    }
                                }
                                Err(_) => {
                                    return Err(Error::new(
                                        Kind::Unavailable,
                                        FOERR_DESTINATION_NOT_ACCESSIBLE,
                                        code_to_str_readable(FOERR_DESTINATION_NOT_ACCESSIBLE),
                                    ));
                                }
                            },
                            // should never be reached
                            Err(_) => {
                                return Err(Error::new(
                                    Kind::Unavailable,
                                    FOERR_SOURCE_NOT_ACCESSIBLE,
                                    code_to_str_readable(FOERR_SOURCE_NOT_ACCESSIBLE),
                                ));
                            }
                        }
                    }
                    // only when asked perform content checking via SHA256
                    // and skip copy if the contents are the same
                    if check_content {
                        match sha256_digest(&source_path) {
                            Ok(source_hash) => match sha256_digest(&destination_path) {
                                Ok(destination_hash) => {
                                    if destination_hash == source_hash {
                                        return Err(Error::new(
                                            Kind::Invalid,
                                            FOERR_DESTINATION_IS_IDENTICAL,
                                            code_to_str_readable(FOERR_DESTINATION_IS_IDENTICAL),
                                        ));
                                    }
                                }
                                Err(_) => {
                                    return Err(Error::new(
                                        Kind::Unavailable,
                                        FOERR_DESTINATION_NOT_ACCESSIBLE,
                                        code_to_str_readable(FOERR_DESTINATION_NOT_ACCESSIBLE),
                                    ));
                                }
                            },
                            Err(_) => {
                                return Err(Error::new(
                                    Kind::Unavailable,
                                    FOERR_SOURCE_NOT_ACCESSIBLE,
                                    code_to_str_readable(FOERR_SOURCE_NOT_ACCESSIBLE),
                                ));
                            }
                        }
                    }

                    // if this point is reached we are actually overwriting
                    overwriting = true;
                }
                Err(_) => {
                    // in case of error check whether or not the destination
                    // directory exists, and if not create it when instructed
                    // to do so
                    // TODO: double check this part!!! If destdir is found a
                    //       CANNOT_CREATE_DIR error is propagated
                    let mut destination_dir = PathBuf::from(&destination_path);
                    if !destination_dir.pop() {
                        return Err(Error::new(
                            Kind::Forbidden,
                            FOERR_CANNOT_CREATE_DIR,
                            code_to_str_readable(FOERR_CANNOT_CREATE_DIR),
                        ));
                    }
                    match metadata(&destination_dir) {
                        Ok(d_dirdata) => {
                            if !d_dirdata.is_dir() {
                                return Err(Error::new(
                                    Kind::Forbidden,
                                    FOERR_CANNOT_CREATE_DIR,
                                    code_to_str_readable(FOERR_CANNOT_CREATE_DIR),
                                ));
                            }
                        }
                        Err(_) => {
                            // if the destination directory is not present and
                            // missing directories are to be created, only bail
                            // out on directory creation errors; otherwise it
                            // is safe to go on without further checks
                            if !create_directories {
                                return Err(Error::new(
                                    Kind::Forbidden,
                                    FOERR_CANNOT_CREATE_DIR,
                                    code_to_str_readable(FOERR_CANNOT_CREATE_DIR),
                                ));
                            }
                            if create_dir_all(&destination_dir).is_err() {
                                return Err(Error::new(
                                    Kind::Forbidden,
                                    FOERR_CANNOT_CREATE_DIR,
                                    code_to_str_readable(FOERR_CANNOT_CREATE_DIR),
                                ));
                            }
                        }
                    }
                }
            }

            // try to send the file to garbage bin if configured to do so
            // and if we are actually overwriting the destination file with
            // no opposing condition (file age, contents, accessibility, etc)
            if overwriting && trash_on_overwrite {
                let _ = trash::delete(&destination_path);
            }

            // actually copy the file using OS API
            // let res = fs::copy(&source_path, &destination_path);
            match fs::copy(&source_path, &destination_path) {
                Ok(_) => {
                    // success is returned only here, after anactually successful operation
                    Ok(())
                }
                Err(res_err) => {
                    if res_err.kind() == std::io::ErrorKind::PermissionDenied {
                        Err(Error::new(
                            Kind::Forbidden,
                            FOERR_DESTINATION_IS_READONLY,
                            code_to_str_readable(FOERR_DESTINATION_IS_READONLY),
                        ))
                    } else {
                        Err(Error::new(
                            Kind::Unknown,
                            ERR_CODE_GENERIC,
                            code_to_str_readable(ERR_CODE_GENERIC),
                        ))
                    }
                }
            }
        }
        Err(_) => Err(Error::new(
            Kind::Unavailable,
            FOERR_SOURCE_NOT_ACCESSIBLE,
            code_to_str_readable(FOERR_SOURCE_NOT_ACCESSIBLE),
        )),
    }
}

/// Attempt to remove a specified file if it exists and if allowed to. A
/// full description of the required parameters follows:
///
///     destination: the full specification of destination file
///     follow_symlinks: follow symbolic links
///     trash_on_delete: to send to garbage bin instead of deleting
fn remove_file(destination: &Path, follow_symlinks: bool, trash_on_delete: bool) -> Result<()> {
    // normalize paths
    let destination_path = destination.canonicalize().unwrap_or_default();

    match metadata(&destination_path) {
        Ok(d_stat) => {
            // if we are here, then destination exists
            if d_stat.is_dir() {
                Err(Error::new(
                    Kind::Invalid,
                    FOERR_DESTINATION_IS_DIR,
                    code_to_str_readable(FOERR_DESTINATION_IS_DIR),
                ))
            } else if d_stat.is_symlink() && !follow_symlinks {
                Err(Error::new(
                    Kind::Invalid,
                    FOERR_DESTINATION_IS_SYMLINK,
                    code_to_str_readable(FOERR_DESTINATION_IS_SYMLINK),
                ))
            } else if trash_on_delete {
                if trash::delete(&destination_path).is_err() {
                    if fs::remove_file(destination_path).is_ok() {
                        Ok(())
                    } else {
                        Err(Error::new(
                            Kind::Unavailable,
                            FOERR_DESTINATION_NOT_ACCESSIBLE,
                            code_to_str_readable(FOERR_DESTINATION_NOT_ACCESSIBLE),
                        ))
                    }
                } else {
                    Ok(())
                }
            } else if fs::remove_file(destination_path).is_ok() {
                Ok(())
            } else {
                Err(Error::new(
                    Kind::Unavailable,
                    FOERR_DESTINATION_NOT_ACCESSIBLE,
                    code_to_str_readable(FOERR_DESTINATION_NOT_ACCESSIBLE),
                ))
            }
        }
        Err(_) => Err(Error::new(
            Kind::Unavailable,
            FOERR_DESTINATION_NOT_ACCESSIBLE,
            code_to_str_readable(FOERR_DESTINATION_NOT_ACCESSIBLE),
        )),
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
/// As internal functions it also includes simple formatters for writing
/// suitable messages when needed.
fn run_single_job(job: &CopyJobConfig, verbose: bool, parsable_output: bool) -> Result<()> {
    // local helpers:

    // l1. format a message (both machine readable and verbose output)
    fn _format_message_rsj(
        parsable_output: bool,
        job: &str,
        operation: &str,
        code: i64,
        source: &Path,
        destination: &Path,
    ) -> String {
        if parsable_output {
            format_output_parsable(
                CONTEXT_JOB,
                job,
                code,
                operation,
                source.to_str().unwrap_or("<unknown>"),
                destination.to_str().unwrap_or("<unknown>"),
            )
        } else {
            match operation {
                OPERATION_JOB_COPY => {
                    if code == 0 {
                        format!(
                            "copied in job {job}: {} => {}",
                            source.display(),
                            destination.display(),
                        )
                    } else {
                        format!(
                            "error in job {job}: '{}' while copying {} => {}",
                            code_to_str_readable(code),
                            source.display(),
                            destination.display(),
                        )
                    }
                }
                OPERATION_JOB_DEL => {
                    if code == 0 {
                        format!("removed in job {job}: {}", destination.display(),)
                    } else {
                        format!(
                            "error in job {job}: '{}' while removing {}",
                            code_to_str_readable(code),
                            destination.display(),
                        )
                    }
                }
                op => {
                    format!("unexpected operation: {op}")
                }
            }
        }
    }

    // l2. format job information (both machine readable and verbose output)
    fn _format_jobinfo_rsj(
        parsable_output: bool,
        job: &str,
        operation: &str,
        code: i64,
        num_copy: usize,
        num_delete: usize,
    ) -> String {
        if parsable_output {
            format_output_parsable(
                CONTEXT_JOB,
                job,
                code,
                operation,
                &format!("{num_copy}"),
                &format!("{num_delete}"),
            )
        } else {
            match operation {
                OPERATION_JOB_BEGIN => {
                    if code == 0 {
                        format!(
                            "\
                            operations in job {job}: {num_copy} file(s) to copy, \
                            {num_delete} to possibly remove on destination"
                        )
                    } else {
                        format!("error before job {job}: '{}'", code_to_str_readable(code))
                    }
                }
                OPERATION_JOB_END => {
                    if code == 0 {
                        format!(
                            "\
                            results for job {job}: {num_copy} file(s) copied, \
                            {num_delete} removed on destination"
                        )
                    } else {
                        format!("error in job {job}: '{}'", code_to_str_readable(code))
                    }
                }
                op => {
                    format!("unexpected operation: {op}")
                }
            }
        }
    }

    // source and destination must exist and be canonicalizeable
    let source_directory = PathBuf::from(&job.source_dir.canonicalize().unwrap_or_default());
    if !source_directory.exists() {
        if verbose {
            eprintln!(
                "{}",
                _format_jobinfo_rsj(
                    parsable_output,
                    &job.job_name,
                    OPERATION_JOB_BEGIN,
                    CJERR_DESTINATION_DIR_NOT_EXISTS,
                    0,
                    0,
                )
            );
        }
        return Err(Error::new(
            Kind::Unavailable,
            CJERR_SOURCE_DIR_NOT_EXISTS,
            code_to_str_readable(CJERR_SOURCE_DIR_NOT_EXISTS),
        ));
    }
    if !job.destination_dir.exists() && !job.create_directories {
        if verbose {
            eprintln!(
                "{}",
                _format_jobinfo_rsj(
                    parsable_output,
                    &job.job_name,
                    OPERATION_JOB_BEGIN,
                    CJERR_DESTINATION_DIR_NOT_EXISTS,
                    0,
                    0,
                )
            );
        }
        return Err(Error::new(
            Kind::Unavailable,
            CJERR_DESTINATION_DIR_NOT_EXISTS,
            code_to_str_readable(CJERR_DESTINATION_DIR_NOT_EXISTS),
        ));
    }

    // build the list of files to be copied
    match list_files_matching(
        &job.source_dir,
        &job.include_pattern,
        &job.exclude_pattern,
        &job.excludedir_pattern,
        job.recursive,
        job.follow_symlinks,
        job.case_sensitive,
    ) {
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
                    job.case_sensitive,
                )
                .unwrap_or_default()
            } else {
                Vec::new()
            };
            if verbose {
                println!(
                    "{}",
                    _format_jobinfo_rsj(
                        parsable_output,
                        &job.job_name,
                        OPERATION_JOB_BEGIN,
                        ERR_CODE_OK,
                        files_to_copy.len(),
                        files_to_delete.len(),
                    )
                );
            }
            for item in files_to_copy {
                // here we also copy the file
                let destination = PathBuf::from(&job.destination_dir);
                let destfile_relative: PathBuf = if job.keep_structure {
                    PathBuf::from(&item)
                        .strip_prefix(&job.source_dir)
                        .unwrap_or(&PathBuf::from(""))
                        .to_path_buf()
                } else {
                    PathBuf::from(&item.file_name().unwrap_or(std::ffi::OsStr::new("")))
                        .to_path_buf()
                };
                if !destfile_relative.as_os_str().is_empty() {
                    let destfile_absolute = destination.join(destfile_relative);
                    // now that the destination path is known, check
                    // whether the list of matching files to delete
                    // contains it and remove it from the list: in
                    // this way the deletion process is selective and
                    // only deletes unwanted files in the target
                    // directory
                    if files_to_delete.contains(&destfile_absolute) {
                        files_to_delete.remove(
                            files_to_delete
                                .iter()
                                .position(|x| x.as_path() == destfile_absolute.as_path())
                                .unwrap(),
                        ); // cannot panic here
                    }
                    match copy_file(
                        &item,
                        &destfile_absolute,
                        job.overwrite,
                        job.skip_newer,
                        job.check_content,
                        job.follow_symlinks,
                        job.create_directories,
                        job.trash_on_overwrite,
                    ) {
                        Ok(()) => {
                            num_files_copied += 1;
                            if verbose {
                                println!(
                                    "{}",
                                    _format_message_rsj(
                                        parsable_output,
                                        &job.job_name,
                                        OPERATION_JOB_COPY,
                                        ERR_CODE_OK,
                                        &item,
                                        &destfile_absolute,
                                    )
                                );
                            }
                        }
                        Err(err) => {
                            if verbose {
                                eprintln!(
                                    "{}",
                                    _format_message_rsj(
                                        parsable_output,
                                        &job.job_name,
                                        OPERATION_JOB_COPY,
                                        err.code(),
                                        &item,
                                        &destfile_absolute,
                                    )
                                );
                            }
                            if job.halt_on_errors {
                                return Err(Error::new(
                                    Kind::Failed,
                                    CJERR_HALT_ON_COPY_ERROR,
                                    code_to_str_readable(CJERR_HALT_ON_COPY_ERROR),
                                ));
                            };
                        }
                    };
                } else {
                    if verbose {
                        eprintln!(
                            "{}",
                            _format_message_rsj(
                                parsable_output,
                                &job.job_name,
                                OPERATION_JOB_COPY,
                                CJERR_CANNOT_DETERMINE_DESTFILE,
                                &item,
                                &destination,
                            )
                        );
                    }
                    if job.halt_on_errors {
                        return Err(Error::new(
                            Kind::Failed,
                            CJERR_HALT_ON_COPY_ERROR,
                            code_to_str_readable(CJERR_HALT_ON_COPY_ERROR),
                        ));
                    }
                }
            }
            // if not remove_other_matching the vector is empty
            for item in files_to_delete {
                match remove_file(&item, job.follow_symlinks, job.trash_on_delete) {
                    Ok(()) => {
                        if verbose {
                            println!(
                                "{}",
                                _format_message_rsj(
                                    parsable_output,
                                    &job.job_name,
                                    OPERATION_JOB_DEL,
                                    ERR_CODE_OK,
                                    &PathBuf::new(),
                                    &item,
                                )
                            );
                        }
                        num_files_deleted += 1;
                    }
                    Err(err) => {
                        if verbose {
                            eprintln!(
                                "{}",
                                _format_message_rsj(
                                    parsable_output,
                                    &job.job_name,
                                    OPERATION_JOB_DEL,
                                    err.code(),
                                    &PathBuf::new(),
                                    &item,
                                )
                            );
                        }
                        if job.halt_on_errors {
                            return Err(Error::new(
                                Kind::Failed,
                                CJERR_HALT_ON_COPY_ERROR,
                                code_to_str_readable(CJERR_HALT_ON_COPY_ERROR),
                            ));
                        };
                    }
                }
            }
            if verbose {
                println!(
                    "{}",
                    _format_jobinfo_rsj(
                        parsable_output,
                        &job.job_name,
                        OPERATION_JOB_END,
                        ERR_CODE_OK,
                        num_files_copied,
                        num_files_deleted,
                    )
                );
            }
        }
        None => {
            if verbose {
                eprintln!(
                    "{}",
                    _format_jobinfo_rsj(
                        parsable_output,
                        &job.job_name,
                        OPERATION_JOB_END,
                        CJERR_NO_SOURCE_FILES,
                        0,
                        0,
                    )
                );
            }
            return Err(Error::new(
                Kind::Unavailable,
                CJERR_NO_SOURCE_FILES,
                code_to_str_readable(CJERR_NO_SOURCE_FILES),
            ));
        }
    }

    Ok(())
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
/// As internal functions it also includes simple formatters for writing
/// suitable messages when needed.
fn run_jobs(
    global_config: &CopyJobGlobalConfig,
    job_configs: &Vec<CopyJobConfig>,
) -> std::io::Result<()> {
    // local helpers:

    // l1. format a message (both machine readable and verbose output)
    fn _format_message_rj(parsable_output: bool, job: &str, code: i64) -> String {
        if parsable_output {
            format_output_parsable(CONTEXT_TASK, job, code, OPERATION_JOB_END, "", "")
        } else if code == 0 {
            format!("job {job} completed successfully")
        } else {
            format!(
                "job {job} failed with error '{}'",
                code_to_str_readable(code)
            )
        }
    }

    for job in job_configs {
        if global_config.active_jobs.contains(&job.job_name) {
            match run_single_job(job, global_config.verbose, global_config.parsable_output) {
                Ok(()) => {
                    if global_config.verbose {
                        println!(
                            "{}",
                            _format_message_rj(global_config.parsable_output, &job.job_name, 0)
                        );
                    }
                }
                Err(err) => {
                    if global_config.verbose {
                        println!(
                            "{}",
                            _format_message_rj(
                                global_config.parsable_output,
                                &job.job_name,
                                err.code()
                            )
                        );
                    }
                    if global_config.halt_on_errors {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            code_to_str_parsable(ERR_CODE_GENERIC).to_string(),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

// argument parsing and command execution: doc comments are used by clap
use clap::Parser;

/// Perform complex copy jobs according to criteria provided in a TOML file
#[derive(Parser)]
#[command(name = "copyjob", version, about)]
struct Args {
    /// Suppress all output
    #[arg(short, long)]
    quiet: bool,

    /// Generate machine readable output (JSON)
    #[arg(short = 'p', long = "parsable-output")]
    parsable_output: bool,

    /// path to configuration file
    #[arg()]
    config: String,
}

// entry point: mandatory arguments are handled by the parser
fn main() -> std::io::Result<()> {
    // formatter to write a message (here for coherence with other functions)
    fn _format_message_main(
        parsable_output: bool,
        operation: &str,
        name: &str,
        e: Option<Error>,
        msg_parsable: &str,
        msg_verbose: &str,
    ) -> String {
        match e {
            Some(err) => {
                if parsable_output {
                    format_output_parsable(
                        CONTEXT_MAIN,
                        name,
                        err.code(),
                        operation,
                        msg_parsable,
                        &err.to_string(),
                    )
                } else {
                    format!("error: {msg_verbose} / {err}")
                }
            }
            _ => {
                if parsable_output {
                    format_output_parsable(
                        CONTEXT_MAIN,
                        name,
                        ERR_CODE_OK,
                        operation,
                        msg_parsable,
                        "",
                    )
                } else {
                    format!("info: {msg_verbose}")
                }
            }
        }
    }

    let args = Args::parse();

    // configuration file name is canonicalized in order to get a correct
    // UNICODE path that includes the prefix, so that substitutions in
    // destination file names can be performed without error; an empty
    // PathBuf is produced if the file path does not exist, and this will
    // cause an error while reading the configuration
    let config = extract_config(
        &PathBuf::from(args.config)
            .canonicalize()
            .unwrap_or(PathBuf::new()),
        !args.quiet,
        args.parsable_output,
    );

    match config {
        Ok((global, jobs)) => {
            if !args.quiet {
                println!(
                    "{}",
                    _format_message_main(
                        args.parsable_output,
                        OPERATION_CONFIG,
                        global.config_file.as_os_str().to_str().unwrap_or(""),
                        None,
                        "",
                        &format!(
                            "using configuration file {}",
                            global.config_file.as_os_str().to_str().unwrap_or(""),
                        ),
                    )
                );
            }

            match run_jobs(&global, &jobs) {
                Ok(_) => {
                    if !args.quiet {
                        println!(
                            "{}",
                            _format_message_main(
                                args.parsable_output,
                                OPERATION_MAIN_END,
                                "",
                                None,
                                code_to_str_parsable(ERR_CODE_OK),
                                code_to_str_readable(ERR_CODE_OK),
                            )
                        );
                    }
                    Ok(())
                }
                Err(e) => {
                    if !args.quiet {
                        eprintln!(
                            "{}",
                            _format_message_main(
                                args.parsable_output,
                                OPERATION_MAIN_END,
                                "",
                                Some(e.into()),
                                code_to_str_parsable(ERR_CODE_GENERIC),
                                code_to_str_readable(ERR_CODE_GENERIC),
                            )
                        );
                    }
                    std::process::exit(2);
                }
            }
        }
        Err(e) => {
            if !args.quiet {
                eprintln!(
                    "{}",
                    _format_message_main(
                        args.parsable_output,
                        OPERATION_MAIN_END,
                        "",
                        Some(e),
                        code_to_str_parsable(ERR_CODE_INVALID_CONFIG_FILE),
                        code_to_str_readable(ERR_CODE_INVALID_CONFIG_FILE),
                    )
                );
            }
            std::process::exit(2);
        }
    }
}

// end.
