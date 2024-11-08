use std::path::Path;

use async_std::process::Command;

/// Dumps the database to the specified file using `pg_dump`. This means that
/// `pg_dump` must be installed on the system running this command.
pub async fn dump_db<P: AsRef<Path>>(
    url: &str,
    file: P,
    schema_only: bool,
) -> std::io::Result<()> {
    let status = if schema_only {
        Command::new("pg_dump")
        .arg(url)
        .arg("--schema-only")
        .arg("--no-owner")
        .arg("--no-privileges")
        .arg("-f")
        .arg(file.as_ref())
        .status()
        .await?
    } else {
        Command::new("pg_dump")
        .arg(url)
        .arg("--rows-per-insert=1000")
        .arg("--no-owner")
        .arg("--column-inserts")
        .arg("--no-privileges")
        .arg("-f")
        .arg(file.as_ref())
        .status()
        .await?
    };

    if status.success() {
        println!("Database dump successful!");
    } else {
        eprintln!("Database dump failed with status: {:?}", status.code());
    }
    Ok(())
}
