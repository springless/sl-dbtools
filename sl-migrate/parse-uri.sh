#!/usr/bin/env bash

# Parses out the component parts of a URI. Given the URI:
# postgres://user:pass@localhost:5432/db
# This will parse out to:
# scheme = `postgres`
# username = `user`
# password = `pass`
# host = `localhost`
# port = `5432`
# resource = `db`

# Possible components:
# - scheme
# - username
# - pasasword
# - host
# - port
# - resource
REQUESTED_COMPONENT=""
URI=""

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -s|--scheme)
        REQUESTED_COMPONENT="scheme"
        shift
        ;;
      -u|--username)
        REQUESTED_COMPONENT="username"
        shift
        ;;
      -P|--password)
        REQUESTED_COMPONENT="password"
        shift
        ;;
      -h|--host)
        REQUESTED_COMPONENT="host"
        shift
        ;;
      -p|--port)
        REQUESTED_COMPONENT="port"
        shift
        ;;
      -r|--resource)
        REQUESTED_COMPONENT="resource"
        shift
        ;;
      *) # positional arguments
        URI="$1"
        shift
        ;;
    esac
  done
}

# Regex for URI. Bash doesn't seem to have non-capturing groups so
# we just need to keep track of all of them
# 1 - scheme
# 3 - user
# 5 - pass
# 6 - host
# 8 - port
# 11 - path
URI_REGEX="^([^:]+)://(([^:@]+)(:([^@]+))?@)?([^:/]+)(:([^/]+))?((/(.*))|$)"

# START ACTUAL PROGRAM

parse_args "$@"

if [[ $URI =~ $URI_REGEX ]]; then
  SCHEME=${BASH_REMATCH[1]}
  USER=${BASH_REMATCH[3]}
  PASS=${BASH_REMATCH[5]}
  HOST=${BASH_REMATCH[6]}
  PORT=${BASH_REMATCH[8]}
  RESOURCE=${BASH_REMATCH[11]}
fi

case "$REQUESTED_COMPONENT" in
  scheme)
    echo $SCHEME
    ;;
  username)
    echo $USER
    ;;
  password)
    echo $PASS
    ;;
  host)
    echo $HOST
    ;;
  port)
    echo $PORT
    ;;
  path)
    echo $PATH
    ;;
  resource)
    echo $RESOURCE
    ;;
esac
