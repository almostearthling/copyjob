//! The common logging system is a simplified version of what is available in
//! the `log` crate, and all logging functions shall use this common module.
//!
//! Some notes on logging:
//!
//! * The log messages are composed by
//!     - the timestamp
//!     - the application name (see below) in brackets
//!     - the log level
//!     - the log message
//! * The log message in turn has the following form:
//!   `context: [MSGTYPE] human readable message`
//!   where
//!     - the context is usually constructed with two space-separated strings
//!       indicating the part of the program where a certain message is issued
//!     - MSGTYPE (in square brackets) consists of two or more alphanumeric
//!       strings, separated by slashes, whose first two are described below
//!       and the further ones may depend on the first two
//!     - the human readable message is an explanation of what happened.
//!
//! The first two elements in MSGTYPE indicate in which point of an operation
//! the event occurs, and the type of event. The first element can be one of:
//!
//! * _INIT_ if the event occurs in an initialization phase
//! * _START_ if the event occurs while starting something, service or process
//! * _PROC_ if the event occurs while processing or during some activity
//! * _END_ if the event occurs at the end of a service or process
//!
//! while the second can be one of:
//!
//! * _OK_ for expected outcomes or behaviours
//! * _FAIL_ for unexpected outcomes or behaviours
//! * _IND_ for indeterminate outcomes
//! * _MSG_ if the human-readable part is exclusively informational
//! * _ERR_ for actual errors
//!
//! This should help using the log as a way of communicating to a wrapper
//! utility the state of the scheduler, and possibily give the opportunity to
//! organize communication to the user in a friendlier way.

use crate::constants::{
    APP_NAME, ERR_LOGGER_NOT_INITIALIZED, LOG_ACTION_COPY, LOG_ACTION_DEL, LOG_ACTION_JOB,
    LOG_ACTION_OTHER, LOG_EMITTER_CONFIG, LOG_EMITTER_GLOBAL, LOG_EMITTER_JOB, LOG_EMITTER_MAIN,
    LOG_STATUS_ERR, LOG_STATUS_FAIL, LOG_STATUS_IND, LOG_STATUS_MSG, LOG_STATUS_OK, LOG_WHEN_END,
    LOG_WHEN_INIT, LOG_WHEN_PROC, LOG_WHEN_START, STR_UNDEFINED_VALUE,
};
use crate::utility::result::{code_to_str_parsable, code_to_str_readable};
use clap::ValueEnum;
use flexi_logger::{DeferredNow, FileSpec, Logger, style};
use log::Record;
use log::{debug, error, info, trace, warn};
use nu_ansi_term::Style;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

// the following global flag is exposed here because it looks like there is
// no actual way to pass anything but a string as payload to the logger, so
// the common logging function should know whether the logger is initialized
// to return JSON message and build the JSON payload itself
// lazy_static! {
//     static ref LOGGER_EMITS_JSON: RwLock<bool> = RwLock::new(false);
// }

static LOGGER_EMITS_JSON: AtomicBool = AtomicBool::new(false);

// time stamp format that is used by the provided format functions.
const NOW_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f";
const NOW_FMT_FULL: &str = "%Y-%m-%dT%H:%M:%S%.6f";

// log formatters
fn log_format_plain(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] ({APP_NAME}) {} {}",
        now.format(NOW_FMT),
        format_args!("{:5}", record.level()),
        record.args(),
    )
}

fn log_format_json(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let header = json!({
        "application": APP_NAME,
        "time": now.format(NOW_FMT_FULL).to_string(),
        "level": record.level().to_string(),
    });
    let payload = record.args();
    write!(w, "{{\"header\":{header},\"contents\":{payload}}}")
}

fn log_format_colors(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    let bold = Style::new().bold();
    let dimmed = Style::new().dimmed();
    write!(
        w,
        "[{}] {} {} {}",
        format_args!("{}", now.format(NOW_FMT)),
        dimmed.paint(format!("({APP_NAME})")),
        style(level).paint(format!("{:5}", level.to_string())),
        bold.paint(record.args().to_string()),
    )
}

/// Log levels (from most verbose to least)
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum LogType {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Logger initialization: if `filename` is not given, the log will be
/// sent to stdout and use color (and the `append` parameter will be
/// ignored); otherwise `filename` will be used as path for the log file:
/// causes an error if it's not possible to open the log file.
pub fn init(
    level: LogType,
    filename: Option<String>,
    append: bool,
    logcolor: bool, // these three values are mutually
    logplain: bool, // exclusive by construction of the
    logjson: bool,  // main `clap` parser
) -> std::io::Result<bool> {
    let level = match level {
        LogType::Trace => "trace",
        LogType::Debug => "debug",
        LogType::Info => "info",
        LogType::Warn => "warn",
        LogType::Error => "error",
    };

    // the following line is to avoid other crates logging (e.g. `zbus`)
    // so it can be commented out for debugging purposes and replaced with
    // the subsequent commented out line. A reminder to documentation:
    // https://docs.rs/flexi_logger/latest/flexi_logger/struct.LogSpecification.html
    // FIXME: maybe we can choose the actual configuration string
    //        automatically according to the current build settings?
    let logspec = format!("{APP_NAME}={level}");
    // let logspec = format!("{level}");

    let mut logger;
    logger = Logger::try_with_str(logspec);
    match logger {
        Ok(l) => {
            if let Some(fname) = filename {
                let log_format = {
                    if logcolor {
                        log_format_plain
                    } else if logplain {
                        log_format_plain
                    } else if logjson {
                        LOGGER_EMITS_JSON.store(true, Relaxed);
                        log_format_json
                    } else {
                        log_format_plain
                    }
                };
                let mut pb = PathBuf::from(&fname);
                if pb.parent().is_none() || pb.parent().unwrap().to_str().unwrap_or("").is_empty() {
                    pb = {
                        let mut dir = PathBuf::from(".");
                        dir.push(pb);
                        dir
                    }
                }
                let fspec =
                    FileSpec::try_from(&pb).map_err(|e| std::io::Error::other(e.to_string()))?;
                logger = Ok(
                    l.log_to_file(fspec).format_for_files(log_format), // .write_mode(WriteMode::BufferAndFlush)
                );
                if append {
                    logger = Ok(logger.unwrap().append());
                }
            } else {
                let log_format = {
                    if logcolor {
                        log_format_colors
                    } else if logplain {
                        log_format_plain
                    } else if logjson {
                        LOGGER_EMITS_JSON.store(true, Relaxed);
                        log_format_json
                    } else {
                        log_format_colors
                    }
                };
                // in json mode to console we also support capture by pipes
                // so that wrappers may use stdout to get updates; it also
                // redirects the logger's own errors to a black hole in
                // order to avoid polluting a wrapper
                if logjson {
                    logger = Ok(l
                        .format_for_stdout(log_format)
                        .write_mode(flexi_logger::WriteMode::Direct)
                        .error_channel(flexi_logger::ErrorChannel::DevNull)
                        .log_to_stdout());
                } else {
                    logger = Ok(l.format_for_stdout(log_format).log_to_stdout());
                }
            }
        }
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                ERR_LOGGER_NOT_INITIALIZED,
            ));
        }
    }
    if let Err(_e) = logger.unwrap().start() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            ERR_LOGGER_NOT_INITIALIZED,
        ));
    }

    Ok(true)
}

/// Common log function. The parameters are granular in order to achieve
/// two benefits: the first is that for most of them a constant can be
/// used, thus reducing the possibility of non-conformant log messages
/// (which may arise on typos) and, to some extent, the executable size;
/// the second is that JSON log messages can be as fine-grained as
/// needed. The constants to be used are defined in _constants.rs_, and
/// in particular:
///
/// * `emitter` is one of the `LOG_EMITTER_...` constants
/// * `action` is one of the `LOG_ACTION_...` constants
/// * `when` is one of the `LOG_WHEN_...` constants
/// * `status` is one of the `LOG_STATUS_...` constants
///
/// while non-constant parameters must be defined as follows
///
/// * `item` can be a tuple consisting of item _name_ and _id_
/// * `message` is the only arbitrary string that can be passed
///
/// This allows JSON messages to be easily interpretable by a wrapper
/// according to the hints given in the documentation.
pub fn log(
    severity: LogType,
    emitter: &str,
    item: Option<String>,
    action: &str,
    source: Option<PathBuf>,
    destination: Option<PathBuf>,
    when: &str,
    status: &str,
    message_code: i64,
    message_extra: Option<String>,
) {
    // we can afford to check that logging is consistent when debugging,
    // just to avoid surprises in release veersions: this might seem
    // heavy but it doesn't even compile in release mode
    debug_assert!(
        vec![
            LOG_EMITTER_MAIN,
            LOG_EMITTER_CONFIG,
            LOG_EMITTER_GLOBAL,
            LOG_EMITTER_JOB,
        ]
        .contains(&emitter)
    );
    debug_assert!(
        vec![
            LOG_ACTION_COPY,
            LOG_ACTION_DEL,
            LOG_ACTION_JOB,
            LOG_ACTION_OTHER,
        ]
        .contains(&action)
    );
    debug_assert!(
        vec![LOG_WHEN_INIT, LOG_WHEN_START, LOG_WHEN_END, LOG_WHEN_PROC,].contains(&when)
    );
    debug_assert!(
        vec![
            LOG_STATUS_OK,
            LOG_STATUS_FAIL,
            LOG_STATUS_ERR,
            LOG_STATUS_MSG,
            LOG_STATUS_IND,
        ]
        .contains(&status)
    );

    // when configuring, only the source is set (to the config file)
    debug_assert!(emitter != LOG_EMITTER_CONFIG || (destination.is_none() && source.is_some()));
    // when logging COPY, both source and destionation must have a value
    debug_assert!(action != LOG_ACTION_COPY || (destination.is_some() && source.is_some()));
    // when logging DEL, destination must be specified but not source
    debug_assert!(action != LOG_ACTION_DEL || (destination.is_some() && source.is_none()));
    // when logging JOB, destination or source (dirs) must be Some
    debug_assert!(action != LOG_ACTION_JOB || (destination.is_some() || source.is_some()));

    let payload = if LOGGER_EMITS_JSON.load(Relaxed) {
        json!({
            "context": json!({
                "emitter": emitter,
                "item": item,
                "action": action,
                "source": &source.map(|p| p.to_string_lossy().to_string()),
                "destination": &destination.map(|p| p.to_string_lossy().to_string()),
            }),
            "message_type": json!({
                "when": when,
                "status": status,
            }),
            "message": json!({
                "id": code_to_str_parsable(message_code),
                "value": code_to_str_readable(message_code),
                "extra": message_extra,
            }),
        })
        .to_string()
    } else {
        let item_repr = if let Some(item) = item {
            format!(" [{item}]")
        } else {
            String::new()
        };
        let action_repr = match action {
            LOG_ACTION_COPY => {
                // we know that source and destination are not None
                format!(
                    "{action}: `{}` -> `{}`",
                    source
                        .unwrap_or(PathBuf::from(STR_UNDEFINED_VALUE))
                        .to_string_lossy(),
                    destination
                        .unwrap_or(PathBuf::from(STR_UNDEFINED_VALUE))
                        .to_string_lossy(),
                )
            }
            LOG_ACTION_DEL => {
                format!("{action}: `{}`", destination.unwrap().to_string_lossy(),)
            }
            LOG_ACTION_JOB => {
                format!(
                    "COPY_JOB: `{}` -> `{}`",
                    source.unwrap().to_string_lossy(),
                    destination.unwrap().to_string_lossy(),
                )
            }
            _ => {
                if emitter == LOG_EMITTER_CONFIG {
                    format!("CONFIG (using `{}`)", source.unwrap().to_string_lossy())
                } else {
                    format!("{action}")
                }
            }
        };
        let message = if let Some(s) = message_extra {
            format!("{} / {s}", code_to_str_readable(message_code))
        } else {
            format!("{}", code_to_str_readable(message_code))
        };
        format!("{emitter}{item_repr} {action_repr}: [{when}/{status}] {message}")
    };
    match severity {
        LogType::Trace => {
            trace!("{payload}")
        }
        LogType::Debug => {
            debug!("{payload}")
        }
        LogType::Info => {
            info!("{payload}")
        }
        LogType::Warn => {
            warn!("{payload}")
        }
        LogType::Error => {
            error!("{payload}")
        }
    }
}
