use std::ops::{Deref, DerefMut};

use sqlx::{postgres::PgConnectOptions, ConnectOptions};
use url::{ParseError, Url};

use super::namer::{DbNamingProps, ToDbId};

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

    pub fn set_dbname(&mut self, dbname: &str) {
        self.set_path(&format!("/{}", dbname))
    }

    pub fn get_pg_conn_opts(&self) -> Result<PgConnectOptions, sqlx::Error> {
        PgConnectOptions::from_url(self)
    }

    /// Attempts to guess what the Postgres maintenance URL for this database would be and returns
    /// a new `DbUrl` with the same connection credentials as this one
    pub fn guess_pg_maintenance_url(&self) -> Self {
        let mut new_conn = self.clone();
        let maint_name = if new_conn.dbname() == "postgres" {
            "template1"
        } else {
            "postgres"
        };
        new_conn.set_dbname(maint_name);
        new_conn
    }

    /// Creates a new transient database connection string
    pub fn new_default_transient_url(&self, name: Option<&str>) -> Self {
        let base = self.dbname();
        let new_name = DbNamingProps::new_default(base, name)
            .to_db_id();
        let mut new_url = self.clone();
        new_url.set_dbname(&new_name);
        new_url
    }
}

impl TryFrom<&str> for DbUrl {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        DbUrl::parse(value)
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

    #[test]
    fn test_set_dburl() {
        let mut parsed_url = DbUrl::parse("postgresql://user:pass@host:5432/dbname?queryparam=12").expect("Failed to parse URL");
        assert_eq!(parsed_url.dbname(), "dbname");
        parsed_url.set_dbname("new_dbname");
        assert_eq!(parsed_url.dbname(), "new_dbname");
        assert_eq!(parsed_url.as_str(), "postgresql://user:pass@host:5432/new_dbname?queryparam=12")

    }
}
