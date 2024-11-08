use std::fs::File;
use std::path::Path;

use std::io::{self, BufRead, BufReader, Lines, Write};
use std::process::{Command, Stdio};

use crate::error::DbToolsError;

/// Dumps the database to the specified file using `pg_dump`. This means that
/// `pg_dump` must be installed on the system running this command.
pub fn dump_db<P: AsRef<Path>>(
    url: &str,
    file: P,
    with_data: bool,
) -> Result<(), DbToolsError> {
    let mut child = if !with_data {
        Command::new("pg_dump")
        .arg(url)
        .arg("--schema-only")
        .arg("--no-owner")
        .arg("--no-privileges")
        .stdout(Stdio::piped())
        .spawn()?
    } else {
        Command::new("pg_dump")
        .arg(url)
        .arg("--rows-per-insert=1000")
        .arg("--no-owner")
        .arg("--column-inserts")
        .arg("--no-privileges")
        .stdout(Stdio::piped())
        .spawn()?
    };

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut outfile = File::create(file.as_ref())?;
        // This is set to true to prevent a series of blank lines in the file.
        // It starts off true to remove any blanks in the leadup to the first
        // kept line.
        let mut consecutive_blank_line = true;
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // check for any `SET` or `--` (comment) lines and skip them when outputting
            if line.starts_with("SET") || line.starts_with("--") {
                continue;
            }
            // Remove any long blocks of blank lines in the file
            if line.is_empty() && consecutive_blank_line {
                continue;
            } else if line.is_empty() {
                consecutive_blank_line = true;
            } else if consecutive_blank_line {
                consecutive_blank_line = false;
            }

            writeln!(outfile, "{}", line)?;
        }
    }

    let status = child.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(DbToolsError::ProcessStatus("pg_dump".to_owned(), status.code()))
    }
}

/// Checks a line to see whether or not it should be output to the dump file. Strips
/// out any comments and `SET` commands, since the `SET` commands can cause issues
/// loading the files later based on transaction timeouts and similar configuration
/// options that aren't really necessary as part of the schema dump.
///
/// Returns `true` if the line should be kept, `false` otherwise.
fn keep_dump_line(line: &str) -> bool {
    !(line.starts_with("SET") || line.starts_with("--"))
}
