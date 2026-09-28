//! Path management utilities

use regex::Regex;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use bstr::*;
use lazy_static::lazy_static;

use crate::utility::result::Result;

/// Character to use in formats to avoid clashes with file names: the
/// star (`*`) character is the best candidate as it is never legal in
/// file names, being a wildcard
pub const FORMAT_SAFE_CHAR: &str = "*";

// separators kept as constants, as well as the index until which double
// backslashes are allowed on Windows
const UNIX_SEP: u8 = b'/';

#[cfg(windows)]
const WIN_SEP: u8 = b'\\';
#[cfg(windows)]
const WIN_DSLASH_ALLOW_UPTO: usize = 2;

lazy_static! {
    // variable mention expressions: *_LOC is the mention of a variable
    // defined in the configuration file, *_ENV is the mention of a variable
    // defined in the system environment
    pub static ref RE_VARMENTION_LOC: Regex = Regex::new(r"[%]\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
    pub static ref RE_VARMENTION_ENV: Regex = Regex::new(r"[\$]\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();

    // these must have a star corresponding to the internal group of the
    // corresponding RE_VARMENTION_* instance
    pub static ref FMT_VARMENTION_LOC: String = format!("%{{{FORMAT_SAFE_CHAR}}}");
    pub static ref FMT_VARMENTION_ENV: String = format!("${{{FORMAT_SAFE_CHAR}}}");
}

/// This makes variables and markers replacement easier
///
/// Note: it only works with strings (both standard ones and OS strings),
/// which can be converted to `PathBuf`.
pub trait ReplaceVars: Sized {
    fn replace_start(&self, vars: &HashMap<Self, Self>) -> Result<Self>;
    fn replace_vars(
        &self,
        pattern: &Regex,
        format: &str,
        vars: &HashMap<Self, Self>,
    ) -> Result<Self>;
}

impl ReplaceVars for String {
    fn replace_start(&self, vars: &HashMap<String, String>) -> Result<Self> {
        let mut s = String::from(self);
        for (k, v) in vars {
            if s.starts_with(k) {
                s = s.replacen(k, v, 1);
                break;
            }
        }
        Ok(s)
    }

    fn replace_vars(
        &self,
        pattern: &Regex,
        format: &str,
        vars: &HashMap<String, String>,
    ) -> Result<Self> {
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
            let occurrence = format.replace(FORMAT_SAFE_CHAR, varname);
            result = result.replace(&occurrence, vars.get(varname).unwrap_or(&String::from("")));
        }
        Ok(result)
    }
}

impl ReplaceVars for OsString {
    fn replace_start(&self, vars: &HashMap<OsString, OsString>) -> Result<OsString> {
        let mut s = OsString::from(self);
        for (k, v) in vars {
            if s.as_encoded_bytes().starts_with(k.as_encoded_bytes()) {
                s = OsString::from(
                    s.as_os_str()
                        .as_encoded_bytes()
                        .replacen(
                            k.as_encoded_bytes(),
                            v.as_os_str().as_encoded_bytes().as_bstr(),
                            1,
                        )
                        .as_bstr()
                        .to_path()?,
                );
                break;
            }
        }
        Ok(s)
    }

    fn replace_vars(
        &self,
        pattern: &Regex,
        format: &str,
        vars: &HashMap<OsString, OsString>,
    ) -> Result<OsString> {
        let mut result = OsString::from(self);
        // mimick shell by replacing undefined variables with the empty string:
        // since the same function is used for both local and environment vars,
        // this represents a difference with the Python version, that considered
        // mentioning an undefined local variable a fatal error
        // WARNING: this actually assumes that the regular expression pattern
        //          "[%$]\{[a-zA-Z_][a-zA-Z0-9_]*\}" cannot appear in the source or
        //          the destination directory within job definitions
        while let Some(caps) = pattern.captures(
            result
                .as_os_str()
                .as_encoded_bytes()
                .as_bstr()
                .to_string()
                .as_str(),
        ) {
            let varname = caps.get(1).map_or("", |m| m.as_str());
            let occurrence = format.replace(FORMAT_SAFE_CHAR, varname);
            let bs = result.as_os_str().as_encoded_bytes();
            if let Some(replacement) = vars.get(&OsString::from(varname)) {
                let bs = bs.replace(
                    occurrence.as_bytes(),
                    replacement.as_os_str().as_encoded_bytes(),
                );
                result = OsString::from(bs.to_os_str()?);
            } else {
                let bs = bs.replace(occurrence.as_bytes(), "".as_bytes());
                result = OsString::from(bs.to_os_str()?);
            }
        }
        Ok(result)
    }
}

#[cfg(windows)]
/// convert slashes to backslashes and remove duplicate backslashes
pub fn normalize_path_slashes(path: &Path, add_trailing: bool) -> PathBuf {
    // we build a target string which is at most as big as the origin, plus
    // one byte to completely avoid reallocation (see below)
    let bpath = path.as_os_str().as_encoded_bytes().as_bstr();
    let mut bres = BString::new(Vec::with_capacity(bpath.len() + 1));

    for (cnt, c) in bpath.iter().enumerate() {
        // convert unix separators to win separators
        let c = if *c == UNIX_SEP { WIN_SEP } else { *c };

        // append the new character as long as it is possible: if it is a
        // separator, we must at least have passed the second path character
        // because double backslashes are allowed at the beginning of UNC
        // paths, or the result must not end with a backslash; otherwise
        // append everything
        if c == WIN_SEP {
            if cnt < WIN_DSLASH_ALLOW_UPTO || !bres.ends_with(&[c]) {
                bres.push(c);
            }
        } else {
            bres.push(c);
        }
    }

    // this is the part where the new bstring might have needed to be extended
    // if we didn't add the extra byte of capacity at the beginning
    if add_trailing {
        bres.push(WIN_SEP);
    }

    // this is not unsafe because the bytes we are working on were
    // originated just above, and no encoding mismatch can take place
    PathBuf::from(unsafe { OsStr::from_encoded_bytes_unchecked(bres.as_slice()) })
}

#[cfg(unix)]
/// remove duplicate slashes
pub fn normalize_path_slashes(path: &Path, add_trailing: bool) -> PathBuf {
    // we build a target string which is at most as big as the origin, plus
    // one byte to completely avoid reallocation (see below)
    let bpath = path.as_os_str().as_encoded_bytes().as_bstr();
    let mut bres = BString::new(Vec::with_capacity(bpath.len()));

    for c in bpath.iter() {
        // append the new character as long as it is possible: if it is a
        // separator it is appended only if the result does not already end
        // with a separator
        if *c == UNIX_SEP && !bres.ends_with(&[*c]) {
            bres.push(*c);
        }
    }

    // this is the part where the new bstring might have needed to be extended
    // if we didn't add the extra byte of capacity at the beginning
    if add_trailing {
        bres.push(UNIX_SEP);
    }

    // this is not unsafe because the bytes we are working on were
    // originated just above, and no encoding mismatch can take place
    PathBuf::from(unsafe { OsStr::from_encoded_bytes_unchecked(bres.as_slice()) })
}

pub fn interpolate_dir(
    raw: &str,
    markers: &HashMap<&str, PathBuf>,
    local_vars: &HashMap<OsString, OsString>,
    sys_vars: &HashMap<OsString, OsString>,
) -> Result<PathBuf> {
    Ok(PathBuf::from({
        OsString::from(raw)
            .replace_start(
                &markers
                    .iter()
                    .map(|(k, v)| (OsString::from(*k), v.as_os_str().to_owned()))
                    .collect(),
            )?
            .replace_vars(
                &RE_VARMENTION_LOC,
                &FMT_VARMENTION_LOC,
                &local_vars
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )?
            .replace_vars(
                &RE_VARMENTION_ENV,
                &FMT_VARMENTION_ENV,
                &sys_vars
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )?
    }))
}

// end.
