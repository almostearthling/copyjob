# **copyjob**


A simple utility to perform complex copy operations according to directives
provided in a TOML configuration file. Each configuration file can contain
instructions for several copy operations, and the parameters may vary for
each operation defined in the configuration: such parameters can be defined
globally in the TOML file and overridden partially or totally in each job.

Jobs are related to a source directory and a destination: it is possible to
create the destination path if it doesn't exist, and to recreate totally or
partially the source structure at the destination. Jobs can be instructed to
selectively copy files, according to criteria specified in the configuration:
such criteria include file name patterns for both inclusion and exclusion,
checking file age and/or contents compared to a possibly existing destination
file, and subdirectories to skip. A job can also remove files existing in the
destination directory if they match the naming criteria given for the source
and are not present in the source directory.

The file name patterns are specified as *regular expressions* in the TOML
configuration file, as this gives more granularity while specifying what has
to be chosen and what to skip, compared to the wildcard semantics found in
most shells. Patterns are provided as lists of regular expression strings,
so that matching any of the list items is considered a good match for the
given condition.

For a description of TOML as a configuration file language, see
[here](https://toml.io/),
and for a description of regular expression syntax and semantics, see
[Wikipedia](https://en.wikipedia.org/wiki/Regular_expression);
there are many useful tutorial around about *RE*s, and also many syntax
checkers and interactive tools to test patterns before using them in real
life cases such as **copyjob** jobs.


## Disclaimer

This software is in early development stage. It works as expected in the use
cases that I have been able to experiment, but it can behave in unexpected
ways in cases not covered by my own tests. It should not destroy any contents
in the source directories, however files in destination directories may be
overwritten or deleted without notice: use it with caution, possibly after a
backup.


## Usage

**copyjob** can be invoked from the command line. By typing `copyjob --help` at
the prompt, the utility will display a brief usage message:

```text
Usage: copyjob [OPTIONS] <CONFIG>

Arguments:
  <CONFIG>  path to configuration file

Options:
  -q, --quiet            Suppress all output
  -p, --parsable-output  Generate machine readable output (JSON)
  -h, --help             Print help
  -V, --version          Print version
```

The command called with `--quiet` or `-q` as parameter, followed by the
configuration file path, will only exit with an *error* value in case of
unrecoverable errors, and when invoked with the `--parsable-output` or `-p`
parameter will produce output in JSON format, that would be easier for another
program to parse, although more difficult for a human to read. The basic
invocation is

```sh
copyjob path/to/config.toml
```

which will read the file `path/to/config.toml` and perform the jobs that the
user defined and activated there, producing a readable (yet messy) output.


## An example of configuration file

In order to understand what **copyjob** can do, let's provide a simple example
of a working configuration file:

```toml
# sample configuration file

# jobs to actually perform
active_jobs = [
    "Reports",
#    "CurrentSummary",
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

This very simple configuration file defines two different jobs that **copyjob**
will be able to perform:

1. *Reports*: will copy Word&trade;, Excel&trade; and PDF files whose names
   begin with the word *Report* followed by an underscore and any other
   characters: the job will copy both old and new Word and Excel files (the
   *RE* states that the *x* at the end is optional); this job walks through
   subdirectories, but any matching files found in the subdirectories will be
   copied in the destination folder without recreating the source directory
   structure,
2. *CurrentSummary*: will copy all the PowerPoint&trade; presentations,
   created with either older or recent editions of PowerPoint, whose file name
   is *Current Summary* followed by a space and a version number with no dots
   in it; this job will ignore all subdirectories of the source folder.

Both jobs start searching in the *Documents/MyData* subfolder of the user home:
this is achieved by using the `HOME` environment variable (it only works on
UNIX-like systems, unless `HOME` is properly defined); also, the destination
is in both cases a subfolder of the *CloudSync/SyncedDocs* folder that might
or might not exist in the same user home directory. Because of the default
configuration of **copyjob**, the appropriate destination folders are created
by the utility itself in case they are not found.

At the global level we instruct **copyjob** to treat file names as case
insensitive, to make sure we don't miss files that could have be named
slightly differently on operating systems such as Windows. Also, we define
a couple of placeholders to use in source and destination directories in
order to easily change the configuration of all jobs simultaneously in
case the source or the target changes: for example, if the destination
directory for all jobs changes to */cloud/Synced*, it's easy to modify the
local variable `DEST_BASE` to `"/cloud/Synced"`, so that all the defined
jobs that mention this variable (in this case, both) change accordingly.

Although the configuration file defines two different jobs, in this case
only one of them will be performed, that is *Reports*: this happens
because the *CurrentSummary* job is commented out in the `active_jobs`
list. This is useful when some jobs have to be temporarily excluded
especially in highly automated tasks.

Also note the system variable `HOME` in the paths (or better, in this case,
in the local variables that are used in the source and target paths) that
will be replaced on UNIX-like systems by the actual user home directory
(we will see that this restriction can be overridden on Windows without
having to explicitly define `HOME`).

A template configuration file is provided, *copyjob_template.toml*.


## Configuration at the global level

Most parameters can be defined at the global level in the configuration file,
and the values defined here are shared by all jobs. On the other hand, jobs
can override such values partly or even totally - except for the list of active
jobs and the local variables, which can only be defined at the global level.

Overridable parameters are the following, and when omitted the corresponding
default value will be used:

| **Name**                 | **Default** | **Description**                                         |
|--------------------------|-------------|---------------------------------------------------------|
| `recursive`              | true        | walk subdirectories                                     |
| `overwrite`              | true        | overwrite existing files                                |
| `skip_newer`             | true        | skip if destination is more recent                      |
| `check_content`          | false       | check contents of files using MD5 hash                  |
| `follow_symlinks`        | true        | follow symbolic links                                   |
| `case_sensitive`         | true        | treat patterns/filenames as case sensitive              |
| `create_directories`     | true        | create directory structure if missing                   |
| `keep_structure`         | true        | keep subdirectory structure, otherwise flat             |
| `halt_on_errors`         | false       | halt each job upon first error (unless overridden)      |
| `trash_on_delete`        | false       | use garbage bin instead of deleting (unless overridden) |
| `trash_on_overwrite`     | false       | use garbage bin before overwriting (unless overridden)  |
| `remove_others_matching` | false       | remove matching files at destination if not in source   |

As said, a list of active jobs has to be defined, otherwise no job will be
performed (although **copyjob** will not issue an error). This is done by
defining the list:

```toml
active_jobs = [ "name1", "name2", ... ]
```

where the jobs *name1*, *name2*, and so on *must* be defined, otherwise the
command will exit with an error. Thanks to TOML flexible syntax, the array can
span multiple lines - which is useful for commenting out jobs when not needed.

The optional `[variables]` section can be used to define local variables:
their string values replace occurrences of the pattern `%{VAR_NAME}` in source
and destination paths (but *not* in regular expression patterns), so that the
string `%{VAR_NAME}/path` becomes `/some/path` if `VAR_NAME` is assigned the
value `/some`. The syntax is the following:

```toml
[variables]
VAR1 = "some value"
VAR2 = "/a/path/chunk"
...
```

where `VAR1`, `VAR2` and so on are alphanumeric strings that begin with an
alphabetic character. Casing is free, and occurrences are case sensitive.
Local variables can mention environment variables: mentioning an undefined
variable will replace the occurrence with the empty string, thus mimicking the
behaviour of UNIX shells. An environment variable can be mentioned in the form
`${VAR_NAME}` both in local variables and in source and destination paths.

Notice that, while it's possible to omit many parameters as said above, any
unknown parameter in the configuration file will be considered an error, and
cause the abortion of the operation before any job execution: the offending
parameter is reported unless the output is suppressed.

Moving to the garbage bin (named *Recycle Bin*, *Trash* and in other ways on
different desktop environments) is supported instead of both deleting files
and also overwriting, respectively setting the `trash_on_delete` flag and the
`trash_on_overwrite` flag to `true` (`trash_on_delete` is `true` by default).
Recycling instead of removing or overwriting is actually *attempted*, and if
it fails the destination is respectively deleted or overwritten anyway if the
respective options are turned on. When overwriting, a file is only moved to
the garbage bin when it is supposed to be overwritten - thus not when other
conditions (such as age or contents checking) fail.

A special mention is due for `remove_others_matching`: when set to `true`, the
files that match the job *RE* specifications and do not exist in the source
directories are *removed* on the destination directory. This still yields when
copy operations from the source to the destination do not succeed for any
reason. The rationale behind this choice is, that an user that turns that
particular parameter on would probably want to clean up the folders at the
destination from unnecessary files, even when there are versions of the source
documents (for example newer) that cause the copy operation to fail.

Also, note that if a flat destination is chosen (`keep_structure = false`) and
the job is set to walk subdirectories (`recursive = true`), the result might
be unexpected when a file with the same name is found in the main directory
and/or in subdirectories: which file will be copied depends on the order in
which the OS traverses subdirectories, and which one of the homonymous source
files is older in case only newer files are set to be replicated.


## Configuration of jobs

Each job is introduced by the string `[[job]]` on a single line (the double
square bracket is the TOML idiom for arrays of mappings), always followed at
least by the mandatory parameters, namely `name`, `source`, `destination`, and
`patterns_include`. Many parameters are the same listed above for global
configuration. The detailed list follows:

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
| `skip_newer`             | true        | skip if destination is more recent                    |
| `check_content`          | false       | check contents of files using MD5 hash                |
| `follow_symlinks`        | true        | follow symbolic links                                 |
| `case_sensitive`         | true        | treat patterns/filenames as case sensitive            |
| `create_directories`     | true        | create directory structure if missing                 |
| `keep_structure`         | true        | keep subdirectory structure, otherwise flat           |
| `halt_on_errors`         | false       | halt operations upon first error in this job          |
| `trash_on_delete`        | false       | use garbage bin instead of deleting                   |
| `trash_on_overwrite`     | false       | use garbage bin before overwriting                    |
| `remove_others_matching` | false       | remove matching files at destination if not in source |

The `patterns_exclude` and `patterns_exclude_dir` parameters are *optional*,
and when specified will respectively skip files that match `patterns_include`
but *also* match `patterns_exclude`, and skip subdirectories whose names match
`patterns_exclude_dir` in case `recursive` is set to `true`. For instance, if
all subdirectories whose names begin with *ARCHIVE_* have to be skipped, the
job will contain the following definition:

```toml
patterns_exclude_dir = [ 'ARCHIVE_.*' ]
```

since the *pattern_** parameters only accept lists as their values.

All other (boolean) parameters are *optional*, and when omitted will carry
their default value, or the value defined at global level if present.

Notice that **copyjob** is strict on job names format (for no actual reason),
only accepting alphanumeric names that begin with a letter; job names can
contain underscores. Both upper and lower case letters can be used, however
job names are *always* case sensitive.


### Use of slashes in directories

Slashes are universally intended as path level separators. On UNIX-like
systems, the forward slash is used and Windows normally uses the backslash.
**copyjob** supports both forward and back slashes on Windows, while on
UNIX-like systems only the forward slash is accepted. On Windows all the
forward slashes are converted to backslashes in the output (and in the actual
file operations).


### Special directory markers

In the `source` and `destination` directory specifications, two predefined
markers can be used:

| **Marker** | **Meaning**                                           |
|------------|-------------------------------------------------------|
| `~/`       | the user home directory                               |
| `@/`       | the directory where the configuration file is located |

*at the beginning* of the path specification. So, if we run copyjob with
*/tmp/copyjob.toml* as an argument and the source directory specification is
`@/some/path`, the actual source directory will be expanded to
`/tmp/some/path`. These special markers can also be used in local variables,
as long as the local variables are mentioned at the beginning of directory
specifications: markers in positions different from the beginning will be
ignored.

On Windows the slash can be replaced by a backslash.


## Output of **copyjob**

**copyjob** produces a quite verbose output, that can be used for logging. For
each job, summary lines will be written to the console reporting the number
of files to copy, the number of files that might have to be deleted (that is,
whose names in the destination directory match the source patterns), and at the
end it will report the numbers of actually copied and/or deleted files. While
executing the job, **copyjob** reports each attempted copy operation mentioning
the full paths of the source and the destination files, as well as whether or
not the copy operation was successful.

The output can be somewhat confusing, and it might be useful to redirect it
to a file in complex or long tasks. Errors are written to *stderr*, so it is
probably appropriate to redirect *stderr* to *stdout* for logging.

A compact JSON output can be produced, easier to parse for other programs:
although **copyjob** is suitable to be used directly from the CLI, it can be
wrapped into a more user-friendly tool, be it still CLI oriented or providing
a GUI.


## Why **copyjob**?

I actually *needed* this tool. There are some alternatives, such as
[rsync](https://rsync.samba.org/),
[unison](https://www.cis.upenn.edu/~bcpierce/unison/),
and similar utilities. However, these programs tend to be bare in terms
of defining what has to be replicated (mostly defining a source, a target,
and possibly some simple criteria to specify what to replicate), and tend
on the other hand to focus on network capabilities, efficiency and bandwidth
optimization. What I needed was something that could easily copy some
documents from one directory to another, while performing even cumbersome
selections of files in the directory of origin. I needed no network support:
the common synchronization utilities (as [Nextcloud](https://nextcloud.com/))
would do this for me. I need to share documents with teams, these documents
are in fact produced by different people, that use different naming
strategies: of course, a better practice would be to change file names
according to some logical pattern, but this is not always possible, mostly
because these files tend to propagate via other means (e-mail) than common
repositories - and, in such cases, different names might be interpreted as
different documents or at least as different versions of the same document.
Thus I decided that the best strategy was to use and distribute documents
preserving their original names.

I keep these files in a directory structure that can optimistically be
defined as an awful mess, but which, in the end, is the one that I now
understand and can cope with. However, I cannot share these directories
with my collaborators, both because they have a structure that doesn't
make much sense if you don't see the whole thing and because the folders
contain much more files than the ones that have to be accessible to the
team. So I decided to develop this small utility. Now, for each directory
containing files that I need to share, I write a simple *copyjob.toml*
file. A script in the main document folder finds them recursively and
executes **copyjob** on each of them:

```sh
#!/bin/sh
find . -name 'copyjob.toml' -exec copyjob {} \;
```

It took me an afternoon to write it in Python, and another day to fine-tune
the Python version in order to have a prettier output and all the needed
parameters and features. But since I wanted to experiment Rust, I decided
that such a simple tool could have been suitable for my first Rust project,
and so it was. It took me *weeks*. I'm still working on a CLI wrapper for
**copyjob**, this time in Python, to cover logging and console output.

I still find Rust difficult, sometimes "byzantine", but **copyjob** helps me
to get acquainted with it and I'm slowly getting used to the Rust philosophy.
And, as a side effect, I keep the folders that I share with my colleagues
clean and tidy.

Nowadays the core (namely, the resident part) of my legacy utility dubbed
[When](https://github.com/almostearthling/when-command) has being completely
reworked in Rust, as [whenever](https://github.com/almostearthling/whenever):
in this moment the original project is temporarily quiescent, especially
because I'm not using Linux as my main desktop operating system now, but also
because it is quite difficult to follow the continuous changes in the *DBus*
messages, services and methods in the versions of Ubuntu that followed the
18.04 release. The next version of *When* will be platform independent, and
its core is already working with the same features (more than the ones that
*When* used to have) on both Windows and Linux. Of course, even with a
development version, I am already able to use **copyjob** in *whenever* jobs.

There is an entire paragraph in this document, named *Disclaimer*, that
basically states that anyone should use this small utility at their own risk.
That said, while it's true that **copyjob** can *permanently delete* files at
the destination directory (if instructed to do so), one should also consider
what to use **copyjob** for: it's designed to automate replication tasks in a
way that attempts to keep the destination directory structure consistent
through consecutive updates. Normally there shouldn't be "unique" documents in
the destination folder, but only copies managed by **copyjob** itself. In this
case it's quite safe to use it, as source documents are *never* deleted or
modified.

Also, this is a *quick-and-dirty* utility (at least for now): it focuses on
getting the job(s) done considering that files are replicated from one folder
to another *on a personal computer*: no checks are performed on concurrency
issues, and possible errors during copy only depend on OS issues such as user
rights on destination directories and files.

I actually use **copyjob** on a daily basis, and it sometimes unintentionally
refused to overwrite or delete files in the destination folder - in its really
early editions - but it never erroneously lost a file.


## License

This utility is released under the GNU LGPL v3. It is free to use and to modify
for everyone, as long as the modified source is made available under the same
license terms.

See the included LICENSE.txt file for details.
