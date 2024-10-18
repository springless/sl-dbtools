# Include this snippet in your Makefile if you want to load any variables
# from a `.env` file
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

include ./sl-migrate/migrate.mk
