include common.mk

RUN_PG_DUMP := PGPASSWORD=$(POSTGRES_PASS) pg_dump \
							 -U $(POSTGRES_USER) \
							 -h $(POSTGRES_HOST) \
							 -d $(POSTGRES_DB)

# Apply to output of pg_dump commands to strip SET commands, pg_dump comments (not db comments),
# and excess newlines
CLEANUP_PG_DUMP := sed '/^SET/d;/^--/d;' | sed '/^$$/N;/^\n$$/D'

###
# Migration management
###

create-db:
	psql $(ADMIN_DB) \
		-c "CREATE DATABASE \"$(MIGRATION_DB_RESOURCE)\" OWNER \"$(MIGRATION_DB_USER)\";"

drop-db:
	-psql $(ADMIN_DB) \
		-c "DROP DATABASE \"$(MIGRATION_DB_RESOURCE)\";"

migrate-db-head:
	./util/migrate-db.sh \
		--uri $(MIGRATION_URL) \
		--target HEAD \
		--directory $(MIGRATION_DIR) \
		--migrationtable $(MIGRATION_TABLE)

seed-db:
	psql $(MIGRATION_URL) \
		-f $(MIGRATION_SEED_FILE)


# Resets the database to a completely empty state and runs all migrations
reset-db-migrate: drop-db create-db migrate-db-head

# Resets the database to match the provided seed file
reset-db-seed: drop-db create-db seed-db

###
# Schema visualization
###

## Several ways to dump data and schemas with pg_dump
# Dumps the database schema with no data
dump-schema:
	pg_dump $(MIGRATION_URL) \
		--schema-only \
		--no-owner \
		--no-privileges \
		| $(CLEANUP_PG_DUMP) \
		> $(SCHEMA_FILE)

# Dumps a backup that uses INSERTs to recreate data
dump-data-insert:
	pg_dump $(MIGRATION_URL) \
		--rows-per-insert=1000 \
		--column-inserts \
		--data-only \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(DATA_FILE)

# Dumps a backup that uses COPY to recreate data
dump-data-copy:
	pg_dump $(MIGRATION_URL) \
		--data-only \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(DATA_FILE)

# Dumps the entire schema + data using INSERT statements
dump-full-insert:
	pg_dump $(MIGRATION_URL) \
		--rows-per-insert=1000 \
		--column-inserts \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(SCHEMA_DATA_FILE)

# Dumps a full backup that uses COPY to recreate data
dump-full-copy:
	pg_dump $(MIGRATION_URL) \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(SCHEMA_DATA_FILE)

dump-data: dump-data-copy

# Schemaspy is a java utility that generates a static site of database tables, relations,
# and other metadata in an easy to browse and easy to host manner. This runs it through
# docker, and is specifically for a postgres database
schemaspy:
	# If docker creates this folder then nobody gets to write to it
	-mkdir -p $(SCHEMASPY_OUTPUT_DIR)
	-chmod 777 $(SCHEMASPY_OUTPUT_DIR)
	docker run --network host -it \
		-v $(SCHEMASPY_OUTPUT_DIR):/output \
		schemaspy/schemaspy:latest \
		-vizjs \
		-t pgsql11 \
		-u $(MIGRATION_DB_USER) \
		-p $(MIGRATION_DB_PASSWORD) \
		-host $(MIGRATION_DB_HOST) \
		-port $(MIGRATION_DB_PORT) \
		-db $(MIGRATION_DB_RESOURCE) \
		-s public


