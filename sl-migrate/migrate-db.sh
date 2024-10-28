#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

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
  -m, --migrationview TABLE_NAME
      Name of the table that should hold migration information.
  -h, --help
      Show this information
"


db_uri=${PROJECT_DB}
target_version=
migration_folder=${MIGRATION_DIR}
migration_view=${MIGRATION_VIEW_NAME}
up_versions=
dn_versions=
cur_version=

migration_path_direction=
migration_path=

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
      -m|--migrationview)
        shift
        migration_view="${1}"
        shift
        ;;
      -h|--help)
        echo "$USAGE"
        shift
        exit 0
        ;;
      *)
        echo "UNKNOWN OPTION: ${1}"
        shift
        ;;
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
  if [ -z ${migration_view} ]; then
    echo "ERROR: Please set -m"
    echo
    echo "$USAGE"
    exit 1
  fi
}

get_all_versions() {
  up_version_files=($(ls ${migration_folder}/*.up.sql))
  dn_version_files=($(ls ${migration_folder}/*.dn.sql))

  up_versions=()
  dn_versions=()

  for f in "${up_version_files[@]}"; do
    base_name=$(basename "${f}" .up.sql)
    up_versions+=("${base_name}")
  done
  for f in "${dn_version_files[@]}"; do
    base_name=$(basename "${f}" .dn.sql)
    dn_versions+=("${base_name}")
  done
}

get_current_version() {
  # -t (tuples only) -A (remove formatting)
  cur_version=$(psql "${db_uri}" -tA -c "
  SELECT version FROM \"${migration_view}\"
  ")
  echo "Cur version: $cur_version"
}

# Based on the current version and the target version will build an array of all of the
# migrations that will need to be run, along with whether these will be up or downgrades
get_migrate_path() {
  migration_path=()
  # we're somewhat relying on the fact that an unset string is less than a set string, here
  start_value=${cur_version}

  # handle special target values
  case "${target_version}" in
    HEAD)
      echo "TARGET IS HEAD"
      target_version="${up_versions[-1]}"
      ;;
    ROOT)
      target_version=""
      ;;
  esac
  end_value="${target_version}"

  # used to start and end the range of migrations to apply without having to write
  # separate logic for when the end is less than the start
  low_value=""
  high_value=""

  echo "Start Version: ${start_value}"
  echo "End version:   ${end_value}"
  echo "Cur version:   ${cur_version}"

  if [[ "${start_value}" < "${end_value}" ]]; then
    migration_path_direction="up"
    low_value="${start_value}"
    high_value="${end_value}"
  elif [[ "${end_value}" < "${start_value}" ]]; then
    migration_path_direction="dn"
    low_value="${end_value}"
    high_value="${start_value}"
  else
    # we are at the requested version
    return
  fi

  for v in "${up_versions[@]}"; do
    if [[ ! "${low_value}" > "${v}" && ! "${v}" > "${high_value}" && "${v}" != "${cur_version}" ]]; then
      migration_path+=("${v}")
    fi
  done

  # if we're downgrading then reverse the direction of the migration path
  if [[ "${migration_path_direction}" == "dn" ]]; then
    temp_migration_path=()
    len=${#migration_path[@]}
    for (( i=$len-1; i>=0; i-- )); do
      temp_migration_path+=("${migration_path[$i]}")
    done
    migration_path=("${temp_migration_path[@]}")
  fi

  echo "Migration path..."
  for f in "${migration_path[@]}"; do
    echo "    ${f}"
  done
}

run_migration_path() {
  migration_files=()
  for v in "${migration_path[@]}"; do
    migration_file="${migration_folder}/${v}.${migration_path_direction}.sql"
    if [ -f "${migration_file}" ]; then
      echo "Running migration: ${migration_file}"
      {
        echo "BEGIN;";
        cat "${migration_file}";
        # Update the version view
        echo ";"
        echo "CREATE OR REPLACE VIEW ${migration_view} AS"
        echo "SELECT '${v}'::TEXT AS version;"
        echo "COMMIT;";
      } | psql -v ON_ERROR_STOP=on "${db_uri}"
      exit_code=$?
      if [ $exit_code -ne 0 ]; then
        echo "Failed to apply migration: ${migration_file}"
        echo "psql failed with exit code: ${exit_code}"
        exit $exit_code
      fi
    else
      echo "Nothing to be done for: ${v}"
    fi

    cur_version="${v}"
  done
}

run() {
  parse_args "$@"
  echo "URI:    ${db_uri}"
  echo "Target: ${target_version}"
  echo "Folder: ${migration_folder}"
  echo "View:   ${migration_view}"
  get_all_versions
  get_current_version
  get_migrate_path
  run_migration_path
}

run "$@"
