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
| 00-create-version-table.dn.sql
| 00-create-version-table.up.sql
| 01-create-user-table.dn.sql
| 01-create-user-table.up.sql
| 02-update-user-table.dn.sql
| 02-update-user-table.up.sql
| 03-clear-password.up.sql
| 04-remove-password.dn.sql
\ 04-remove-password.up.sql
```

There are 5 different versions of the database here, so if you are starting from a new
database and attempt to migrate to the latest version, it will run:

```
- 00-create-version-table.up.sql
- 01-create-user-table.up.sql
- 02-update-user-table.up.sql
- 03-clear-password.up.sql
- 04-remove-password.up.sql
```

The first migration has to include the version table. It can do other things, but after each
migration it writes the current version into the version table. By default this table is called
`sl_migration`, but this can be changed via environment variable or passing options to the
command line utilities. The version table should only ever have one value in it, and that
value will be the version name for the current schema version. For example, if we updated to
`HEAD` with the migration table above, the final version should be `04-remove-password`.

If we then downgrade, for instance to version `01-create-user-table`, it will run:

```
- 04-remove-password.dn.sql
- 02-update-user-table.dn.sql
```

You'll notice it skipped `03-clear-password`. This is just because that downgrade does not exist,
so it is considered a no-op. It's not strictly necessary to use the downgrade files, omitting
them will just obviously mean that rolling back changes will not be supported. You can reference
the sample migrations folder in the `tests` directory of this repository if you still need
more clarity on how this part of the system works, but there's not much more to it.

## Naming migration files

The sample migrations folder uses a simple number prefix on the filename to keep order, but
any filename is acceptable, just so long as sorting them alphabetically yields the correct
order in which to run the migrations. If the migrations folder is getting too cluttered and you
would like to clean it up, you can manually combine migrations, or just change the first file
to a full schema dump, and either keep the name of the file the same as the latest version,
or rename the file and then update the value in the migration version table to match the new
version name. For instance if I wanted to condense the above migrations, I could take a full
schema dump and place it in a file called `04-remove-password.up.sql`, or place it in a file
called something like `00-initial-schema.up.sql` and then manually run:

```sql
UPDATE TABLE "sl_migration"
SET "version" = '00-initial-schema';
```

And then the system would continue as normal.

## Why No Automated Migrations

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

## System Requirements

The Bash tools rely heavily on some pretty common Linux utilities:

- `bash`
- `make`
- `psql`
- `pg_dump`
- `pg_restore`

