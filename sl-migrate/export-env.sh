#!/usr/bin/env bash

# Sources a file and exports all of the variables defined inside of it
source_file() {
  if [ -f "$1" ]; then
    set -a
    source "$1"
    set +a
  else
    echo "Warning: File '$1' not found."
  fi
}

if [ $# -eq 0 ]; then
  # If no arguments are passed then source the `.env` file in the working directory if it exists
  source_file ".env"
else
  # Otherwise read each argument as a file to export
  for file in "$@"; do
    source_file "$file"
  done
fi
