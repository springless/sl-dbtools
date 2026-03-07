# Springless Database Tools

These are just some simple utilities I've put together over time to help me out with database
tasks like migrations, tracking schemas, migrating test seed databases, backing up and
restoring databases, etc.

# Migrations

The philosophy behind managing migrations in the scripts and files of this system is pretty
straightforward:

You have a folder that contains raw SQL files representing each migration to a specific version
of the codebase. These files are in lexicographic order of version, and are postpended with
either `.up.sql` for an upgrade migration file, or `.dn.sql` for a downgrade migration file.

So, given a folder with the contents:

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

There are 4 different versions of the database here, so if you are starting from a new
database and attempt to migrate to the latest version, it will run:

```
- 01-create-user-table.up.sql
- 02-update-user-table.up.sql
- 03-clear-password.up.sql
- 04-remove-password.up.sql
```

Whenever a migration is applied successfully the system creates a view with the same name
as the `MIGRATION_VIEW_NAME` environment variable. This view returns a static value, which is
the version name for the current schema version. This name is everything up until the
postfix and extension of the `up` migration that we have just finished. For example,
if we updated to `HEAD` with the migrations above, the final version would be
`04-remove-password`.

If we then downgrade, for example to version `01-create-user-table`, it will run:

```
- 04-remove-password.dn.sql
- 02-update-user-table.dn.sql
```

You'll notice it skipped `03-clear-password`. This is just because that downgrade does not exist,
so it is considered a no-op. It's not strictly necessary to create the downgrade files, omitting
them will just obviously mean that rolling back changes will not be supported. You can reference
the sample migrations folder in the `tests` directory of this repository if you still need
more clarity on how this part of the system works, but there's not much more to it.

## Naming migration files

The sample migrations folder uses a simple number prefix on the filename to keep order, but
any filename is acceptable, just so long as sorting them alphabetically yields the correct
order in which to run the migrations.

### Collapsing migrations

Files that do not end with `.up.sql` or `.dn.sql` are ignored, so a simple way to collapse
migrations is to make a no-op up migration, output a version of the current schema, save it
in the migrations folder as `ROOT.sql`, and then delete the rest of the migrations.

So given:

```
- 01-create-user-table.up.sql
- 02-update-user-table.up.sql
- 03-clear-password.up.sql
- 04-remove-password.up.sql
```

Create a blank file (or just include a comment explaining it's a re-baseline):

```
- 01-create-user-table.up.sql
- 02-update-user-table.up.sql
- 03-clear-password.up.sql
- 04-remove-password.up.sql
- 05-new-baseline.up.sql
```

Migrate to that version, and then dump the entire schema into `ROOT.sql`. This will give you:

```
- 01-create-user-table.up.sql
- 02-update-user-table.up.sql
- 03-clear-password.up.sql
- 04-remove-password.up.sql
- 05-new-baseline.up.sql
- ROOT.sql
```

You can then delete the old migrations, leaving you with:

```
- 05-new-baseline.up.sql
- ROOT.sql
```

Anytime you create a new database, just make sure `ROOT.sql` is the first seed file run.
`sldb` already does this by default. Additionally, at this point if you migrate to the
`ROOT` target, the only action that will be taken is to remove the view tracking the
schema version.

As with any migration system, make sure to get all databases using the migrations onto the
new baseline prior to removing the migrations preceding it.

#### Manually changing version

Alternatively, if you needed to change the version name for whatever reason you are
always able to override the view that declares the current schema. For example, if
you wanted to rename your schema version to `00-initial-schema`:

```sql
CREATE OR REPLACE VIEW _schema_version AS
SELECT '00-initial-schema'::TEXT AS version;
```

## Why No Schema Change Detection

This does not do any automated migration generation or schema change detections, primarily
because that's a lot of work that has been done better by others than I would be able to
accomplish in my free time.

Rather, this methodology encourages manually writing what changes should be made
in migration files, and then using a schema dump to track and reference the actual full version
of the database schema. There are utilities provided to create those dumps.

If you'd rather not deal with raw SQL, or you'd like to have a declarative database schema in a
different language native to your codebase then these utilities probably won't help you much
there. I mostly just put this together so I could cut out some dependencies that I'd rather not
have to keep up to date or have caused me headaches in the past and just stay closer to bare
metal and avoid too much magic when it comes to the database.

# Bash Tools

## Installing
Using the bash scripts and makefiles requires copying the `sl-migrate` folder into your project
and including `migrate.mk` in your root Makefile (the folder does not have to be named
`sl-migrate`). This will expose all of the targets available in `migrate.mk`. By default,
migrate-mk will attempt to load a `.env` file in your root directory, so you can use that
file to set defaults and values for environment variables that should be seen and used
by the scripts.

```makefile
include ./sl-migrate/migrate.mk

# ... rest of the Makefile
```

In order to update, just copy over any changed files.

## System Requirements

The Bash tools rely heavily on some pretty common Linux utilities:

- `bash`
- `make`
- `psql`
- `pg_dump`
- `pg_restore`

