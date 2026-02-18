#!/bin/sh
set -eu

if [ $# -ne 3 ]; then
	echo "Usage: $0 <env-file> <username> <password>"
	exit 1
fi

ENV_FILE="$1"
USERNAME="$2"
PASSWORD="$3"

SERVER="http://$(grep '^SERVICE_CONTINUWUITY_BIND_PORT_CLIENT_API=' "${ENV_FILE}" | cut -d= -f2)"
REGISTER_URL="${SERVER}/_matrix/client/v3/register"

echo "Registering user '${USERNAME}' on ${SERVER}..."

SESSION_RESPONSE=$(curl -s -X POST "${REGISTER_URL}" \
	-H 'Content-Type: application/json' \
	-d "{\"username\": \"${USERNAME}\", \"password\": \"${PASSWORD}\"}")

SESSION_ID=$(echo "${SESSION_RESPONSE}" | grep -o '"session":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -z "${SESSION_ID}" ]; then
	echo "Error: Could not get session ID. Response: ${SESSION_RESPONSE}"
	exit 1
fi

# Determine the required auth flow from the server response.
# The first user requires m.login.registration_token (bootstrap token from logs).
# Subsequent users use m.login.dummy (open registration).
if echo "${SESSION_RESPONSE}" | grep -q 'm.login.registration_token'; then
	CONTAINER_ID=$(docker ps -q --filter name=baibot-continuwuity-continuwuity)
	REG_TOKEN=$(docker logs "${CONTAINER_ID}" 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep 'using the registration token' | grep -oP 'registration token \K[A-Za-z0-9]+' | head -1)
	AUTH_BODY="{\"type\": \"m.login.registration_token\", \"token\": \"${REG_TOKEN}\", \"session\": \"${SESSION_ID}\"}"
else
	AUTH_BODY="{\"type\": \"m.login.dummy\", \"session\": \"${SESSION_ID}\"}"
fi

RESULT=$(curl -s -X POST "${REGISTER_URL}" \
	-H 'Content-Type: application/json' \
	-d "{\"username\": \"${USERNAME}\", \"password\": \"${PASSWORD}\", \"auth\": ${AUTH_BODY}}")

if echo "${RESULT}" | grep -q '"user_id"'; then
	echo "Successfully registered user: $(echo "${RESULT}" | grep -o '"user_id":"[^"]*"' | cut -d'"' -f4)"
else
	echo "Registration failed. Response: ${RESULT}"
	exit 1
fi
