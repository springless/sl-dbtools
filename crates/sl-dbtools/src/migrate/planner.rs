use std::path::{Path, PathBuf};

use crate::error::DbToolsError;

/// Represents a specific version of the database. When a version is
/// `Root`, that means that the database currently has no version, at all.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum SchemaVersion {
    /// Represents the state of the database before the first migration is run.
    Root,
    Version(String),
}

impl SchemaVersion {
    /// Generates the list of migrations to apply based on the folder supplied, a target version,
    /// and the current version.
    pub fn load_migration_folder<P>(p: P) -> std::io::Result<Vec<SchemaVersion>>
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
        let mut versions: Vec<_> = versions
            .into_iter()
            .map(|ver| SchemaVersion::Version(ver))
            .collect();
        versions.insert(0, SchemaVersion::Root);
        Ok(versions)
    }

    pub fn up_file_name(&self) -> Option<String> {
        match &self {
            Self::Root => None,
            Self::Version(version_string) => {
                Some(format!("{version}.up.sql",
                    version = version_string,
                ))
            },
        }
    }

    pub fn dn_file_name(&self) -> Option<String> {
        match &self {
            Self::Root => None,
            Self::Version(version_string) => {
                Some(format!("{version}.dn.sql",
                    version = version_string,
                ))
            },
        }
    }

    /// Attempts to locate the migration file. If it exists, then it will return the
    /// path to the file, and if it does not then will return None.
    pub fn up_migration_file<P>(
        &self,
        folder: P,
    ) -> Option<PathBuf>
    where P: AsRef<Path> {
        let folder_path = folder.as_ref();
        let file_name = self.up_file_name()?;
        let full_path = folder_path.join(file_name);

        if full_path.exists() {
            Some(full_path)
        } else {
            None
        }
    }

    /// Attempts to locate the down migration file. If it exists, then it will return the
    /// path to the file, and if it does not then will return None.
    pub fn dn_migration_file<P>(
        &self,
        folder: P,
    ) -> Option<PathBuf>
    where P: AsRef<Path> {
        let folder_path = folder.as_ref();
        let file_name = self.dn_file_name()?;
        let full_path = folder_path.join(file_name);

        if full_path.exists() {
            Some(full_path)
        } else {
            None
        }
    }
}


/// Represents a version being requested
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum TargetVersion {
    /// Represents the state of the database before the first migration is run.
    Root(i32),
    /// Represents the current version of the database plus an offset.
    Current(i32),
    /// A version identified by search string and an optional offset from that version.
    Search((String, i32)),
    /// Represents the final value in the migration path plus an offset.
    Head(i32),
}

impl TargetVersion {
    /// Given a string, will parse it into the target version being requested. There are
    /// three special target versions:
    ///
    /// 1. `ROOT` - Targets the migration version BEFORE the very first migration version, aka.
    ///     the state of the database prior to applying any migrations, at all.
    /// 2. `HEAD` - Targets the very last migration version
    /// 3. `@` - Targets the current migration version
    ///
    /// If the version string is any other value, it will treat it like a search string, meaning
    /// that it will target whatever migration that search string uniquely identifies in the
    /// version list.
    ///
    /// In addition to the specific target, an optional offset can also be provided via `+` to
    /// specify a version after the one specified, or `~` to specify a version before the one
    /// specified. So `HEAD~2` targets two versions before the very last version, and `ROOT+1`
    /// would target the first actual version in the migration path. `@+2` would target two
    /// versions after the current version.
    pub fn new_from_str(target: &str) -> TargetVersion {
        let (target_ver, offset) = if let Some(_) = target.rfind('~') {
            let (target_ver, offset_str) = target.rsplit_once('~').unwrap();
            let offset = 0 - offset_str.parse::<i32>().unwrap_or(0);
            (target_ver, offset)
        } else if let Some(_) = target.rfind('+') {
            let (target_ver, offset_str) = target.rsplit_once('+').unwrap();
            let offset = offset_str.parse::<i32>().unwrap_or(0);
            (target_ver, offset)
        } else {
            (target, 0)
        };

        match &target_ver {
            &"ROOT" => TargetVersion::Root(offset),
            &"@" => TargetVersion::Current(offset),
            &"HEAD" => TargetVersion::Head(offset),
            _ => TargetVersion::Search((target_ver.to_owned(), offset)),
        }
    }

    /// Will convert the target to the shorthand version of the target, meaning the string
    /// representation that a user would type to use this target.
    pub fn to_shorthand(&self) -> String {
        let offset: i32 = match self {
            TargetVersion::Root(offset) => *offset,
            TargetVersion::Current(offset) => *offset,
            TargetVersion::Search((_, offset)) => *offset,
            TargetVersion::Head(offset) => *offset,
        };
        let target_str = match self {
            TargetVersion::Root(_) => "ROOT",
            TargetVersion::Head(_) => "HEAD",
            TargetVersion::Current(_) => "@",
            TargetVersion::Search((search_str, _)) => search_str,
        };
        if offset == 0 {
            format!("{}", target_str)
        } else {
            format!(
                "{}{}{}",
                target_str,
                if offset > 0 { "+" } else { "~" },
                offset.abs(),
            )
        }
    }
}

/// A specific sequence of migration versions that must be stepped through in order to complete
/// a requested migration.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum MigrationPath {
    Up(Vec<SchemaVersion>),
    Eq,
    Dn(Vec<SchemaVersion>),
}

impl MigrationPath {
    pub fn files<P>(&self, dir: &P) -> Vec<Option<PathBuf>>
    where P: AsRef<Path> + ?Sized {
        match self {
            MigrationPath::Up(path_vec) => path_vec.iter()
                .map(|item| item.up_migration_file(dir))
                .collect(),
            MigrationPath::Dn(path_vec) => path_vec.iter()
                .map(|item| item.dn_migration_file(dir))
                .collect(),
            MigrationPath::Eq => vec![],
        }
    }
}

/// The current status of a database within the migration system
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct MigrationPlanner {
    versions: Vec<SchemaVersion>,
    /// Position of the current version in the versions path
    current_pos: usize,
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
        let versions = SchemaVersion::load_migration_folder(p)?;
        let current_pos = versions.find_version_position(&current)
            .ok_or_else(|| DbToolsError::VersionDoesNotExist(current.clone()))?;

        Ok(MigrationPlanner {
            versions,
            current_pos,
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
    pub fn current_migration_path(
        &self,
        target: &TargetVersion,
    ) -> Result<MigrationPath, DbToolsError> {
        self.build_migration_path(&TargetVersion::Current(0), target)
    }

    /// Constructs an arbitrary migration path given a start and end position.
    pub fn build_migration_path(
        &self,
        start: &TargetVersion,
        end: &TargetVersion,
    ) -> Result<MigrationPath, DbToolsError> {
        let start_pos = self.get_target_position(start)
            .ok_or_else(|| DbToolsError::TargetNotFound(start.clone()))?;
        let end_pos = self.get_target_position(end)
            .ok_or_else(|| DbToolsError::TargetNotFound(end.clone()))?;

        if start_pos < end_pos {
            let mut migrate_vec = vec![];
            for n in (start_pos)..=(end_pos) {
                migrate_vec.push(self.versions[n].clone());
            }
            Ok(MigrationPath::Up(migrate_vec))
        } else if start_pos > end_pos {
            let mut migrate_vec = vec![];
            for n in (end_pos..=start_pos).rev() {
                migrate_vec.push(self.versions[n].clone());
            }
            Ok(MigrationPath::Dn(migrate_vec))
        } else {
            Ok(MigrationPath::Eq)
        }
    }

    /// Fetches the current version
    pub fn get_current(&self) -> &SchemaVersion {
        &self.versions[self.current_pos]
    }

    /// Sets the current version based on an absolutely provided migration version
    pub fn set_current(&mut self, version: SchemaVersion) -> Result<(), DbToolsError> {
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
        assert_eq!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Root,
            )
                .unwrap()
                .current_migration_path(&TargetVersion::Head(0))
                .unwrap(),
            MigrationPath::Up(
                vec![
                    SchemaVersion::Root,
                    SchemaVersion::Version("01-create-user-table".into()),
                    SchemaVersion::Version("02-update-user-table".into()),
                    SchemaVersion::Version("03-clear-password".into()),
                    SchemaVersion::Version("04-remove-password".into()),
                ],
            ),
        );

        // Upgrade from a middle version
        assert_eq!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("02-update-user-table".into()),
            )
                .unwrap()
                .current_migration_path(&TargetVersion::Search(("03-clear-password".into(), 0)))
                .unwrap(),
            MigrationPath::Up(
                vec![
                    SchemaVersion::Version("02-update-user-table".into()),
                    SchemaVersion::Version("03-clear-password".into()),
                ],
            ),
        );

        // Downgrade to root
        assert_eq!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("04-remove-password".into()),
            )
                .unwrap()
                .current_migration_path(&TargetVersion::Root(0))
                .unwrap(),
            MigrationPath::Dn(
                vec![
                    SchemaVersion::Version("04-remove-password".into()),
                    SchemaVersion::Version("03-clear-password".into()),
                    SchemaVersion::Version("02-update-user-table".into()),
                    SchemaVersion::Version("01-create-user-table".into()),
                    SchemaVersion::Root,
                ],
            )
        );

        // Target is current
        assert_eq!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("02-update-user-table".into()),
            )
                .unwrap()
                .current_migration_path(&TargetVersion::Search(("04-remove-password".into(), -2)))
                .unwrap(),
            MigrationPath::Eq,
        );
    }

    #[test]
    fn test_get_migration_version_file_name() {
        assert_eq!(
            SchemaVersion::Version("01-create-user-table".into()).dn_file_name().unwrap(),
            "01-create-user-table.dn.sql",
        );
    }

    #[test]
    fn test_get_migration_path_files() {
        assert_eq!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Root,
            )
                .unwrap()
                .current_migration_path(&TargetVersion::Head(0))
                .unwrap()
                .files(TEST_FOLDER),
            vec![
                None,
                Some("../../tests/migrations/01-create-user-table.up.sql".into()),
                Some("../../tests/migrations/02-update-user-table.up.sql".into()),
                Some("../../tests/migrations/03-clear-password.up.sql".into()),
                Some("../../tests/migrations/04-remove-password.up.sql".into()),
            ],
        );

        assert_eq!(
            MigrationPlanner::new_from_folder(
                TEST_FOLDER,
                SchemaVersion::Version("04-remove-password".into()),
            )
                .unwrap()
                .current_migration_path(&TargetVersion::Root(0))
                .unwrap()
                .files(TEST_FOLDER),
            vec![
                Some("../../tests/migrations/04-remove-password.dn.sql".into()),
                None,
                Some("../../tests/migrations/02-update-user-table.dn.sql".into()),
                Some("../../tests/migrations/01-create-user-table.dn.sql".into()),
                None,
            ],
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
