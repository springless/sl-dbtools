use sqlx::{postgres::{PgConnectOptions, PgDatabaseError}, ConnectOptions, Error, Executor};

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

/// Creates a database, given an admin connection, and assigns ownership to the
/// username provided in the base connection
/// Shamelessly stolen and tweaked from core sqlx
pub async fn create_owned_database(
    to_create: &PgConnectOptions,
    admin_db: &PgConnectOptions,
) -> Result<(), Error> {
    let mut conn = admin_db.connect().await?;
    let database = to_create.get_database()
        .ok_or_else(|| Error::RowNotFound)?;
    let owner = to_create.get_username();

    let _ = conn
        .execute(&*format!(
            "CREATE DATABASE \"{}\" OWNER \"{}\"",
            database.replace('"', "\"\""),
            owner.replace('"', "\"\""),
        ))
        .await?;

    Ok(())
}

/// Creates a database, given an admin connection, and assigns ownership to the
/// username provided in the base connection
/// Shamelessly stolen and tweaked from core sqlx
pub async fn create_owned_database_from_template(
    to_create: &PgConnectOptions,
    template: &PgConnectOptions,
    admin_db: &PgConnectOptions,
) -> Result<(), Error> {
    let mut conn = admin_db.connect().await?;
    let template_database = template.get_database()
        .ok_or_else(|| Error::RowNotFound)?;
    let database = to_create.get_database()
        .ok_or_else(|| Error::RowNotFound)?;
    let owner = to_create.get_username();

    let _ = conn
        .execute(&*format!(
            "CREATE DATABASE \"{}\" WITH TEMPLATE \"{}\" OWNER \"{}\"",
            database.replace('"', "\"\""),
            template_database.replace('"', "\"\""),
            owner.replace('"', "\"\""),
        ))
        .await?;

    Ok(())
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
