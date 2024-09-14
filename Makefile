include common.mk

RUN_PG_DUMP := PGPASSWORD=$(POSTGRES_PASS) pg_dump \
							 -U $(POSTGRES_USER) \
							 -h $(POSTGRES_HOST) \
							 -d $(POSTGRES_DB)

# Apply to output of pg_dump commands to strip SET commands, pg_dump comments (not db comments),
# and excess newlines
CLEANUP_PG_DUMP := sed '/^SET/d;/^--/d;' | sed '/^$$/N;/^\n$$/D'

print-scheme:
	echo $(PROJECT_DB_SCHEME)

# Dumps the database schema with no data
dump-schema:
	pg_dump $(PROJECT_DB) \
		--schema-only \
		--no-owner \
		--no-privileges \
		| $(CLEANUP_PG_DUMP) \
		> $(SCHEMA_FILE)

# Dumps a backup that uses INSERTs to recreate data
dump-data-insert:
	pg_dump $(PROJECT_DB) \
		--rows-per-insert=1000 \
		--column-inserts \
		--data-only \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(DATA_FILE)

# Dumps a backup that uses COPY to recreate data
dump-data-copy:
	pg_dump $(PROJECT_DB) \
		--data-only \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(DATA_FILE)

# Dumps the entire schema + data using INSERT statements
dump-full-insert:
	pg_dump $(PROJECT_DB) \
		--rows-per-insert=1000 \
		--column-inserts \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(SCHEMA_DATA_FILE)

# Dumps a full backup that uses COPY to recreate data
dump-full-copy:
	pg_dump $(PROJECT_DB) \
		--quote-all-identifiers \
		| $(CLEANUP_PG_DUMP) \
		> $(SCHEMA_DATA_FILE)

dump-data: dump-data-copy

create-db:
	psql $(ADMIN_DB) \
		-c "CREATE DATABASE \"$(PROJECT_DB_RESOURCE)\" OWNER \"$(PROJECT_DB_USER)\";"

drop-db:
	psql $(ADMIN_DB) \
		-c "DROP DATABASE \"$(PROJECT_DB_RESOURCE)\";"

