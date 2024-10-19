use std::time::SystemTime;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// This describes the values used to generate a new name for a database starting from a
/// base name. The example and original use case for this is constructing slightly random
/// but still readable names for temporary test databases. So for example my main project
/// database might be `my_project`. A test might request a database with the struct:
/// ```rust
/// let props = DbNamingProps {
///     base: "my_project",
///     time: Some(timestamp_var), // eg. 2024.10.17 20:38:14.1234
///     uuid: Some(uuid_var), // eg. "3a45686d-8213-48b3-b817-7e28c80f6e71"
///     name: "my_test",
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
pub struct DbNamingProps {
    base: String,
    time: Option<DateTime<Utc>>,
    name: Option<String>,
    uuid: Option<Uuid>,
    keep_full: bool,
}

/// Postgres can only have a maximum identifier length of 63 characters, so this will take
/// a passed-in name and truncate it with an MD5 hash based on the full name appended to the end.
/// This is the same way that postgres will rename a too-long identifier when it is created
/// under the hood. Postgres seems to cap these shortened names at 60 characters, though, rather
/// than taking up the full allowed 63, so this function does the same. Up until 63 characters,
/// however, the name remains untouched.
///
/// ```rust
/// assert_eq!(
///     truncate_identifier(
///         "20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71"
///     ),
///     "20241017203814_my_project_my_test_3a45686d_8213_48b3_b81118c",
///     //                                                       ^^^^
///     //                                                       hash
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
    /// assert_eq!(
    ///     uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8")
    ///         .to_db_id(),
    ///     "67e55044_10b1_426f_9247_bb680e5fe0c8",
    /// );
    /// ```
    fn to_db_id(&self) -> String {
        self.to_string()
            .replace('-', "_")
    }
}

impl ToDbId for DbNamingProps {
    fn to_db_id(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(part) = self.time {
            parts.push(part.to_db_id());
        }
        parts.push(self.base.to_string());
        if let Some(part) = &self.name {
            parts.push(part.to_string());
        }
        if let Some(part) = self.uuid {
            parts.push(part.to_db_id());
        }

        let combined = parts.join("_");

        if self.keep_full {
            combined
        } else {
            truncate_identifier(combined)
        }
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
    fn test_dbnamingprops_to_db_id() {
        let timestamp = Utc.with_ymd_and_hms(2024, 10, 17, 20, 38, 14).unwrap();
        let this_uuid = uuid!("3a45686d-8213-48b3-b817-7e28c80f6e71");
        assert_eq!(
            DbNamingProps {
                base: "my_project".into(),
                name: Some("my_test".into()),
                time: Some(timestamp),
                uuid: Some(this_uuid),
                keep_full: false,
            }.to_db_id(),
            "20241017203814_my_project_my_test_3a45686d_8213_48b3_b81118c",
        );
        assert_eq!(
            DbNamingProps {
                base: "my_project".into(),
                name: Some("my_test".into()),
                time: Some(timestamp),
                uuid: Some(this_uuid),
                keep_full: true,
            }.to_db_id(),
            "20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71",
        );
    }
}
