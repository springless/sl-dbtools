use std::time::SystemTime;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// This describes the values used to generate a new name for a database starting from a
/// base name. The example and original use case for this is constructing slightly random
/// but still readable names for temporary test databases. So for example my main project
/// database might be `my_project`. A test might request a database with the struct:
/// ```rust
/// DbNameProps {
///     base: "my_project",
///     time: Some(timestamp_var), // 2024.10.17 20:38:14.1234
///     uuid: Some(uuid_var), // "3a45686d-8213-48b3-b817-7e28c80f6e71"
///     name: "my_test",
/// };
/// ```
///
/// This would yield a database with the full name:
/// ```
/// {time}_{base}_{name}_{uuid}
/// 20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71
/// ```
///
/// Postgres typically limits identifier lengths to 63 characters, so this system preemptively
/// cuts the name short and adds the small hash value that postgres uses to truncate values
/// at the end, yielding:
///
/// ```
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
}

/// Postgres can only have a maximum identifier length of 63 characters, so this will take
/// a passed-in name and truncate it with an MD5 hash based on the full name appended to the end.
/// This is the same way that postgres will rename a too-long identifier when it is created
/// under the hood. Postgres seems to cap these shortened names at 60 characters, though, rather
/// than taking up the full allowed 63, so this function does the same. Up until 63 characters,
/// however, the name remains untouched.
///
/// ```
/// before: 20241017203814_my_project_my_test_3a45686d_8213_48b3_b817_7e28c80f6e71
/// after:  20241017203814_my_project_my_test_3a45686d_8213_48b3_b81118c
///                                                                 ^^^^
///                                                                 hash
/// before: short_identifier
/// after:  short_identifier
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

pub trait ToNameTime {
    /// Converts a time structure in such a way that it complies with the time portion of the
    /// database naming convention described above:
    /// ```
    /// yyyymmddHHMMSS
    /// ```
    fn to_name_time(&self) -> String;
}

fn get_now() -> DateTime<Utc> {
    chrono::Utc::now()
}

#[cfg(test)]
mod tests {
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
