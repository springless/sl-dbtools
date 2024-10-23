use sqlx::{postgres::PgConnectOptions, Error, ConnectOptions, Executor};

/// Attempts to guess the name of the maintenance database. It is theoretically possible
/// that a postgres server will not have `postgres` or `template1`, but this assumes that
/// they will.
/// Shamelessly stolen and tweaked from core sqlx
pub fn parse_for_maintenance(
    options: &PgConnectOptions,
) -> PgConnectOptions {
    // pull out the name of the database to create
    let database = options
        .get_database()
        .as_deref()
        .unwrap_or(&options.get_username())
        .to_owned();

    // switch us to the maintenance database
    // use `postgres` _unless_ the database is postgres, in which case, use `template1`
    // this matches the behavior of the `createdb` util
    let maint_options = options.clone().database(if database == "postgres" {
        "template1"
    } else {
        "postgres"
    });

    maint_options
}

/// Drops a database, given an admin connection.
/// Shamelessly stolen and tweaked from core sqlx
pub async fn force_drop_database(
    to_delete: &PgConnectOptions,
    admin_db: &PgConnectOptions,
) -> Result<(), Error> {
    let mut conn = admin_db.connect().await?;
    let del_db_name = to_delete.get_database()
        .ok_or_else(|| Error::RowNotFound)?;

    let _ = conn
        .execute(&*format!(
            "DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)",
            del_db_name.replace('"', "\"\"")
        ))
        .await?;

    Ok(())
}


