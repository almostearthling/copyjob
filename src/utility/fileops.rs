//! File operations: copy or delete a file

use std::fs;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::metadata;
use std::path::{Path, PathBuf};

use std::io::BufReader;
use std::io::Read;
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};

use crate::utility::result::*;
use crate::constants::*;


// helper to calculate hash for a single file
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
pub fn copy_file(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    skip_newer: bool,
    check_content: bool,
    follow_symlinks: bool,
    create_directories: bool,
    trash_on_overwrite: bool,
    dry_run: bool,
) -> Result<()> {
    // normalize paths
    let source_path = source.canonicalize().unwrap_or_default();
    let destination_path = destination
        .canonicalize()
        .unwrap_or(PathBuf::from(&destination));
    // NOTE: https://doc.rust-lang.org/nightly/std/fs/fn.canonicalize.html#errors
    //       `canonicalize` returns an error if the target does not exist, thus
    //       we either get the canonicalized path or the original path.

    // a flag that keeps track of whether we are overwriting or not
    let mut overwriting = false;

    // check source and destination metadata (and whether or not they exist)
    match metadata(&source_path) {
        Ok(s_stat) => {
            // first check that source <> destination
            if source_path == destination_path {
                return Err(Error::new(Kind::Invalid, FOERR_DESTINATION_IS_ITSELF));
            }
            if s_stat.is_dir() {
                return Err(Error::new(Kind::Invalid, FOERR_SOURCE_IS_DIR));
            }
            if s_stat.is_symlink() && !follow_symlinks {
                // TODO: is it expected?
                return Err(Error::new(Kind::Forbidden, FOERR_SOURCE_IS_SYMLINK));
            }
            match metadata(&destination_path) {
                Ok(d_stat) => {
                    // if we are here, then the destination exists: check
                    // whether overwrite is false, compare s_stat, d_stat and
                    // possibly hashes
                    if !overwrite {
                        return Err(Error::new(Kind::Forbidden, FOERR_DESTINATION_EXISTS));
                    } else if d_stat.is_dir() {
                        return Err(Error::new(Kind::Invalid, FOERR_DESTINATION_IS_DIR));
                    } else if d_stat.is_symlink() && !follow_symlinks {
                        return Err(Error::new(Kind::Forbidden, FOERR_DESTINATION_IS_SYMLINK));
                    }

                    if skip_newer && s_stat.modified()? <= d_stat.modified()? {
                        return Err(Error::new(Kind::Invalid, FOERR_DESTINATION_IS_NEWER));
                    }

                    // only when asked perform content checking via SHA256
                    // and skip copy if the contents are the same
                    if check_content
                        && sha256_digest(&source_path)? == sha256_digest(&destination_path)?
                    {
                        return Err(Error::new(Kind::Invalid, FOERR_DESTINATION_IS_IDENTICAL));
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
                        return Err(Error::new(Kind::Forbidden, FOERR_CANNOT_CREATE_DIR));
                    }
                    match metadata(&destination_dir) {
                        Ok(d_dirdata) => {
                            if !d_dirdata.is_dir() {
                                return Err(Error::new(Kind::Forbidden, FOERR_CANNOT_CREATE_DIR));
                            }
                        }
                        Err(_) => {
                            // if the destination directory is not present and
                            // missing directories are to be created, only bail
                            // out on directory creation errors; otherwise it
                            // is safe to go on without further checks
                            if !create_directories {
                                return Err(Error::new(Kind::Forbidden, FOERR_CANNOT_CREATE_DIR));
                            }
                            if create_dir_all(&destination_dir).is_err() {
                                return Err(Error::new(Kind::Forbidden, FOERR_CANNOT_CREATE_DIR));
                            }
                        }
                    }
                }
            }

            if !dry_run {
                // try to send the file to garbage bin if configured to do so
                // and if we are actually overwriting the destination file with
                // no opposing condition (file age, contents, accessibility, etc)
                if overwriting && trash_on_overwrite {
                    let _ = trash::delete(&destination_path);
                }

                // actually copy the file using OS API
                fs::copy(&source_path, &destination_path)
                    .map(|_| ())
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            Error::new(Kind::Forbidden, FOERR_DESTINATION_IS_READONLY)
                        } else {
                            Error::new(Kind::Unknown, ERR_CODE_GENERIC)
                        }
                    })
            } else {
                Ok(())
            }
        }
        Err(_) => Err(Error::new(Kind::Unavailable, FOERR_SOURCE_NOT_ACCESSIBLE)),
    }
}

/// Attempt to remove a specified file if it exists and if allowed to.
pub fn remove_file(
    destination: &Path,
    follow_symlinks: bool,
    trash_on_delete: bool,
    dry_run: bool,
) -> Result<()> {
    // normalize paths
    let destination_path = destination.canonicalize().unwrap_or_default();

    match metadata(&destination_path) {
        Ok(d_stat) => {
            // if we are here, then destination exists
            if d_stat.is_dir() {
                Err(Error::new(Kind::Invalid, FOERR_DESTINATION_IS_DIR))
            } else if d_stat.is_symlink() && !follow_symlinks {
                Err(Error::new(Kind::Invalid, FOERR_DESTINATION_IS_SYMLINK))
            } else if trash_on_delete {
                if !dry_run {
                    trash::delete(&destination_path)
                        .or_else(|_| fs::remove_file(destination_path))
                        .map_err(|_| {
                            Error::new(Kind::Unavailable, FOERR_DESTINATION_NOT_ACCESSIBLE)
                        })
                } else {
                    Ok(())
                }
            } else if !dry_run {
                fs::remove_file(destination_path)
                    .map_err(|_| Error::new(Kind::Unavailable, FOERR_DESTINATION_NOT_ACCESSIBLE))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(Error::new(
            Kind::Unavailable,
            FOERR_DESTINATION_NOT_ACCESSIBLE,
        )),
    }
}

// end.
