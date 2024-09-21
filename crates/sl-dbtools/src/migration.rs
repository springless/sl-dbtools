use std::path::Path;

/// An individual migration that can be performed on the database
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct MigrationVersion {
    version: String,
}

/// Represents a version being requested by the user
pub enum TargetVersion {
    /// Represents the final value in the migration path
    Head,
    /// Represents the database at the end of the very first migration
    First,
    /// Represents adding or subtracting a specific number of migrations from the current version
    Offset(i32),
    /// A version identified by name
    Name(String),
}

impl MigrationVersion {
    /// Generates the list of migrations to apply based on the folder supplied, a target version,
    /// and the current version.
    pub fn load_migration_folder<P>(p: P) -> std::io::Result<Vec<MigrationVersion>>
        where P: AsRef<Path> {
        let mut versions = Vec::new();

        for entry in std::fs::read_dir(p)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(fname) = path.file_name() {
                    if let Some(fname_str) = fname.to_str() {
                        if fname_str.ends_with(".up.sql") {
                            if let Some(vname) = fname_str.strip_suffix(".up.sql") {
                                versions.push(vname.to_string());
                            }
                        }
                    }
                }
            }
        }
        versions.sort();
        let versions = versions.into_iter().map(|ver| MigrationVersion { version: ver }).collect();
        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version_files() {
        assert_eq!(
            MigrationVersion::load_migration_folder("../../tests/migrations").unwrap(),
            vec![
                MigrationVersion { version: "00-create-version-table".into() },
                MigrationVersion { version: "01-create-user-table".into() },
                MigrationVersion { version: "02-update-user-table".into() },
                MigrationVersion { version: "03-clear-password".into() },
                MigrationVersion { version: "04-remove-password".into() },
            ],
        );
    }
}
