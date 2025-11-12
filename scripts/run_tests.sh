#!/bin/bash

echo "Starting Oracle XE..."
docker-compose up -d oracle-xe

echo "Waiting for Oracle to be healthy..."
until [ "$(docker inspect -f {{.State.Health.Status}} oracle-xe)" == "healthy" ]; do
    echo -n "."
    sleep 5
done
echo ""
echo "Oracle is ready!"

echo "Running integration tests..."
TEST_ORACLE_USER=system \
TEST_ORACLE_PASSWORD=Welcome1 \
TEST_ORACLE_CONNECT=localhost:1521/XE \
cargo test -- --ignored

TEST_RESULT=$?

echo "Stopping Oracle..."
docker-compose down

exit $TEST_RESULT
