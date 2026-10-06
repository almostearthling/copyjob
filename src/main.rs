//! copyjob
//! An utility to perform complex copy operations based on TOML files
//! (c) 2023-2026, Francesco Garosi

use std::env;
use std::fs;

use lazy_static::lazy_static;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use regex::{Regex, RegexBuilder};

use dirs::home_dir;
use soft_canonicalize::soft_canonicalize;
use walkdir::WalkDir;

use cfgmap::{CfgMap, CfgValue, Checkable, Condition::*};

mod constants;
mod utility;

use constants::*;
use utility::cfghelp::*;
use utility::fileops::*;
use utility::logging::{LogType, init as log_init, log as log_base};
use utility::pathutils::*;
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
    active_jobs: Vec<String>, // list of active jobs in config (names)
    job_list: Vec<String>,    // list of all job names found in config
    variables: HashMap<OsString, OsString>, // variables/values defined in config
    recursive: bool,          // recurse directories
    case_sensitive: bool,     // consider filenames as case sensitive
    follow_symlinks: bool,    // follow symlinks
    overwrite: bool,          // possibly overwrite destination
    skip_newer: bool,         // do not overwrite more recent files
    check_content: bool,      // check whether contents are the same
    remove_others_matching: bool, // remove matching files not present in source
    create_directories: bool, // create non-existing directories
    keep_structure: bool,     // keep directory structure as in source
    trash_on_delete: bool,    // use garbage bin instead of deleting
    trash_on_overwrite: bool, // send to garbage bin before overwrite
    halt_on_errors: bool,     // exit job if an error occurs

    // the following parameters are defined through CLI arguments only
    dry_run: bool, // just write messages, don't actually perform jobs
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
}

// helper to convert a list of regexp patterns into a single ORed regexp
fn combine_regexp_patterns(res: &[String]) -> String {
    format!("({})", res.join("|"))
}

/// Extract the configuration from a TOML file, given the file name and the
/// pertaining arguments as resulting from the command line.
fn extract_config(
    config_file: &PathBuf,
    dry_run: bool,
) -> Result<(CopyJobGlobalConfig, Vec<CopyJobConfig>)> {
    // local helpers:

    // create a specific error
    fn error_invalid_config(key: &str) -> Error {
        Error::new_with_message(
            Kind::Invalid,
            ERR_CODE_INVALID_CONFIG_FILE,
            format!(
                "{}:{key}",
                code_to_str_parsable(ERR_CODE_INVALID_CONFIG_FILE)
            )
            .as_str(),
        )
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
        dry_run,
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
            return Err(Error::new(Kind::Invalid, ERR_CODE_INVALID_CONFIG_FILE));
        }
    };

    // check that global keys are all known: if not report offending key
    cfg_check_keys(&config_map, &allowed_globals)?;

    // strings that will be used to build actal paths
    let var_user_home = home_dir().unwrap();
    let var_config_file_dir = PathBuf::from(config_file.clone().parent().unwrap());

    // here I should use WIN_SEP and UNIX_SEP, but I really don't
    // want to allocate even more and use an owned String
    let mut markers: HashMap<&str, PathBuf> = HashMap::new();
    markers.insert("~/", var_user_home.clone());
    markers.insert("@/", var_config_file_dir.clone());
    if cfg!(windows) {
        markers.insert("~\\", var_user_home);
        markers.insert("@\\", var_config_file_dir);
    }

    let mut sys_variables: HashMap<OsString, OsString> = HashMap::new();
    for (var, value) in env::vars_os() {
        sys_variables.insert(var.clone(), value.clone());
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
        return Err(error_invalid_config(cur_key));
    } else {
        match cur_item {
            Some(c) => {
                for (key, item) in c.as_map().unwrap().iter() {
                    if !item.is_str() {
                        return Err(error_invalid_config(cur_key));
                    }
                    global_config.variables.insert(
                        OsString::from(key.as_str()),
                        OsString::from(item.as_str().unwrap()),
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
        cfg_bool(&config_map, "trash_on_overwrite")?.unwrap_or(global_config.trash_on_overwrite);
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
                return Err(error_invalid_config(cur_key));
            }
            for job_entry in c.as_list().unwrap_or(&Vec::<CfgValue>::new()).iter() {
                if !job_entry.is_map() {
                    return Err(error_invalid_config(cur_key));
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
                        cfg_mandatory!(cfg_string_check_regex(job_map, "name", &RE_JOBNAME))?
                            .unwrap();
                    job.source_dir = normalize_path_slashes(
                        interpolate_dir(
                            &(cfg_mandatory!(cfg_string(job_map, "source"))?.unwrap()),
                            &markers,
                            &global_config.variables,
                            &sys_variables,
                        )?
                        .as_path(),
                        true,
                    );
                    job.destination_dir = normalize_path_slashes(
                        interpolate_dir(
                            &(cfg_mandatory!(cfg_string(job_map, "destination"))?.unwrap()),
                            &markers,
                            &global_config.variables,
                            &sys_variables,
                        )?
                        .as_path(),
                        true,
                    );
                    job.include_pattern = combine_regexp_patterns(
                        &cfg_mandatory!(cfg_vec_string(job_map, "patterns_include"))?.unwrap(),
                    );
                    job.exclude_pattern = cfg_vec_string(job_map, "patterns_exclude")?
                        .map_or(job.exclude_pattern, |v| combine_regexp_patterns(&v));
                    job.excludedir_pattern = cfg_vec_string(job_map, "patterns_exclude_dir")?
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
                    job.halt_on_errors =
                        cfg_bool(job_map, "halt_on_errors")?.unwrap_or(job.halt_on_errors);

                    if job.job_name.is_empty() {
                        return Err(error_invalid_config("job_name"));
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
            return Err(error_invalid_config(cur_key));
        }
    }

    // now the configuration is complete (unless this function panicked)
    Ok((global_config, job_configs))
}

/// Build a list of files in a directory matching/unmatching a pattern by
/// either listing the files in that directory or traversing it recursively.
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
) -> Vec<PathBuf> {
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
    #[cfg(windows)]
    let psre = format!("\\{WIN_SEP}");
    #[cfg(unix)]
    let psre = format!("\\{UNIX_SEP}");
    let excludedir_match = RegexBuilder::new(format!("{psre}{excludedir_pattern}{psre}").as_str())
        .case_insensitive(!case_sensitive)
        .build()
        .unwrap_or(RE_MATCH_NO_FILE.clone());

    let depth: usize = if recursive { usize::MAX } else { 1 };
    let mut result: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(search_dir)
        .max_depth(depth)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        // skip errors
        if !(entry.file_type().is_dir() || (follow_symlinks && entry.file_type().is_symlink())) {
            let mut entry_dir = PathBuf::from(entry.path());
            let subdir_name = if entry_dir.pop() {
                entry_dir
                    .strip_prefix(&search_dir)
                    .map_or(None, |x| Some(x))
            } else {
                None
            };

            if let Some(subdir_name) = subdir_name
                && !excludedir_match.is_match(&subdir_name.to_str().unwrap_or(""))
                && let Some(file_name) = entry.path().file_name()
                && include_match.is_match(file_name.to_str().unwrap_or(""))
                && !exclude_match.is_match(file_name.to_str().unwrap_or(""))
            {
                result.push(PathBuf::from(entry.path()));
            }
        }
    }
    result
}

/// Perform a single copy job, by building a list of files to copy and by
/// copying them if possible using `copyfile` seen above.
fn run_job(job: &CopyJobConfig, dry_run: bool) -> Result<()> {
    let log = |severity, action, source, destination, when, status, message_code, message_extra| {
        log_base(
            severity,
            LOG_EMITTER_JOB,
            Some(job.job_name.clone()),
            action,
            source,
            destination,
            when,
            status,
            message_code,
            message_extra,
        );
    };

    // source and destination must exist and be canonicalizeable
    let source_directory = &job.source_dir.canonicalize()?;
    if !source_directory.exists() {
        log(
            LogType::Error,
            LOG_ACTION_JOB,
            Some(job.source_dir.clone()),
            None,
            LOG_WHEN_START,
            LOG_STATUS_ERR,
            CJERR_SOURCE_DIR_NOT_EXISTS,
            None,
        );
        return Err(Error::new(Kind::Unavailable, CJERR_SOURCE_DIR_NOT_EXISTS));
    }
    if !job.destination_dir.exists() && !job.create_directories {
        log(
            LogType::Error,
            LOG_ACTION_JOB,
            None,
            Some(job.destination_dir.clone()),
            LOG_WHEN_START,
            LOG_STATUS_ERR,
            CJERR_DESTINATION_DIR_NOT_EXISTS,
            None,
        );
        return Err(Error::new(
            Kind::Unavailable,
            CJERR_DESTINATION_DIR_NOT_EXISTS,
        ));
    }

    // build the list of files to be copied
    let files_to_copy = list_files_matching(
        &job.source_dir,
        &job.include_pattern,
        &job.exclude_pattern,
        &job.excludedir_pattern,
        job.recursive,
        job.follow_symlinks,
        job.case_sensitive,
    );
    if !files_to_copy.is_empty() {
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
        } else {
            // an empty vector will delete no files
            Vec::new()
        };
        log(
            LogType::Info,
            LOG_ACTION_JOB,
            Some(job.source_dir.clone()),
            Some(job.destination_dir.clone()),
            LOG_WHEN_START,
            LOG_STATUS_MSG,
            ERR_CODE_NONE,
            Some(format!("copying {} files", files_to_copy.len())),
        );
        for item in &files_to_copy {
            // here we also copy the file: if there is any error while
            // determining the destination file name, the copy operation
            // is aborted; this is however unlikely, since source file
            // names are actually retrieved from the OS
            let destination = PathBuf::from(&job.destination_dir);
            let srcfile_relative = PathBuf::from(&item)
                .strip_prefix(&job.source_dir)
                .expect("error stripping file prefix") // impossible: parent directory determines the file
                .to_path_buf();
            let destfile_relative = if job.keep_structure {
                srcfile_relative.clone()
            } else {
                PathBuf::from(&item.file_name().unwrap_or(OsStr::new(""))).to_path_buf()
            };
            if !destfile_relative.as_os_str().is_empty() {
                let destfile_absolute = destination.join(&destfile_relative);
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
                    dry_run,
                ) {
                    Ok(o) => match o {
                        Outcome::Done => {
                            num_files_copied += 1;
                            log(
                                LogType::Info,
                                LOG_ACTION_COPY,
                                Some(srcfile_relative.clone()),
                                Some(destfile_relative.clone()),
                                LOG_WHEN_PROC,
                                LOG_STATUS_OK,
                                ERR_CODE_OK,
                                Some(format!("copied")),
                            );
                        }
                        Outcome::Skipped(code) => {
                            log(
                                LogType::Info,
                                LOG_ACTION_COPY,
                                Some(srcfile_relative.clone()),
                                Some(destfile_relative.clone()),
                                LOG_WHEN_PROC,
                                LOG_STATUS_OK,
                                code,
                                Some(format!("skipped")),
                            );
                        }
                    },
                    Err(err) => {
                        log(
                            if job.halt_on_errors {
                                LogType::Error
                            } else {
                                LogType::Warn
                            },
                            LOG_ACTION_COPY,
                            Some(srcfile_relative.clone()),
                            Some(destfile_relative.clone()),
                            if job.halt_on_errors {
                                LOG_WHEN_END
                            } else {
                                LOG_WHEN_PROC
                            },
                            LOG_STATUS_ERR,
                            err.code(),
                            Some(format!("{ERR_FAILED} ({err})")),
                        );
                        if job.halt_on_errors {
                            return Err(Error::new(Kind::Failed, CJERR_HALT_ON_ERROR));
                        };
                    }
                };
            } else {
                log(
                    if job.halt_on_errors {
                        LogType::Error
                    } else {
                        LogType::Warn
                    },
                    LOG_ACTION_COPY,
                    Some(srcfile_relative.clone()),
                    Some(PathBuf::from(STR_UNDEFINED_VALUE)),
                    if job.halt_on_errors {
                        LOG_WHEN_END
                    } else {
                        LOG_WHEN_PROC
                    },
                    LOG_STATUS_ERR,
                    CJERR_CANNOT_DETERMINE_DESTFILE,
                    None,
                );
                if job.halt_on_errors {
                    return Err(Error::new(Kind::Failed, CJERR_HALT_ON_ERROR));
                }
            }
        }
        // if not remove_other_matching the vector is empty
        if !files_to_delete.is_empty() {
            log(
                LogType::Info,
                LOG_ACTION_JOB,
                Some(job.source_dir.clone()),
                Some(job.destination_dir.clone()),
                LOG_WHEN_START,
                LOG_STATUS_MSG,
                ERR_CODE_NONE,
                Some(format!("deleting {} files", files_to_delete.len())),
            );
            for item in &files_to_delete {
                let rmfile_relative = if job.keep_structure {
                    PathBuf::from(&item)
                        .strip_prefix(&job.source_dir)
                        .expect("error stripping file prefix") // impossible: parent directory determines the file
                        .to_path_buf()
                } else {
                    PathBuf::from(&item.file_name().unwrap_or(OsStr::new(""))).to_path_buf()
                };
                match remove_file(&item, job.follow_symlinks, job.trash_on_delete, dry_run) {
                    Ok(o) => match o {
                        Outcome::Done => {
                            num_files_deleted += 1;
                            log(
                                LogType::Info,
                                LOG_ACTION_DEL,
                                None,
                                Some(rmfile_relative.clone()),
                                LOG_WHEN_PROC,
                                LOG_STATUS_OK,
                                ERR_CODE_OK,
                                Some(format!("deleted")),
                            );
                        }
                        Outcome::Skipped(code) => {
                            log(
                                LogType::Info,
                                LOG_ACTION_DEL,
                                None,
                                Some(rmfile_relative.clone()),
                                LOG_WHEN_PROC,
                                LOG_STATUS_OK,
                                code,
                                Some(format!("skipped")),
                            );
                        }
                    },
                    Err(err) => {
                        log(
                            if job.halt_on_errors {
                                LogType::Error
                            } else {
                                LogType::Warn
                            },
                            LOG_ACTION_DEL,
                            None,
                            Some(rmfile_relative.clone()),
                            if job.halt_on_errors {
                                LOG_WHEN_END
                            } else {
                                LOG_WHEN_PROC
                            },
                            LOG_STATUS_ERR,
                            err.code(),
                            Some(format!("{ERR_FAILED} ({err})")),
                        );
                        if job.halt_on_errors {
                            return Err(Error::new(Kind::Failed, CJERR_HALT_ON_ERROR));
                        };
                    }
                }
            }
        }
        log(
            LogType::Info,
            LOG_ACTION_JOB,
            Some(job.source_dir.clone()),
            Some(job.destination_dir.clone()),
            LOG_WHEN_END,
            LOG_STATUS_OK,
            ERR_CODE_OK,
            Some(format!(
                "copied {} files, deleted {} files",
                num_files_copied, num_files_deleted,
            )),
        );
    } else {
        return Err(Error::new(Kind::Unavailable, CJERR_NO_SOURCE_FILES));
    }

    Ok(())
}

// this is similar to my usual exiterror
macro_rules! exit_if_fails {
    ( $quiet:expr, $might_fail:expr ) => {
        match $might_fail {
            Err(e) => {
                if !$quiet {
                    if cfg!(debug_assertions) {
                        eprintln!("{APP_NAME} error: {:?}", e);
                    } else {
                        eprintln!("{APP_NAME} error: {}", e.to_string());
                    }
                }
                std::process::exit(2);
            }
            Ok(value) => value,
        }
    };
}

// argument parsing and command execution: doc comments are used by clap
use clap::Parser;

/// Perform complex and selective copy operations according to criteria
/// provided in a TOML file
#[derive(Parser)]
#[command(name = "copyjob", version, about)]
struct Args {
    /// suppress all output
    #[arg(short, long)]
    quiet: bool,

    /// Specify the log file
    #[arg(short, long, value_name = "LOGFILE")]
    log: Option<String>,

    /// Specify the log level
    #[arg(
        short = 'L',
        long,
        value_name = "LEVEL",
        default_value_t = LogType::Warn,
        default_missing_value = "warn",
        value_enum,
    )]
    log_level: LogType,

    /// Append to an existing log file if found
    #[arg(short = 'a', long, requires = "log")]
    log_append: bool,

    /// No colors when logging (default when logging to file)
    #[arg(short = 'P', long, group = "logformat")]
    log_plain: bool,

    /// Use colors when logging (default, ignored when logging to file)
    #[arg(short = 'C', long, group = "logformat")]
    log_color: bool,

    /// Use JSON format for logging
    #[arg(short = 'J', long, group = "logformat")]
    log_json: bool,

    /// just write output without modifying the file system
    #[arg(short = 'D', long = "dry-run")]
    dry_run: bool,

    /// path to configuration file
    #[arg()]
    config: String,
}

// entry point: mandatory arguments are handled by the parser
fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // configure the logger
    let log_file_name = args.log;
    exit_if_fails!(
        args.quiet,
        log_init(
            args.log_level,
            log_file_name,
            args.log_append,
            args.log_color,
            args.log_plain,
            args.log_json,
        )
    );

    fn log(severity: LogType, when: &str, status: &str, message_code: i64, extra: Option<String>) {
        log_base(
            severity,
            LOG_EMITTER_MAIN,
            None,
            LOG_ACTION_OTHER,
            None,
            None,
            when,
            status,
            message_code,
            extra,
        );
    }

    let (global, jobs) = exit_if_fails!(args.quiet, {
        let using = PathBuf::from(args.config);
        log_base(
            LogType::Debug,
            LOG_EMITTER_CONFIG,
            None,
            LOG_ACTION_OTHER,
            Some(using.clone()),
            None,
            LOG_WHEN_INIT,
            LOG_STATUS_MSG,
            ERR_CODE_NONE,
            None,
        );
        let using = soft_canonicalize(using).unwrap_or_default(); // will result in a config error anyway
        extract_config(&using, args.dry_run)
    });

    // run all configured and active jobs, and halt on errors
    let mut res = Ok(());
    for job in jobs {
        if global.active_jobs.contains(&job.job_name) {
            res = run_job(&job, global.dry_run);
            if res.is_err() {
                break;
            }
        }
    }

    match res {
        Ok(()) => {
            log(
                LogType::Debug,
                LOG_WHEN_END,
                LOG_STATUS_OK,
                ERR_CODE_OK,
                None,
            );
            Ok(())
        }
        Err(e) => {
            log(
                LogType::Error,
                LOG_WHEN_END,
                LOG_STATUS_FAIL,
                ERR_CODE_GENERIC,
                Some(e.to_string()),
            );
            std::process::exit(2);
        }
    }
}

// end.
