/// Utility functions for dumping data from the datbase

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use crate::url::DbUrl;
use crate::error::DbToolsError;


#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum DumpType {
    /// Schema and data
    All,
    DataOnly,
    SchemaOnly,
}

/// Dumps the database to the specified file using `pg_dump`. This means that
/// `pg_dump` must be installed on the system running this command.
pub fn dump_db<W: Write>(
    url: &DbUrl,
    writer: &mut W,
    dump_type: &DumpType,
    schemas: &Option<Vec<String>>,
) -> Result<(), DbToolsError> {
    let mut cmd = Command::new("pg_dump");
    let cmd = cmd
        .arg(url.to_string())
        .arg("--no-owner")
        .arg("--no-privileges");

    let cmd = if &DumpType::All == dump_type || &DumpType::DataOnly == dump_type {
        cmd
            .arg("--rows-per-insert=1000")
            .arg("--column-inserts")
    } else { cmd };

    let cmd = if &DumpType::DataOnly == dump_type {
        cmd
            .arg("--data-only")
    } else if &DumpType::SchemaOnly == dump_type {
        cmd
            .arg("--schema-only")
    } else {
        cmd
    };

    let cmd = if let Some(schemas) = schemas {
        schemas.iter().fold(cmd, |acc, schema| {
            acc
                .arg("--schema")
                .arg(schema)
        })
    } else { cmd };

    let mut child = cmd.stdout(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        // This is set to true to prevent a series of blank lines in the file.
        // It starts off true to remove any blanks in the leadup to the first
        // kept line.
        let mut consecutive_blank_line = true;
        for line in reader.lines() {
            let line = line?;
            let is_blank = line.trim().is_empty();

            // check for any `SET` or `--` (comment) lines and skip them when outputting
            if !keep_dump_line(&line) {
                continue;
            }
            // Remove any long blocks of blank lines in the file
            if is_blank && consecutive_blank_line {
                continue;
            } else if is_blank {
                consecutive_blank_line = true;
            } else if consecutive_blank_line {
                consecutive_blank_line = false;
            }

            writeln!(writer, "{}", line)?;
        }
    }

    let status = child.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(DbToolsError::ProcessStatus("pg_dump".to_owned(), status.code()))
    }
}

/// Returns `true` if the line should be kept, `false` otherwise.
///
/// Checks a line to see whether or not it should be output to the dump file. Strips
/// out any comments and most `SET` commands, since some of the timing and lock settings
/// can cause problems when re-loading a file in different contexts. Other `SET` commands
/// such as `SET check_function_bodies` DO need to be kept in order to ensure the file
/// can still be reloaded correctly later.
fn keep_dump_line(line: &str) -> bool {
    !(
    line.starts_with("--")
    || (
    line.starts_with("SET")
    && !(
    // keep these `SET` statements to ensure loading the file back
    // remains more or less the same
    line.starts_with("SET client_encoding")
    || line.starts_with("SET standard_conforming_strings")
    || line.starts_with("SET xmloption")
    || line.starts_with("SET check_function_bodies")
)
)
)
}
