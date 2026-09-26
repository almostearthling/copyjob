//! Path management utilities

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::ffi::{OsStr, OsString};
use regex::Regex;

use lazy_static::lazy_static;
use bstr::*;


use crate::utility::result::Result;


lazy_static! {
    // regexs to normalize slashes
    static ref RE_NORMALIZE_SLASHES: Regex = Regex::new(if cfg!(windows) { "\\[\\]+" } else { "/[/]+" }).unwrap();
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
                s = s.replace(k, v);
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
            let occurrence = format.replace("*", varname);
            if let Some(replacement) = vars.get(varname) {
                result = result.replace(&occurrence, replacement);
            } else {
                result = result.replace(&occurrence, "");
            }
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
                        .replace(
                            k.as_encoded_bytes(),
                            v.as_os_str().as_encoded_bytes().as_bstr(),
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
        // WARNING: conversion from bytes to a string to a str may b lossy
        while let Some(caps) = pattern.captures(
            result
                .as_os_str()
                .as_encoded_bytes()
                .as_bstr()
                .to_string()
                .as_str(),
        ) {
            let varname = caps.get(1).map_or("", |m| m.as_str());
            let occurrence = format.replace("*", varname);
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

// pub fn normalize_path_slashes(path: &Path) -> Option<PathBuf> {
//     let s = BString::from(path.as_os_str().as_bytes());
//     let s1 = if cfg!(windows) {
//         s.replace("/", "\\").as_bstr()
//     } else  {
//         s.as_bstr()
//     };

//     Some(PathBuf::from(
//         RE_NORMALIZE_SLASHES.replace_all(s1, if cfg!(windows) { "\\" } else { "/" })
//     ))
// }


// end.

