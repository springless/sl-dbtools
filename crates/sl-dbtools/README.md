# Springless DB Tools

This is a Rust command line utility for managing database migrations, and also contains a library for running ephemeral test databases for integration/end-to-end tests.

## CLI

The command line application can be installed from this repository:

```bash
cargo install --git https://github.com/springless/sl-dbtools
```

Or from a local checkout

```bash
cargo install --path .
```

Either of these will place the executable `sldb` on your command line. It can be invoked with the `-h` or `--help` flag to see a list of arguments.

### Configuration

`sldb` utilizes the nearest `.env` file it can find by recursing upward through the folder path. This expects to find a `DATABASE_URL`, which it will then use as the base connection string to the database.

It can also optionally be provided a `DATABASE_URL_ADMIN` value, which would then be used for any operations requiring a database to be destroyed/created. If `DATABASE_URL_ADMIN` is not provided, then any creation/destruction actions will attempt to connect to `postgres` with the same credentials as `DATABASE_URL` and perform the actions. Most of these can also be provided via command line flags.

| Environment Variable | Required | Flag | Purpose |
| --- | --- | --- | --- |
| `DATABASE_URL` | `yes` | `sldb -u`/`sldb --url` | The database connection string used to connect to the database under test. This will also be used by the test system as the "root" database name and connection values.
| `DATABASE_URL_ADMIN` | `no` | `sldb -a`/`sldb --admin-url` | A connection string with `CREATE`/`DROP` privileges. If this is not provided then `DATABASE_URL` will be used for the credentials, and it will attempt to use those to connect to the `postgres` database. You can specify a specific administrative database using this string, as well (such as `template1`) |
| `MIGRATION_DIR` | `yes` | `sldb migrate -d`/`sldb migrate --dir` | The folder that should be used for migrations. |
| `MIGRATION_VIEW_NAME` | `no` | `sldb migrate -v`/`sldb migrate --view-name` | The name of the view that should hold the current schema version. By default this will be `_schema_version` |

Inside the environment files it can accept variables, so if, for example, you already have the database connection string defined in the variable `DB_URL`, you can just reuse the same value without repeating yourself in the `.env` file like:

```env
DATABASE_URL=$(DB_URL)
```

## Migrations

The migration system expects a folder containing an alphanumerically-sortable list of files that will be run on the migration database. It accepts standard git-style migration targets. Given the migration directory:

```
MyMigrationFolder
| 01-create-user-table.dn.sql
| 01-create-user-table.up.sql
| 02-update-user-table.dn.sql
| 02-update-user-table.up.sql
| 03-clear-password.up.sql
| 04-remove-password.dn.sql
\ 04-remove-password.up.sql
```

Targets can be one of `HEAD`, `ROOT`, `@`, or a full or partial version name. After resolving the version, if the requested version is higher than the current version it will run every `up` migration until the target version, and if it is lower then it will run every `dn` migration until the target version. If the target is `ROOT` it will additionally remove the `_schema_version` database. `HEAD` targets the last version, `ROOT` targets the first version (or really "before the first version"), and `@` targets the current version. These can additionally be modified with `+` or `~`, where `+1` will target the version after the resolved version, and `~1` will target the version before the resolved version. For example, these targets resolve to the following versions:

| Target                 | Resolved Version       |
| ---------------------- | ---------------------- |
| `HEAD`                 | `04-remove-password`   |
| `HEAD~2`               | `02-update-user-table` |
| `ROOT`                 | (before first version) |
| `ROOT+1`               | `01-create-user-table` |
| `@`                    | Current version        |
| `@+1`                  | Migrate up one version |
| `02-update-user-table` | `02-update-user-table` |
| `03`                   | `03-clear-password`    |
| `rem`                  | `04-remove-password`   |

There are also two reserved filenames, predictably `ROOT.sql` and `HEAD.sql`. These are not directly used by the core of the migration system and do not need to be included (meaning they will not be run if included in the migration folder) but can be used to store both the database schema prior to any migratios (`ROOT.sql`) and the database at the end of the migration chain (`HEAD.sql`). There are only two instances where these files are utilized:

1. `ROOT.sql` will be run as the first file after creating the database when you tell the migration runner to migrate from a new database. It is not referenced at all when migrating to `ROOT`. If this file is not in the migration folder, then it just runs the first `up` migration on an empty database.
2. `HEAD.sql` will be written to when you ask the the `sldb dump` command to dump or migrate the schema without providing a file argument. It is also not referenced at all when migrating to `HEAD`.

Besides those cases these files are ignored entirely by the migration system.

# Requirements

## `pg_dump`

`pg_dump` is required only if you are performing an action that dumps the contents of a Postgresql database (schema, data, or schema+data). It should be whatever version is required to interact with the version of Postgresql that you are using and should be on your `PATH`.

If you are not performing any Postgresql database dumping actions then it is not needed.
