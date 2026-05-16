use std::path::PathBuf;

use uuid::Uuid;
use chrono::{DateTime, Utc};

const DEFAULT_NAME_PATTERN: &str = "z{timestamp}_{base}_{name}";

#[derive(Debug, Clone)]
pub enum DbNamingTemplate {
    /// Use the default naming pattern (`z{timestamp}_{base}_{name}`)
    Default,
    /// Specify a custom naming pattern. See `DbNamingProps` for more details
    Pattern(String),
}

impl DbNamingTemplate {
    pub fn into_pattern(&self) -> String {
        match self {
            DbNamingTemplate::Default => DEFAULT_NAME_PATTERN.to_owned(),
            DbNamingTemplate::Pattern(patt) => patt.to_owned(),
        }
    }
}

/// This describes the values used to generate a new name for a database starting from a
/// base name. The example and original use case for this is constructing slightly random
/// but still readable names for temporary test databases. So for example my main project
/// database might be `my_project`. A test might request a database with the struct:
/// ```rust
/// use sl_dbtools::namer::{DbNamingProps, DbNamingTemplate::Pattern};
/// let props = DbNamingProps {
///     base: Some("my_project".into()),
///     name: Some("my_test".into()),
///     pattern: Pattern("{timestamp}_{base}_{name}_{uuid}".into()),
///     keep_full: true,
/// };
/// ```
///
/// This would yield a database with the full name:
/// ```text
/// {time}_{base}_{name}_{uuid}
/// 20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71
/// ```
///
/// Postgres typically limits identifier lengths to 63 characters, so this system preemptively
/// cuts the name short and adds the small hash value that postgres uses to truncate values
/// at the end, yielding:
///
/// ```text
/// 20241017203814_my_project_my_test_3a45686d_8213_48b3_b81118c
///                                                         ^^^^
///                                                         hash
/// ```
///
/// It will do this for all database types, regardless whether it holds the same restrictions
/// as Postgres, unless `keep_full` is set to `true`.
///
/// The available pattern variables are:
///
/// - `base` - The base name of the database, taken from the `DATABASE_URL` connection
/// - `name` - An extra name appended to the database, typically used to identify what
/// - `timestamp` - A timestamp string in the format: `YYYYmmddHHMMSS`
///   test or specific reason the database exists to serve
/// - 'uuid' - A randomly generated UUID
///
/// Any characters outside of curly braces will be interpreted literally.
pub struct DbNamingProps {
    pub pattern: DbNamingTemplate,
    pub base: Option<String>,
    pub name: Option<String>,
    pub keep_full: bool,
}

impl DbNamingProps {
    /// Creates a new database name utilizing the default configuration, which is
    /// including a timestamp, a uuid, and truncating the full name if it is over
    /// the character limit. The timestamp is generated at the time of calling
    /// this function, and the UUID is random.
    pub fn new_default<T: AsRef<str>>(base: T, name: Option<T>) -> Self {
        let this_name = name.map(|v| v.as_ref().to_string());
        DbNamingProps {
            pattern: DbNamingTemplate::Default,
            base: Some(base.as_ref().into()),
            name: this_name,
            keep_full: false,
        }
    }

    /// Creates a new DbNamingProps instance from a DbNamingOpts object
    pub fn new_from_opts(opts: DbNamingOpts) -> Self {
        opts.build()
    }

    /// Creates a regex that makes a best-guess match for the naming props
    pub fn into_regex(&self) -> String {
        let pattern = match &self.pattern {
            DbNamingTemplate::Default => DEFAULT_NAME_PATTERN,
            DbNamingTemplate::Pattern(p) => p.as_str(),
        };

        let mut regex = String::from("^");
        let mut min_len: usize = 0;
        let mut uncertain = false;
        let mut rest = pattern;

        while !rest.is_empty() && !uncertain {
            if let Some(open) = rest.find('{') {
                if open > 0 {
                    let literal = &rest[..open];
                    if !self.keep_full && min_len + literal.len() > 56 {
                        uncertain = true;
                        break;
                    }
                    regex.push_str(&pg_regex_escape(literal));
                    min_len += literal.len();
                }
                rest = &rest[open + 1..];

                if let Some(close) = rest.find('}') {
                    let placeholder = &rest[..close];
                    rest = &rest[close + 1..];

                    match placeholder {
                        "timestamp" => {
                            if !self.keep_full && min_len + 14 > 56 {
                                uncertain = true;
                            } else {
                                regex.push_str(r"\d{14}");
                                min_len += 14;
                            }
                        }
                        "uuid" => {
                            if !self.keep_full && min_len + 36 > 56 {
                                uncertain = true;
                            } else {
                                regex.push_str(r"[0-9a-f]{8}_[0-9a-f]{4}_[0-9a-f]{4}_[0-9a-f]{4}_[0-9a-f]{12}");
                                min_len += 36;
                            }
                        }
                        "base" => match &self.base {
                            Some(n) => {
                                if !self.keep_full && min_len + n.len() > 56 {
                                    uncertain = true;
                                } else {
                                    regex.push_str(&pg_regex_escape(n));
                                    min_len += n.len();
                                }
                            }
                            None => uncertain = true,
                        }
                        "name" => match &self.name {
                            Some(n) => {
                                if !self.keep_full && min_len + n.len() > 56 {
                                    uncertain = true;
                                } else {
                                    regex.push_str(&pg_regex_escape(n));
                                    min_len += n.len();
                                }
                            }
                            None => uncertain = true,
                        },
                        _ => uncertain = true,
                    }
                } else {
                    regex.push_str(&pg_regex_escape(rest));
                    rest = "";
                }
            } else {
                if !self.keep_full && min_len + rest.len() > 56 {
                    uncertain = true;
                } else {
                    regex.push_str(&pg_regex_escape(rest));
                    min_len += rest.len();
                }
                rest = "";
            }
        }

        if uncertain {
            // The separator before an absent optional field may have been eaten by
            // the underscore-collapsing logic in interpolate_db_name, so make it optional.
            if regex.ends_with('_') {
                regex.pop();
                regex.push_str("_?");
            }
            regex.push_str(".*");
        } else if self.keep_full {
            regex.push('$');
        } else {
            // Truncation may have altered the tail, so don't anchor at end
            regex.push_str(".*");
        }

        regex
    }
}

impl ToDbId for DbNamingProps {
    fn to_db_id(&self) -> String {
        let pattern = match &self.pattern {
            DbNamingTemplate::Default => DEFAULT_NAME_PATTERN,
            DbNamingTemplate::Pattern(p) => p.as_str(),
        };

        let timestamp = pattern
            .contains("{timestamp}")
            .then(Utc::now);

        let uuid = pattern
            .contains("{uuid}")
            .then(Uuid::new_v4);

        let result = interpolate_db_name(
            pattern,
            self.base.as_deref(),
            self.name.as_deref(),
            timestamp,
            uuid,
        );

        if self.keep_full {
            result
        } else {
            truncate_identifier(result)
        }
    }
}

impl ToDbId for PathBuf {
    fn to_db_id(&self) -> String {
        self.to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    }
}

fn interpolate_db_name(
    pattern: &str,
    base: Option<&str>,
    name: Option<&str>,
    timestamp: Option<chrono::DateTime<Utc>>,
    uuid: Option<Uuid>,
) -> String {
    let timestamp_str = timestamp.map(|ts| ts.to_db_id());
    let uuid_str = uuid.map(|uuid| uuid.to_db_id());
    let raw = pattern
        .replace("{base}", base.unwrap_or(""))
        .replace("{name}", name.unwrap_or(""))
        .replace("{timestamp}", timestamp_str.as_deref().unwrap_or(""))
        .replace("{uuid}", uuid_str.as_deref().unwrap_or(""));

    // Collapse runs of underscores left behind by missing optional values.
    // Starting with prev=true also eats any leading underscore.
    let mut out = String::with_capacity(raw.len());
    let mut prev_underscore = true;
    for ch in raw.chars() {
        if ch == '_' {
            if !prev_underscore {
                out.push(ch);
                prev_underscore = true;
            }
        } else {
            out.push(ch);
            prev_underscore = false;
        }
    }
    if out.ends_with('_') {
        out.pop();
    }
    out
}


fn pg_regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        if matches!(ch, '.' | '\\' | '[' | ']' | '^' | '$' | '*' | '+' | '?' | '{' | '}' | '|' | '(' | ')') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

// When called, an implementor of this trait should be able to generate a new version
// of itself, but incorporating the provided "name" into the new database version, and
// the rest of the `DbNamingProps` values.
pub trait MakeNewConnectOpts {
    /// Uses the default naming conventions described in `new_default` of `DbNamingProps`.
    fn make_new_connection_default(&self, name: Option<&str>) -> Self;
}

/// Options for automatically generating a new name, so certain elements can be
/// omitted or included without having to explicitly create a new Uuid or timestamp
pub struct DbNamingOpts {
    pub pattern: DbNamingTemplate,
    pub base: Option<String>,
    pub name: Option<String>,
    pub keep_full: bool,
}

impl DbNamingOpts {
    pub fn build(self) -> DbNamingProps {
        let base = if let Some(base) = self.base {
            base
        } else { "temp".to_owned() };
        DbNamingProps {
            base: Some(base),
            name: self.name,
            pattern: self.pattern,
            keep_full: self.keep_full,
        }
    }
}

/// Postgres can only have a maximum identifier length of 63 characters, so this will take
/// a passed-in name and truncate it with an MD5 hash based on the full name appended to the end.
/// This is the same way that postgres will rename a too-long identifier when it is created
/// under the hood. Postgres seems to cap these shortened names at 60 characters, though, rather
/// than taking up the full allowed 63, so this function does the same. Up until 63 characters,
/// however, the name remains untouched.
///
/// ```rust
/// use sl_dbtools::namer::truncate_identifier;
/// assert_eq!(
///     truncate_identifier(
///         "20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71"
///     ),
///     "20241017203814_my_project_my_test_3a45686d_8213_48b3_b81118c",
///     //                                                       ^^^^
///     //                                                       hash
/// );
/// assert_eq!(
///     truncate_identifier(
///         "short_identifier"
///     ),
///     "short_identifier",
/// );
/// ```
pub fn truncate_identifier<T: AsRef<str>>(val: T) -> String {
    let v = val.as_ref();
    if v.len() < 63 {
        v.to_string()
    } else {
        let digest = md5::compute(v);
        // The truncated string only keeps the last 4 characters of the MD5 sum
        let suffix = format!(
            "{:02x}{:02x}",
            digest[digest.len() - 2],
            digest[digest.len() - 1],
        );
        let truncated_identifier = &v[..56];
        format!("{}{}", truncated_identifier, suffix)
    }
}

pub trait ToDbId {
    /// Outputs a string formatted in a standardized way such that
    /// it can be used as an identifier in the database.
    fn to_db_id(&self) -> String;
}

impl ToDbId for DateTime<Utc> {
    /// Outputs a date in the format:
    /// ```text
    /// yyyymmddHHMMSS
    /// ```
    fn to_db_id(&self) -> String {
        self.format("%Y%m%d%H%M%S").to_string()
    }
}

impl ToDbId for Uuid {
    /// Outputs the standard UUID string format, but with underscores
    /// instead of dashes
    /// ```rust
    /// use sl_dbtools::namer::ToDbId;
    /// assert_eq!(
    ///     uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8")
    ///         .to_db_id(),
    ///     "67e55044_10b1_426f_9247_bb680e5fe0c8",
    /// );
    /// ```
    fn to_db_id(&self) -> String {
        self.to_string()
            .replace('-', "_")
    }
}

#[cfg(test)]
mod tests_truncate_identifier {
    use super::*;

    #[test]
    fn test_truncate_identifier_long() {
        let identifier = "20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71";
        let expected = "20241017203814_my_project_my_test_3a45686d_8213_48b3_b81118c";
        assert_eq!(truncate_identifier(identifier), expected);
    }

    #[test]
    fn test_truncate_identifier_short() {
        let identifier = "short_identifier";
        let expected = "short_identifier";
        assert_eq!(truncate_identifier(identifier), expected);
    }

    #[test]
    fn test_truncate_identifier_62() {
        let identifier = "00000000001111111111222222222233333333334444444444555555555566";
        let expected = "00000000001111111111222222222233333333334444444444555555555566";
        assert_eq!(truncate_identifier(identifier), expected);
    }

    #[test]
    fn test_truncate_identifier_63() {
        let identifier = "000000000011111111112222222222333333333344444444445555555555666";
        let expected = "000000000011111111112222222222333333333344444444445555554308";
        assert_eq!(truncate_identifier(identifier), expected);
    }
}

#[cfg(test)]
mod tests_to_db_id {
    use chrono::TimeZone;
    use uuid::uuid;

    use super::*;

    fn ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 10, 17, 20, 38, 14).unwrap()
    }
    const UUID: Uuid = uuid!("3a45686d-8213-48b3-b817-7e28c80f6e71");

    #[test]
    fn test_datetime_to_db_id() {
        assert_eq!(
            Utc.with_ymd_and_hms(2024, 10, 19, 10, 19, 20).unwrap()
                .to_db_id(),
            "20241019101920",
        );
    }

    #[test]
    fn test_uuid_to_db_id() {
        assert_eq!(
            uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8")
                .to_db_id(),
            "67e55044_10b1_426f_9247_bb680e5fe0c8",
        );
    }

    #[test]
    fn test_interpolate_all_present() {
        assert_eq!(
            interpolate_db_name(
                "{timestamp}_{base}_{name}_{uuid}",
                Some("my_project"), Some("my_test"), Some(ts()), Some(UUID),
            ),
            "20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71",
        );
    }

    #[test]
    fn test_interpolate_no_name() {
        assert_eq!(
            interpolate_db_name(
                "{timestamp}_{base}_{name}_{uuid}",
                Some("my_project"), None, Some(ts()), Some(UUID),
            ),
            "20241017203814_my_project_3a45686d_8213_48b3_b817_7e28c80f6e71",
        );
    }

    #[test]
    fn test_interpolate_no_timestamp() {
        assert_eq!(
            interpolate_db_name(
                "{timestamp}_{base}_{name}_{uuid}",
                Some("my_project"), Some("my_test"), None, Some(UUID),
            ),
            "my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71",
        );
    }

    #[test]
    fn test_interpolate_no_uuid() {
        assert_eq!(
            interpolate_db_name(
                "{timestamp}_{base}_{name}_{uuid}",
                Some("my_project"), Some("my_test"), Some(ts()), None,
            ),
            "20241017203814_my_project_my_test",
        );
    }

    #[test]
    fn test_interpolate_base_only() {
        assert_eq!(
            interpolate_db_name(
                "{timestamp}_{base}_{name}_{uuid}",
                Some("my_project"), None, None, None,
            ),
            "my_project",
        );
    }

    #[test]
    fn test_interpolate_default_pattern() {
        assert_eq!(
            interpolate_db_name(DEFAULT_NAME_PATTERN, Some("my_project"), Some("my_test"), Some(ts()), None),
            "z20241017203814_my_project_my_test",
        );
    }

    #[test]
    fn test_interpolate_default_pattern_no_name() {
        assert_eq!(
            interpolate_db_name(DEFAULT_NAME_PATTERN, Some("my_project"), None, Some(ts()), None),
            "z20241017203814_my_project",
        );
    }

    #[test]
    fn test_dbnamingprops_to_db_id_smoke() {
        let result = DbNamingProps {
            pattern: DbNamingTemplate::Default,
            base: Some("my_project".into()),
            name: Some("my_test".into()),
            keep_full: false,
        }.to_db_id();
        assert!(!result.contains('{'), "unresolved placeholder in output");
        assert!(result.starts_with('z'));
        assert!(result.contains("my_project"));
        assert!(result.len() <= 63);
    }
}

#[cfg(test)]
mod tests_dbnamingprops {
    use super::*;

    #[test]
    fn test_new_default() {
        let props = DbNamingProps::new_default("my_db", Some("my_test"));
        assert_eq!(props.base, Some("my_db".into()));
        assert_eq!(props.name, Some("my_test".into()));
        assert!(!props.keep_full);
    }
}
