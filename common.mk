# Common includes and variables used in the target makefiles

# Load a .env file if it exists
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

# Variables to support acting on a project DB
MIGRATION_DB_HOST := $(shell ./util/parse-uri.sh "$(DATABASE_URL)" --host)
MIGRATION_DB_PORT := $(shell port=$$(./scripts/parse-uri.sh "$(MIGRATION_URL)" --port); if [ -z "$$port" ]; then echo 5432; else echo $$port; fi)
MIGRATION_DB_USER := $(shell ./util/parse-uri.sh "$(MIGRATION_URL)" --username)
MIGRATION_DB_PASSWORD := $(shell ./util/parse-uri.sh "$(MIGRATION_URL)" --password)
MIGRATION_DB_RESOURCE := $(shell ./util/parse-uri.sh "$(MIGRATION_URL)" --resource)

# Variables to support acting on an admin DB
ADMIN_DB := $(shell if [ -z "$(MIGRATION_ADMIN_URL)" ]; then echo "$(MIGRATION_URL)"; else echo "$(MIGRATION_ADMIN_URL)"; fi)
