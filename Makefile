# Retrieves the absolute path to this makefile, and the directory containing it.
# This value is difficult to override, so if you are placing this in other makefiles
# being included in this one, make sure they are namespaced appropriately
_MK_PATH := $(abspath $(lastword $(MAKEFILE_LIST)))
_MK_DIR := $(realpath $(dir $(_MK_PATH)))

# Include this snippet in your Makefile if you want to load any variables
# from a `.env` file
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

include ./sl-migrate/migrate.mk
