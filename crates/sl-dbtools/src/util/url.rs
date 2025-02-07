use std::ops::{Deref, DerefMut};

use url::{ParseError, Url};

/// Some utility functions and traits for dealing with connection URLs

/// Newtype struct so we can add a couple utility functions on top of
/// the URL type, such as being able to get a cleaned up version of the
/// database string.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct DbUrl(Url);

impl Deref for DbUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DbUrl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DbUrl {
    pub fn new(url: Url) -> Self {
        DbUrl(url)
    }

    pub fn parse(url: &str) -> Result<Self, ParseError> {
        Ok(DbUrl(Url::parse(url)?))
    }

    /// Retrieve the database name portion of the url, which is basically just
    /// the path sans the beginning and/or ending slash
    pub fn dbname(&self) -> &str {
        let path = self.path();
        path.trim_matches('/')
    }
}

#[cfg(test)]
mod tests {
    use url::Host;

    use super::*;

    #[test]
    fn test_dburl_parse() {
        let parsed_url = DbUrl::parse("postgresql://user:pass@host:5432/dbname?queryparam=12").expect("Failed to parse URL");
        assert_eq!(parsed_url.scheme(), "postgresql");
        assert_eq!(parsed_url.username(), "user");
        assert_eq!(parsed_url.password(), Some("pass"));
        assert_eq!(parsed_url.host(), Some(Host::Domain("host")));
        assert_eq!(parsed_url.port(), Some(5432));
        assert_eq!(parsed_url.path(), "/dbname");
        assert_eq!(parsed_url.dbname(), "dbname");
    }
}
