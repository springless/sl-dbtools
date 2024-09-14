# Common includes and variables used in the target makefiles

# Load a .env file if it exists
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

# Variables to support acting on a project DB
PROJECT_DB_USER := $(shell ./util/parse-uri.sh "$(PROJECT_DB)" --username)
PROJECT_DB_RESOURCE := $(shell ./util/parse-uri.sh "$(PROJECT_DB)" --resource)

# Variables to support acting on an admin DB
