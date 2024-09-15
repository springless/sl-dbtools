#!/usr/bin/env bash

USAGE="\
Usage: migrate-db -u CONNECTION_URI -t TARGET_MIGRATION -d MIGRATIO_FOLDER
Migrates the database to the specified target, running any migrations that
have not yet been run on the database, or reverting migrations if the target
is earlier than the current DB version. If no target is provided it will just
run to the latest migration file.

  -u, --uri URI
      URI string used to connect to the database. This should be in a form
      similar to: postgres://user:pass@host:5432/dbname.
  -t, --target TARGET
      The target migration version. Migration files should be in alphabetical
      order according to the sequence in which they should be run, and should
      be named in the format: VERSION.up.sql for an upgrade migration, and
      VERSION.dn.sql for a downgrade migration. If the target is after the
      current version of the database, then it will attempt to run each upgrade
      migration between the current version and the target. If the target is
      before the current version of the database, then it will run the downgrade
      migrations between the current version and the target.

      VERSION names can be any valid filename, but in order to ensure order it
      can be helpful to prefix them with the current datetime, eg.
      202409142309-add_user_table.up.sql. Alternatively you can number each
      migration, or any other naming scheme that will yield an alphabetical
      order to the files. The only requirement is the .up.sql or .dn.sql, based
      on whether it is an upgrade or downgrade file.

      A version must have a .up.sql file, but the .dn.sql file may be omitted if
      it is a no-op, or if you are not interested in providing downgrade
      functionality. It might still be good practice to include it, however, just
      to be explicit.

      When specifying the target version, only use the part of the filename prior
      to the .up.sql or .dn.sql. You may also use any unique string in the VERSION.
      eg. given the migration file:

      202409142309-add_user_table.up.sql

      You may specify this version with \"20240914\", \"add_user_table\",
      \"09-add_user\", etc. So long as that uniquely identifies the version.

      Alternatively, -3 will downgrade 3 versions from the current version,
      +3 will upgrade that many versions, and HEAD will upgrade to the most
      recent version.
  -d, --directory DIR
      Migration file directory location.
  -h, --help
      Show this information
"


db_uri=
target_version=
migration_folder=

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -u|--uri)
        shift
        db_uri="$1"
        shift
        ;;
      -t|--target)
        shift
        target_version="$1"
        shift
        ;;
      -d|--directory)
        shift
        # remove trailing slash if it exists
        migration_folder="${1%/}"
        shift
        ;;
      -h|--help)
        echo "$USAGE"
        shift
        exit 0
    esac
  done
  # Check to make sure the necessary variables are set. If not, then exit with an error
  # ${var+x} expands to "x" if the variable is set, and nothing otherwise
  if [ -z ${db_uri} ]; then
    echo "ERROR: Please set -u"
    echo
    echo "$USAGE"
    exit 1
  fi
  if [ -z ${target_version} ]; then
    echo "ERROR: Please set -t"
    echo
    echo "$USAGE"
    exit 1
  fi
  if [ -z ${migration_folder} ]; then
    echo "ERROR: Please set -d"
    echo
    echo "$USAGE"
    exit 1
  fi
}

get_all_versions() {
  up_versions=$(ls ${migration_folder}/*.up.sql)
  dn_versions=$(ls ${migration_folder}/*.dn.sql)
  echo "Up versions: $up_versions"
  echo "Dn versions: $dn_versions"
}

run() {
  parse_args "$@"
  echo "URI:    ${db_uri}"
  echo "Target: ${target_version}"
  echo "Folder: ${migration_folder}"
  get_all_versions
}

run "$@"
