use std::path::{Path, PathBuf};

use crate::error::DbToolsError;
use super::version::{SchemaVersion, TargetVersion};
use super::step::{Direction, MigrationAction, MigrationPath, MigrationStep};


/// The current status of a database within the migration system
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct MigrationPlanner {
    pub versions: Vec<SchemaVersion>,
    /// Position of the current version in the versions path
    current_pos: usize,
    folder: PathBuf,
}

trait FindVersion {
    /// Searches for a version that matches the provided target version string. And returns
    /// the fully specified MigrationVersion if found.
    fn search_version(self: &Self, target: &str) -> Option<&SchemaVersion>;
    /// Searches for a version that matches the provided target version string and returns
    /// the position of that version.
    fn search_version_position(self: &Self, target: &str) -> Option<usize>;
    /// Finds the position of an absolute version, if it exists
    fn find_version_position(self: &Self, version: &SchemaVersion) -> Option<usize>;
}

impl FindVersion for Vec<SchemaVersion> {
    fn search_version_position(self: &Self, target: &str) -> Option<usize> {
        self.iter().position(|item| {
            match item {
                SchemaVersion::Version(this_vers) => {
                    target.to_lowercase().contains(&this_vers.to_lowercase())
                },
                _ => false,
            }
        })
    }

    fn search_version(self: &Self, target: &str) -> Option<&SchemaVersion> {
        Some(&self[self.search_version_position(target)?])
    }

    fn find_version_position(self: &Self, version: &SchemaVersion) -> Option<usize> {
        self.iter().position(|item| item == version)
    }
}

impl MigrationPlanner {
    pub fn new_from_folder<P>(
        p: P,
        current: SchemaVersion,
    ) -> Result<Self, DbToolsError>
    where P: AsRef<Path>,
    {
        let versions = SchemaVersion::load_migration_folder(&p)?;
        let current_pos = versions.find_version_position(&current)
            .ok_or_else(|| DbToolsError::VersionDoesNotExist(current.clone()))?;

        Ok(MigrationPlanner {
            versions,
            current_pos,
            folder: p.as_ref().to_owned(),
        })
    }

    /// Returns the position of the target version if it can be found in the version list
    fn get_target_position(&self, target: &TargetVersion) -> Option<usize> {
        let num_versions = self.versions.len();
        let starting_index = match &target {
            TargetVersion::Root(_) => 0,
            TargetVersion::Head(_) => num_versions - 1,
            TargetVersion::Current(_) => self.current_pos,
            TargetVersion::Search((target_name, _)) => {
                let search_lower = target_name.to_lowercase();
                self.versions.iter().position(|item| {
                    match item {
                        SchemaVersion::Version(this_vers) => {
                            this_vers.to_lowercase().contains(&search_lower)
                        },
                        _ => false,
                    }
                })?
            },
        };
        let offset: i32 = match &target {
            TargetVersion::Root(offset) => *offset,
            TargetVersion::Head(offset) => *offset,
            TargetVersion::Current(offset) => *offset,
            TargetVersion::Search((_, offset)) => *offset,
        };

        let final_offset = (((starting_index as i32) + offset).max(0) as usize)
            .min(num_versions - 1);
        Some(final_offset)
    }

    /// Gets a reference to the specified target version if it can be found in the version
    /// list.
    pub fn get_target(&self, target: &TargetVersion) -> Option<&SchemaVersion> {
        Some(&self.versions[self.get_target_position(target)?])
    }

    /// Given a target version, constructs a migration path to get from the current
    /// position to the target position.
    pub fn current_migration_path_to_target(
        &self,
        target: &TargetVersion,
    ) -> Result<MigrationPath, DbToolsError> {
        self.build_migration_path_from_targets(&TargetVersion::Current(0), target)
    }

    /// Given an absolute version, constructs a migration path to get from the current
    /// position to the target position
    pub fn current_migration_path_to_version(
        &self,
        version: &SchemaVersion,
    ) -> Result<MigrationPath, DbToolsError> {
        let start_pos = self.current_pos;
        let end_pos = self.get_version_pos(version)
            .ok_or_else(|| DbToolsError::VersionDoesNotExist(version.clone()))?;
        Ok(self.build_migration_path_from_pos(start_pos, end_pos))
    }

    /// Constructs an arbitrary migration path given a start and end position.
    pub fn build_migration_path_from_targets(
        &self,
        start: &TargetVersion,
        end: &TargetVersion,
    ) -> Result<MigrationPath, DbToolsError> {
        let start_pos = self.get_target_position(start)
            .ok_or_else(|| DbToolsError::TargetNotFound(start.clone()))?;
        let end_pos = self.get_target_position(end)
            .ok_or_else(|| DbToolsError::TargetNotFound(end.clone()))?;
        Ok(self.build_migration_path_from_pos(start_pos, end_pos))
    }

    /// Constructs a migration path given two absolute versions
    pub fn build_absolute_migration_path(
        &self,
        start: &SchemaVersion,
        end: &SchemaVersion,
    ) -> Result<MigrationPath, DbToolsError> {
        let start_pos = self.get_version_pos(start)
            .ok_or_else(|| DbToolsError::VersionDoesNotExist(start.clone()))?;
        let end_pos = self.get_version_pos(end)
            .ok_or_else(|| DbToolsError::VersionDoesNotExist(end.clone()))?;
        Ok(self.build_migration_path_from_pos(start_pos, end_pos))
    }

    /// Retrieves the position index of the specified version, or `None` if it does not
    /// exist.
    fn get_version_pos (&self, version: &SchemaVersion) -> Option<usize> {
        self.versions.iter().position(|v| v == version)
    }

    /// Struct-aware migration path builder, referencing the specific start and end
    /// positions of the target versions
    fn build_migration_path_from_pos(&self, start_pos: usize, end_pos: usize) -> MigrationPath {
        if start_pos < end_pos {
            let mut migrate_vec = vec![];
            for n in (start_pos+1)..=(end_pos) {
                migrate_vec.push(MigrationStep{
                    version: self.versions[n].clone(),
                    action: MigrationAction::new_from_direction(
                        &Direction::Up,
                        &self.versions[n],
                        &self.folder,
                    )
                });
            }
            migrate_vec
        } else if start_pos > end_pos {
            let mut migrate_vec = vec![];
            for n in (end_pos+1..=start_pos).rev() {
                migrate_vec.push(MigrationStep{
                    version: self.versions[n-1].clone(),
                    action: MigrationAction::new_from_direction(
                        &Direction::Dn,
                        &self.versions[n],
                        &self.folder,
                    )
                });
            }
            migrate_vec
        } else {
            vec![]
        }
    }

    /// Fetches the current version
    pub fn get_current(&self) -> &SchemaVersion {
        &self.versions[self.current_pos]
    }

    /// Sets the current version based on an absolutely provided migration version
    pub fn set_current(&mut self, version: &SchemaVersion) -> Result<(), DbToolsError> {
        let current_pos = self.versions.find_version_position(&version)
            .ok_or_else(|| DbToolsError::VersionDoesNotExist(version.clone()))?;
        self.current_pos = current_pos;
        Ok(())
    }

    /// Sets the current version based on a target
    pub fn set_current_target(&mut self, target: &TargetVersion) -> Result<(), DbToolsError> {
        let current_pos = self.get_target_position(target)
            .ok_or_else(|| DbToolsError::TargetNotFound(target.clone()))?;
        self.current_pos = current_pos;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;

    const TEST_FOLDER: &str = "../../tests/migrations";

    #[test]
    fn test_get_version_files() {
        assert_eq!(
            SchemaVersion::load_migration_folder(TEST_FOLDER).unwrap(),
            vec![
                SchemaVersion::Root,
                SchemaVersion::Version("01-create-user-table".into()),
                SchemaVersion::Version("02-update-user-table".into()),
                SchemaVersion::Version("03-clear-password".into()),
                SchemaVersion::Version("04-remove-password".into()),
            ],
        );
    }

    #[test]
    fn test_search_version() {
        let migration_status = MigrationPlanner::new_from_folder(
            TEST_FOLDER,
            SchemaVersion::Version("02-update-user-table".into()),
        )
            .unwrap();
        // Head
        assert_eq!(
            migration_status.get_target(&TargetVersion::Head(0))
                .unwrap(),
            &SchemaVersion::Version("04-remove-password".into()),
        );
        // Root
        assert_eq!(
            migration_status
                .get_target(&TargetVersion::Root(1))
                .unwrap(),
            &SchemaVersion::Version("01-create-user-table".into()),
        );
        // Name
        assert_eq!(
            migration_status
                .get_target(&TargetVersion::Search(("update-user".into(), 0)))
                .unwrap(),
            &SchemaVersion::Version("02-update-user-table".into()),
        );
    }

    #[test]
    fn test_get_migration_path() {
        // Upgrade from None
        assert_debug_snapshot!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Root,
            )
                .unwrap()
                .current_migration_path_to_target(&TargetVersion::Head(0))
                .unwrap(),
            @r#"
        [
            MigrationStep {
                version: Version(
                    "01-create-user-table",
                ),
                action: SqlFile(
                    "../../tests/migrations/01-create-user-table.up.sql",
                ),
            },
            MigrationStep {
                version: Version(
                    "02-update-user-table",
                ),
                action: SqlFile(
                    "../../tests/migrations/02-update-user-table.up.sql",
                ),
            },
            MigrationStep {
                version: Version(
                    "03-clear-password",
                ),
                action: SqlFile(
                    "../../tests/migrations/03-clear-password.up.sql",
                ),
            },
            MigrationStep {
                version: Version(
                    "04-remove-password",
                ),
                action: SqlFile(
                    "../../tests/migrations/04-remove-password.up.sql",
                ),
            },
        ]
        "#,
        );

        // Upgrade from a middle version
        assert_debug_snapshot!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("02-update-user-table".into()),
            )
                .unwrap()
                .current_migration_path_to_target(&TargetVersion::Search(("03-clear-password".into(), 0)))
                .unwrap(),
            @r#"
        [
            MigrationStep {
                version: Version(
                    "03-clear-password",
                ),
                action: SqlFile(
                    "../../tests/migrations/03-clear-password.up.sql",
                ),
            },
        ]
        "#,
        );

        // Downgrade to root
        assert_debug_snapshot!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("04-remove-password".into()),
            )
                .unwrap()
                .current_migration_path_to_target(&TargetVersion::Root(0))
                .unwrap(),
            @r#"
        [
            MigrationStep {
                version: Version(
                    "03-clear-password",
                ),
                action: SqlFile(
                    "../../tests/migrations/04-remove-password.dn.sql",
                ),
            },
            MigrationStep {
                version: Version(
                    "02-update-user-table",
                ),
                action: NoFileFound,
            },
            MigrationStep {
                version: Version(
                    "01-create-user-table",
                ),
                action: SqlFile(
                    "../../tests/migrations/02-update-user-table.dn.sql",
                ),
            },
            MigrationStep {
                version: Root,
                action: SqlFile(
                    "../../tests/migrations/01-create-user-table.dn.sql",
                ),
            },
        ]
        "#,
        );

        // Target is current
        assert_debug_snapshot!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("02-update-user-table".into()),
            )
                .unwrap()
                .current_migration_path_to_target(&TargetVersion::Search(("04-remove-password".into(), -2)))
                .unwrap(),
            @"[]",
        );
    }

    #[test]
    fn test_target_version_new_from_str() {
        assert_eq!(
            TargetVersion::new_from_str("HEAD"),
            TargetVersion::Head(0),
        );
        assert_eq!(
            TargetVersion::new_from_str("HEAD~2"),
            TargetVersion::Head(-2),
        );
        assert_eq!(
            TargetVersion::new_from_str("ROOT"),
            TargetVersion::Root(0),
        );
        assert_eq!(
            TargetVersion::new_from_str("ROOT+3"),
            TargetVersion::Root(3),
        );
        assert_eq!(
            TargetVersion::new_from_str("@"),
            TargetVersion::Current(0),
        );
        assert_eq!(
            TargetVersion::new_from_str("@+3"),
            TargetVersion::Current(3),
        );
        assert_eq!(
            TargetVersion::new_from_str("@~44"),
            TargetVersion::Current(-44),
        );
        assert_eq!(
            TargetVersion::new_from_str("find-ver"),
            TargetVersion::Search(("find-ver".into(), 0)),
        );
        assert_eq!(
            TargetVersion::new_from_str("find-ver~3"),
            TargetVersion::Search(("find-ver".into(), -3)),
        );
    }
}
