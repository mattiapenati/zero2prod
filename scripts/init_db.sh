#!/bin/bash
set -x
set -eo pipefail

if ! command -v psql &> /dev/null
then
    >&2 echo "Error: `psql` is not installed."
    exit 1
fi

if ! command -v sqlx &> /dev/null
then
    >&2 echo "Error: `sqlx` is not installed."
    >&2 echo "Use the following command to install it:"
    >&2 echo "    cargo install sqlx-cli --no-default-features --features postgres"
    exit 1
fi

DB_USER="${POSTGRES_USER:=postgres}"
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_PORT="${POSTGRES_PORT:=5432}"

if [ "${SKIP_DOCKER}" != "yes" ]
then
    docker run \
        -e POSTGRES_USER=${DB_USER} \
        -e POSTGRES_PASSWORD=${DB_PASSWORD} \
        -e POSTGRES_DB=${DB_NAME} \
        -p "${DB_PORT}":5432 \
        -d postgres:alpine \
        postgres -N 1000
fi

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -c '\q'
do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}"
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"
