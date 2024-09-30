use std::{io::{Error, ErrorKind}, path::{Path, PathBuf}};

/// An individual migration that can be performed on the database
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct MigrationVersion {
    version: String,
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

    pub fn file_name(&self, direction: MigrationDirection) -> String {
        format!("{version}.{dir}.sql",
            version = self.version,
            dir = if direction == MigrationDirection::Dn {
                "dn"
            } else {
                "up"
            }
        )
    }

    /// Attempts to locate the migration file. If it exists, then it will return the
    /// path to the file, and if it does not then will return None.
    pub fn migration_file<P>(
        &self,
        direction: MigrationDirection,
        folder: P,
    ) -> Option<PathBuf>
    where P: AsRef<Path> {
        let folder_path = folder.as_ref();
        let file_name = self.file_name(direction);
        let full_path = folder_path.join(file_name);

        if full_path.exists() {
            Some(full_path)
        } else {
            None
        }
    }
}


/// Represents a version being requested
pub enum TargetVersion {
    /// Represents the final value in the migration path
    Head,
    /// Represents the database at the end of the very first migration
    First,
    /// A version identified by name and an optional offset from that version
    Name((String, i32)),
}

/// The direction of a migration sequence
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum MigrationDirection {
    Up,
    Eq,
    Dn,
}

/// A specific sequence of migration versions that must be stepped through in order to complete
/// a requested migration.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct MigrationPath {
    versions: Vec<MigrationVersion>,
    direction: MigrationDirection,
}

pub trait SearchVersion {
    fn search_version(self: &Self, target: TargetVersion) -> Option<&MigrationVersion>;
}

impl SearchVersion for Vec<MigrationVersion> {
    fn search_version(self: &Self, target: TargetVersion) -> Option<&MigrationVersion> {
        match target {
            TargetVersion::Head => self.last(),
            TargetVersion::First => self.first(),
            TargetVersion::Name((search_name, offset)) => {
                let found_idx = self.iter().position(|vers| vers.version.contains(&search_name))?;
                let offset_idx = (found_idx as isize) + (offset as isize);
                return if offset_idx >= 0 {
                    self.get(offset_idx as usize)
                } else {
                    None
                }
            },
        }
    }
}

impl MigrationPath {
    pub fn new_from_folder<P>(
        p: P,
        current: Option<MigrationVersion>,
        target: TargetVersion,
    ) -> std::io::Result<MigrationPath>
    where P: AsRef<Path>,
    {
        let mut version_path = Vec::new();

        let all_versions = MigrationVersion::load_migration_folder(p)?;
        let target_version = all_versions.search_version(target);

        // Exit early if we don't have a target
        let target_version = target_version
            .ok_or_else(
                || Error::new(ErrorKind::NotFound, "Cannot find requested target version")
            )?;

        let direction = match current {
            Some(ref current_version) => {
                if current_version < target_version {
                    MigrationDirection::Up
                } else if current_version > target_version {
                    MigrationDirection::Dn
                } else {
                    MigrationDirection::Eq
                }
            },
            None => MigrationDirection::Up,
        };

        let low = match current {
            Some(ref current_version) => {
                if current_version <= target_version {
                    Some(current_version)
                } else {
                    Some(target_version)
                }
            },
            None => None,
        };
        let high = match current {
            Some(ref current_version) => {
                if current_version <= target_version {
                    target_version
                } else {
                    &current_version
                }
            },
            None => target_version,
        };

        all_versions.iter().for_each(|val| {
            match low {
                Some(low_version) => {
                    if val > low_version && val <= high {
                        version_path.push(val.to_owned());
                    }
                },
                None => {
                    if val <= high {
                        version_path.push(val.to_owned());
                    }
                },
            }
        });

        if direction == MigrationDirection::Dn {
            version_path.reverse();
        }

        Ok(Self { versions: version_path, direction })
    }

    pub fn migration_files<P>(&self, folder: P) -> Vec<Option<PathBuf>>
    where P: AsRef<Path> {
        let folder_path = folder.as_ref();
        self.versions.iter()
            .map(|ver| ver.migration_file(self.direction, folder_path))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_FOLDER: &str = "../../tests/migrations";

    #[test]
    fn test_get_version_files() {
        assert_eq!(
            MigrationVersion::load_migration_folder(TEST_FOLDER).unwrap(),
            vec![
                MigrationVersion { version: "00-create-version-table".into() },
                MigrationVersion { version: "01-create-user-table".into() },
                MigrationVersion { version: "02-update-user-table".into() },
                MigrationVersion { version: "03-clear-password".into() },
                MigrationVersion { version: "04-remove-password".into() },
            ],
        );
    }

    #[test]
    fn test_search_version() {
        // Head
        assert_eq!(
            MigrationVersion::load_migration_folder(TEST_FOLDER)
                .unwrap()
                .search_version(TargetVersion::Head)
                .unwrap(),
            &MigrationVersion { version: "04-remove-password".into() },
        );
        // First
        assert_eq!(
            MigrationVersion::load_migration_folder(TEST_FOLDER)
                .unwrap()
                .search_version(TargetVersion::First)
                .unwrap(),
            &MigrationVersion { version: "00-create-version-table".into() },
        );
        // Name
        assert_eq!(
            MigrationVersion::load_migration_folder(TEST_FOLDER)
                .unwrap()
                .search_version(TargetVersion::Name(("update-user".into(), 0)))
                .unwrap(),
            &MigrationVersion { version: "02-update-user-table".into() },
        );
    }

    #[test]
    fn test_get_migration_path() {
        // Upgrade from None
        assert_eq!(
            MigrationPath::new_from_folder(
                TEST_FOLDER,
                None,
                TargetVersion::Head,
            ).unwrap(),
            MigrationPath {
                versions: vec![
                    MigrationVersion { version: "00-create-version-table".into() },
                    MigrationVersion { version: "01-create-user-table".into() },
                    MigrationVersion { version: "02-update-user-table".into() },
                    MigrationVersion { version: "03-clear-password".into() },
                    MigrationVersion { version: "04-remove-password".into() },
                ],
                direction: MigrationDirection::Up,
            }
        );

        // Upgrade from a middle version
        assert_eq!(
            MigrationPath::new_from_folder(
                TEST_FOLDER,
                Some(MigrationVersion { version: "01-create-user-table".into() }),
                TargetVersion::Name(("03-clear-password".into(), 0)),
            ).unwrap(),
            MigrationPath {
                versions: vec![
                    MigrationVersion { version: "02-update-user-table".into() },
                    MigrationVersion { version: "03-clear-password".into() },
                ],
                direction: MigrationDirection::Up,
            }
        );

        // Downgrade to first
        assert_eq!(
            MigrationPath::new_from_folder(
                TEST_FOLDER,
                Some(MigrationVersion { version: "04-remove-password".into() }),
                TargetVersion::First,
            ).unwrap(),
            MigrationPath {
                versions: vec![
                    MigrationVersion { version: "04-remove-password".into() },
                    MigrationVersion { version: "03-clear-password".into() },
                    MigrationVersion { version: "02-update-user-table".into() },
                    MigrationVersion { version: "01-create-user-table".into() },
                ],
                direction: MigrationDirection::Dn,
            }
        );

        // Target is current
        assert_eq!(
            MigrationPath::new_from_folder(
                TEST_FOLDER,
                Some(MigrationVersion { version: "02-update-user-table".into() }),
                TargetVersion::Name(("04-remove-password".into(), -2)),
            ).unwrap(),
            MigrationPath {
                versions: vec![],
                direction: MigrationDirection::Eq,
            }
        );
    }

    #[test]
    fn test_get_migration_version_file_name() {
        assert_eq!(
            MigrationVersion { version: "00-create-version-table".into() }.file_name(MigrationDirection::Dn),
            "00-create-version-table.dn.sql",
        );
    }

    #[test]
    fn test_get_migration_path_files() {
        assert_eq!(
            MigrationPath::new_from_folder(
                TEST_FOLDER,
                None,
                TargetVersion::Head,
            )
                .unwrap()
                .migration_files(TEST_FOLDER),
            vec![
                Some("../../tests/migrations/00-create-version-table.up.sql".into()),
                Some("../../tests/migrations/01-create-user-table.up.sql".into()),
                Some("../../tests/migrations/02-update-user-table.up.sql".into()),
                Some("../../tests/migrations/03-clear-password.up.sql".into()),
                Some("../../tests/migrations/04-remove-password.up.sql".into()),
            ],
        );

        assert_eq!(
            MigrationPath::new_from_folder(
                TEST_FOLDER,
                Some(MigrationVersion { version: "04-remove-password".into() }),
                TargetVersion::First,
            )
                .unwrap()
                .migration_files(TEST_FOLDER),
            vec![
                Some("../../tests/migrations/04-remove-password.dn.sql".into()),
                None,
                Some("../../tests/migrations/02-update-user-table.dn.sql".into()),
                Some("../../tests/migrations/01-create-user-table.dn.sql".into()),
            ],
        );
    }
}
