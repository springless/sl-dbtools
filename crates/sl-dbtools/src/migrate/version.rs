use std::{fmt::Display, path::Path};

/// Represents a specific version of the database. When a version is
/// `Root`, that means that the database currently has no version, at all.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum SchemaVersion {
    /// Represents the state of the database before the first migration is run.
    Root,
    Version(String),
}

impl Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaVersion::Root => {
                write!(f, "ROOT")
            },
            SchemaVersion::Version(version_str) => {
                write!(f, "{}", version_str)
            }
        }
    }
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

impl Display for TargetVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_shorthand())
    }
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
