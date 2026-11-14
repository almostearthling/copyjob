# **copyjob**


This is a small utility that can perform selective file copy operations according to directives provided in a TOML configuration file. Each configuration file can contain instructions for several copy operations, and the parameters may vary for each operation defined in the configuration: such parameters can be defined globally in the TOML file and overridden partially or totally in each job.

> [!WARNING]
> **BREAKING CHANGES:** this version of **copyjob** introduces some breaking changes, both in the output format (which now is more similar to a log file, unless JSON is chosen as output format) and in the behaviour of the utility itself. Some CLI parameters are different from the previous versions, so scripts using **copyjob** should be adjusted consequently; and although the configuration format is unchanged, some configurations that could have been accepted in previous versions are now considered errors - such as mentioning local variables in other local variables. Also, for safety reasons, in this release the `follow_symlinks` parameter always defaults to *false*.

Jobs are related to a source and a destination directory: it is possible to create the destination path if it doesn't exist, and to recreate totally or partially the source structure at the destination. Jobs can be instructed to copy files, according to selection criteria specified in the configuration: such criteria include file name patterns for both inclusion and exclusion, checking file age and/or contents compared to a possibly existing destination file, and patterns for subdirectories to skip. A job can also remove files existing in the destination directory if they match the naming criteria provided for the source and are not present in the source directory.

File name patterns are passed to **copyjob** as *regular expressions* in the TOML configuration file, as this gives more granularity while specifying what has to be chosen and what to skip, compared to the wildcard semantics found in most shells. All patterns are provided as lists of regular expression strings, so that matching any pattern in the list items is considered a good match for the given condition.

For a description of TOML as a configuration file language, see the language [specification](https://toml.io/), and for a description of regular expression syntax and semantics, see [Wikipedia](https://en.wikipedia.org/wiki/Regular_expression); there are many useful tutorials around about *RE*s, and also many syntax checkers and interactive tools to test patterns before using them in real life cases such as **copyjob** jobs.

> [!CAUTION]
> This software is in early development stage. Besides [known issues](#known-problems), it might behave in a way other than expected in cases that I haven't been able to personally test. No file in the source directory should be at risk (unless there are references/links to it in the destination directory), but the destination directory is in control of **copyjob**: in case of incorrect configurations, or even bugs, files located there are subject to being overwritten or deleted.


## Usage

**copyjob** is invoked from the command line. By typing `copyjob --help` at the prompt, the utility will display a brief usage message:

```text
Perform selective file copy operations according to a configuration file

Usage: copyjob.exe [OPTIONS] <CONFIG>

Arguments:
  <CONFIG>  Path to configuration file

Options:
  -q, --quiet              Suppress all output
  -l, --log <LOGFILE>      Specify the log file
  -L, --log-level <LEVEL>  Specify the log level [default: warn] [possible values: trace, debug, info, warn, error]
  -a, --log-append         Append to an existing log file if found
  -P, --log-plain          No colors when logging (default when logging to file)
  -C, --log-color          Use colors when logging (default, ignored when logging to file)
  -J, --log-json           Use JSON format for logging
  -D, --dry-run            Just log what would be copied without modifying the file system
  -h, --help               Print help
  -V, --version            Print version
  ```

When called with `--quiet` or `-q` as parameter, followed by the configuration file path, will only exit with an *error* (nonzero) value in case of errors: a log is produced only when a log file is specified; When **copyjob** is invoked with the `--log-json` or `-J` parameter, it will produce output in JSON format, which is easier for an invoking frontend to parse, although more difficult for a human to read. In order to test a configuration and see what modifications it would produce, without actually touching the file system, the `--dry-run` (or its `-D` short form) can be used.

> [!NOTE]
> The `--dry-run` produced output is the same as if **copyjob** were actually performing the operations specified in the configuration file: no extra field or string is reported in the log or to the console to remind that current run is not effective. Also, `--dry-run` may report as successful operations that would not succeed in actual jobs, since it does not check for permission errors: such errors, in copy/deletion operations or while creating directories, can only take place in actual runs.

The basic invocation is

```sh
copyjob path/to/config.toml
```

which will read the file `path/to/config.toml` and perform the jobs that the user defined and activated there, producing a readable output when any warning or error arises. To get more (or less) detail in the output, a higher (or lower) log level can be chosen; levels are `trace`, `debug`, `info`, `warn`, and `error`, from most to least verbose.


## Configuration

Configuration files for **copyjob** are standard TOML files which generally consist of two sections each, that is

* a *global* section which defines the list of jobs to be performed, the default values for job related parameters, and some configuration *variables*, that is, values that can be used as placeholders in path parameters throughout the configuration, and
* a *jobs* section, where jobs are defined.

The detail below better describes how **copyjob** configuration files are structured.


### An example of configuration file

In order to understand what **copyjob** can do, let's provide a simple example of a working configuration file:

```toml
# sample configuration file

# jobs to actually perform
active_jobs = [
  "Reports",
#  "CurrentSummary",
]

# override defaults globally
case_sensitive = false

# variables defined at configuration level
[variables]
SOURCE_BASE = "${HOME}/Documents"
DEST_BASE = "${HOME}/CloudSync/SyncedDocs"

# list of all jobs
[[job]]
name = "Reports"
source = "%{SOURCE_BASE}/MyData"
destination = "%{DEST_BASE}/Reports"
keep_structure = false
patterns_include = [
  'Report_.*\.pdf',
  'Report_.*\.docx?',
  'Report_.*\.xlsx?',
]

[[job]]
name = "CurrentSummary"
source = "%{SOURCE_BASE}/MyData"
destination = "%{DEST_BASE}/Summary"
patterns_include = ['Current Summary v[1-9][0-9]*\.pptx?']
recursive = false

# end.
```

This very simple configuration file defines two different jobs that **copyjob** will be able to perform:

1. *Reports*: will copy Word&trade;, Excel&trade; and PDF files whose names begin with the word *Report* followed by an underscore and any other characters: the job will copy both old and new Word and Excel files (the *RE* states that the *x* at the end is optional); this job walks through subdirectories, but any matching files found in the subdirectories will be copied in the destination folder without recreating the source directory structure,
2. *CurrentSummary*: will copy all the PowerPoint&trade; presentations, created with either older or recent editions of PowerPoint, whose file name is *Current Summary* followed by a space and a version number with no dots in it; this job will ignore all subdirectories of the source folder.

Both jobs start searching in the *Documents/MyData* subfolder of the user home: this is achieved by using the `HOME` environment variable (it only works on UNIX-like systems, unless `HOME` is properly defined, Windows uses `USERPROFILE` instead); also, the destination is in both cases a subfolder of the *CloudSync/SyncedDocs* folder that might or might not exist in the same user home directory. Because of the default configuration of **copyjob**, the appropriate destination folders are created by the utility itself in case they are not found.

At the global level we instruct **copyjob** to treat file names as case insensitive, to make sure we don't miss files that could have be named slightly differently on operating systems such as Windows. Also, we define a couple of placeholders to use in source and destination directories in order to easily change the configuration of all jobs simultaneously in case the source or the target changes: for example, if the destination directory for all jobs changes to */cloud/Synced*, it's easy to modify the
local variable `DEST_BASE` to `"/cloud/Synced"`, so that all the defined jobs that mention this variable (in this case, both) change accordingly.

Although the configuration file defines two different jobs, in this case only one of them will be performed, that is *Reports*: this happens because the *CurrentSummary* job is commented out in the `active_jobs` list. This is useful when some jobs have to be temporarily excluded especially in highly automated tasks.

Also note the system variable `HOME` in the paths (or better, in this case, in the local variables that are used in the source and target paths) that will be replaced on UNIX-like systems by the actual user home directory; we will see that this restriction can be overridden on Windows without having to explicitly define `HOME` by using a specific [marker](#special-directory-markers).

A template configuration file is provided, in *copyjob_template.toml*.


### Configuration at the global level

Most parameters can be defined at the global level in the configuration file, and the values defined here are shared by all jobs. On the other hand, jobs can override such values partly or even totally - except for the list of active jobs and the local variables, which can only be defined at the global level. Overridable parameters are the following, and when omitted the corresponding default value will be used:

| **Name**                 | **Default** | **Description**                                       |
|--------------------------|-------------|-------------------------------------------------------|
| `recursive`              | true        | walk subdirectories                                   |
| `overwrite`              | true        | overwrite existing files                              |
| `skip_newer`             | true        | skip if destination is not older than source          |
| `check_content`          | false       | check contents of files using a SHA-256 hash          |
| `follow_symlinks`        | false       | follow symbolic links                                 |
| `case_sensitive`         | true        | treat patterns/filenames as case sensitive            |
| `create_directories`     | true        | create directory structure if missing                 |
| `keep_structure`         | true        | keep subdirectory structure, otherwise flat           |
| `halt_on_errors`         | false       | stop running after first job error                    |
| `trash_on_delete`        | true        | use garbage bin instead of deleting                   |
| `trash_on_overwrite`     | false       | use garbage bin before overwriting                    |
| `remove_others_matching` | false       | remove matching files at destination if not in source |

As said, a list of active jobs has to be defined. This is done by defining the mandatory list:

```toml
active_jobs = [
  "name1",
  "name2",
  # ...
]
```

where the jobs *name1*, *name2*, and so on *must* be defined, otherwise the command will exit with an error. Thanks to TOML flexible syntax, the array can span multiple lines - which is useful for commenting out jobs when not needed. The job execution order is the order in which it appears on the list. Note that a job can appear multiple times, in which case **copyjob** will execute it as many times as it is mentioned, and at the positions where it is mentioned.

The optional `[variables]` section can be used to define local variables: their string values replace occurrences of the pattern `%{VAR_NAME}` in source and destination paths (but *not* in regular expression patterns), so that the string `%{VAR_NAME}/path` becomes `/some/path` if `VAR_NAME` is assigned the value `/some`. The syntax is the following:

```toml
[variables]
VAR1 = "some value"
VAR2 = "/a/path/chunk"
# ...
```

where `VAR1`, `VAR2` and so on are alphanumeric strings that begin with an alphabetic character or an underscore. Casing is free, and occurrences are case sensitive. Local variables can mention environment variables: mentioning an undefined variable will replace the occurrence with the empty string, thus mimicking the behaviour of UNIX shells. An environment variable can be mentioned in the form `${VAR_NAME}` both in local variables and in source and destination paths. Environment variable names are case sensitive on Unix based operating systems, and case insentitive on Windows.

> [!WARNING]
> Mentioning an undefined local variable results in a configuration error. Citing local variables in other local variables *is not allowed* as well, and raises a configuration error. In fact, even using the `%{VAR_NAME}` pattern, where `VAR_NAME` is an alphanumeric string, in a variable definition is reported as an error. Environment variables are not checked for citations: environment variables that include a `${VAR_NAME}` pattern in their contents *will have this pattern replaced* by **copyjob**. At the time, a maximum of 16 variable replacements (both local and environment) per path can be performed, and a configuration file that passes the limit is refused. This limit might eventually be removed in a future release.

Moving to the garbage bin (named *Recycle Bin*, *Trash* and in other ways on different desktop environments) is supported instead of both deleting and overwriting files, respectively setting the `trash_on_delete` and the `trash_on_overwrite` flags to `true` (`trash_on_delete` is `true` by default). Failure to recycle a file with the `trash_on...` flags set raises an error, and stop a job when the `halt_on_errors` parameter is set. When overwriting, a file is only moved to the garbage bin when it is supposed to be overwritten - thus not when other conditions (such as age or contents checking) fail.

A special mention is due for `remove_others_matching`: when set to `true`, the files in the destination folder that match `patterns_include`, and do not exist in the source, are *removed* from the destination. This still holds when copy operations from the source to the destination fail: the rationale behind this choice is, that a user that turns that particular parameter on would probably want to clean up the folders at the destination from unnecessary or obsolete files, even when there are versions of the source documents (for example newer) that cause the copy operation to fail. A corollary is that a file in the destination folder will not be deleted if it exists in the source, even if its copy operation is skipped. When no source file is matched, nothing is deleted at the destination.

Deletion takes place *after* all matching files have been copied: an error during the copy stage will abort *all* deletions when `halt_on_errors` is set.

> [!WARNING]
> The `remove_others_matching` flag, coupled with an undefined environment variable, can lead to dangerous results: a misspelled environment variable in a destination directory can cause unwanted file deletions if the user performing the job has enough rights. For example, if `HOEM` is not defined and a destination is defined as `"${HOEM}/share"`, the `/share` directory will be used instead.


### Configuration of jobs

Each job is introduced by the string `[[job]]` on a single line (the double square bracket is the TOML idiom for arrays of mappings), always followed at least by the mandatory parameters, namely `name`, `source`, `destination`, and `patterns_include`. Many parameters are the same listed above for global configuration. The detailed list follows:

| **Name**                 | **Default** | **Description**                                       |
|--------------------------|-------------|-------------------------------------------------------|
| `name`                   | *undefined* | **job name** (*string*)                               |
| `source`                 | *undefined* | **source directory** (*string*)                       |
| `destination`            | *undefined* | **destination directory** (*string*)                  |
| `patterns_include`       | *undefined* | **RE patterns of filenames to copy** (*string list*)  |
| `patterns_exclude`       | *empty*     | RE patterns of filenames to skip (*string list*)      |
| `patterns_exclude_dir`   | *empty*     | RE patterns of subdir names to skip (*string list*)   |
| `recursive`              | true        | walk subdirectories                                   |
| `overwrite`              | true        | overwrite existing files                              |
| `skip_newer`             | true        | skip if destination is not older than source          |
| `check_content`          | false       | check contents of files using a SHA-256 hash          |
| `follow_symlinks`        | false       | follow symbolic links                                 |
| `case_sensitive`         | true        | treat patterns/filenames as case sensitive            |
| `create_directories`     | true        | create directory structure if missing                 |
| `keep_structure`         | true        | keep subdirectory structure, otherwise flat           |
| `halt_on_errors`         | false       | halt operations upon first error in this job          |
| `trash_on_delete`        | true        | use garbage bin instead of deleting                   |
| `trash_on_overwrite`     | false       | use garbage bin before overwriting                    |
| `remove_others_matching` | false       | remove matching files at destination if not in source |

> [!NOTE]
> While it is possible to omit many parameters as said above, any *undefined* parameter in the configuration file will be considered an error, and cause aborting the operation before any job execution: the offending parameter is reported unless the output is suppressed.

The `patterns_exclude` and `patterns_exclude_dir` parameters are *optional*, and when specified will respectively skip files that match `patterns_include` but *also* do not match `patterns_exclude`, and skip subdirectories whose names match `patterns_exclude_dir` in case `recursive` is set to `true`. For instance, if all subdirectories of the source whose names begin with `ARCHIVE_` need to be skipped, the job will contain the following definition:

```toml
patterns_exclude_dir = [ 'ARCHIVE_.*' ]
```

since the `patterns_...` parameters only accept lists as their values. Note that each provided pattern matches against a *whole* file or directory name, not just a part of it. `patterns_exclude_dir` never matches against the source root.

> [!NOTE]
> Patterns and file names are treated case-insensitively on operating systems that consider paths not case sensitive.

All other (boolean) parameters are *optional*, and when omitted will carry their default value, or the value defined at global level if present.

Notice that **copyjob** is strict on job names format (for no actual reason), only accepting alphanumeric names that begin with a letter; job names can contain underscores. Both upper and lower case letters can be used, however job names are *always* case sensitive.


### Use of slashes in directories

Slashes are universally intended as path level separators. On UNIX-like systems, the forward slash is used and Windows normally uses the backslash. **copyjob** supports both forward and back slashes on Windows, while on UNIX-like systems only the forward slash is used. On Windows all the forward slashes are converted to backslashes in the output (and in the actual file operations).


### Special directory markers

In the `source` and `destination` directory specifications, two predefined markers can be used:

| **Marker** | **Meaning**                                                                               |
|------------|-------------------------------------------------------------------------------------------|
| `~/`       | the user home directory, shortcut for `${HOME}/` on Unix and `${USERPROFILE}/` on Windows |
| `@/`       | the directory where the configuration file is located                                     |

*at the beginning* of the path specification. So, if we run copyjob with */tmp/copyjob.toml* as an argument and the source directory specification is `@/some/path`, the actual source directory will be expanded to `/tmp/some/path`. These special markers can also be used in local variables, as long as they are mentioned at the beginning of directory specifications: markers in positions different from the beginning will be ignored.

On Windows, the slash can be replaced by a backslash. The slash (or backslash, on Windows) must *always* be included. Also, these markers are not expanded if they come from an environment variable.


## Known problems

There are some edge cases in which **copyjob** might behave in a different way than one could expect, especially when handling symbolic links. This is the reason why the `follow_symlinks` configuration parameter is set to *false* by default both at global and at job level, and the suggestion is to avoid following symlinks - unless the destination directory is *entirely* managed by **copyjob**. **copyjob**, in fact, will never create links in the destination folder, it just copies contents. When the destination directory contains links:

* if `follow_symlinks` is *true* and a link points to anything (especially a directory) outside the destination tree, the copy and delete operations will be performed at the actual, linked, destination - as long as the user invoking the command has the permissions to do this;
* if `follow_symlinks` is `trash_on_overwrite` are set, and a dangling link is found in the destination that is eligible for overwriting, the link itself is overwritten by the actual file and the link is sent to the recycle bin;
* with `follow_symlinks` set to *true*, deleting a symlinked file removes the link, while files inside a symlinked directory are deleted at the target.

In any case, if the destination is not managed exclusively by **copyjob**, the suggestion is to keep `follow_symlinks` unset, and to avoid using directories containing symbolic links as targets for the utility.

Also, note that if a flat destination layout is chosen (`keep_structure = false`) and the job is set to walk subdirectories (`recursive = true`) in the source, the result *might be unexpected* when a file with the same name is found multiple times in the tree: which file will be copied depends on the order in which the OS traverses subdirectories, and which one of the homonymous source files is older in case only newer files are set to be replicated.

Although this cannot be considered a problem, it is worth mentioning that **copyjob** will refuse a configuration file where, at least in a job, the source directory is contained in the destination, or the opposite case where the destination is a subdirectory of the source.


## Output of **copyjob**

**copyjob** can produce a quite verbose output when the logging level is set to either *debug* or *trace*. For each job, summary lines will be written to the console reporting the number of files to copy, the number of files that will deleted (that is, whose names in the destination directory match the source patterns, except for the ones coming from source), and at the end it will report the numbers of actually copied and/or deleted files. While executing the job, **copyjob** reports each attempted copy operation mentioning both the source and the destination files, as well as whether or not the copy operation was successful.

> [!NOTE]
> The default *warn* level will not log the single file operations (at least *info* is needed for this), thus, a successful run will log nothing at the default log level.

A compact JSON output can be produced, easier to parse for a possible frontend. The format of the JSON records is the following:

```json
{
  "header": {
    "time": "2026-11-14T21:43:26.643107",
    "application": "copyjob",
    "level": "<level>"
  },
  "contents": {
    "context": {
      "emitter": "<part>",
      "item": "<job name if any>",
      "action": "<action>",
      "source": "<source path>",
      "destination": "<destination path>"
    },
    "message_type": {
      "when": "<takes place at>",
      "status": "<success specification>"
    },
    "message": "a human readable message"
  }
}
```

where:

* `<level>` is the log level
* `<emitter>` says whether the action is at the job level or in the main loop,
* `<action>` is either *COPY* or *DELETE*
* `<takes place at>` is *START*, *END*, or *PROC* depending whether the record is issued at the beginning, at the end, or in the middle of something
* `<success specification>` can be *OK* for success, *ERR* or *FAIL* for failures, *IND* for undetermined results, *MSG* for informational messages.

The entries might not be in this order in actual logs, and all records are output as single lines.


## A possible use scenario

The way I use **copyjob** is, for some of the subdirectories in a certain tree, to perform some jobs which are configured on a per-directory basis. In order to do this, I simply create a dedicated *copyjob_XXXX.toml* file in each directory where I want **copyjob** to operate on, and then invoke the utility on each of these files: thanks to the `@/` marker it is possible to define jobs that are applied to the directory where the configuration file is found, in a generic way that allows to reuse most of the configuration when the actions to perform are similar. **copyjob** is called on each of these files using the `find` command on Unix and the `Get-ChildItem . -Recurse` Powershell cmdlet on Windows, as follows:

* **on Unix**

  ```shell
  find . -name "copyjob_*.toml"  -exec copyjob {} \;
  ```

* **on Windows**

  ```powershell
  Get-ChildItem . -Recurse -Filter "copyjob_*.toml" | % { copyjob "$_.FullName" }
  ```

You might want to use the `-a` flag to use a single, specific log file, especially if these actions are automated.


## Use of LLMs and AI in this project

I am using this little project to experiment with LLM-based code review: no code has been actually written or explicitly proposed by a LLM. LLMs (mostly Anthropic's *Claude Code*) just reviewed the codebase, catching issues except the ones that they have been told to leave alone, either because I didn't consider them erratic behaviours or because I preferred to defer a solution, or a choice on how to handle them.


## License

This utility is released under the GNU LGPL v3. It is free to use and to modify for everyone, as long as the modified source is made available under the same license terms.

See the included LICENSE.txt file for details.
