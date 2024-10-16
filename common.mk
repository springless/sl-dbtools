# Common includes and variables used in the target makefiles

# Load a .env file if it exists
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

# Variables to support acting on a project DB
MIGRATION_DB_USER := $(shell ./util/parse-uri.sh "$(MIGRATION_URL)" --username)
MIGRATION_DB_RESOURCE := $(shell ./util/parse-uri.sh "$(MIGRATION_URL)" --resource)

# Variables to support acting on an admin DB
ADMIN_DB := $(shell if [ -z "$(MIGRATION_ADMIN_URL)" ]; then echo "$(MIGRATION_URL)"; else echo "$(MIGRATION_ADMIN_URL)"; fi)
