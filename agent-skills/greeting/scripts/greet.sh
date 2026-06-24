#!/usr/bin/env bash
# agent-skills/greeting/scripts/greet.sh

set -e

# Get current hour (0-23)
HOUR=$(date +%H)
USER_NAME="${USER:-friend}"

# Determine greeting based on hour
if [ "$HOUR" -ge 5 ] && [ "$HOUR" -lt 12 ]; then
    GREETING="Good morning"
elif [ "$HOUR" -ge 12 ] && [ "$HOUR" -lt 17 ]; then
    GREETING="Good afternoon"
elif [ "$HOUR" -ge 17 ] && [ "$HOUR" -lt 21 ]; then
    GREETING="Good evening"
else
    GREETING="Good night"
fi

# Get local time formatting
LOCAL_TIME=$(date "+%I:%M %p")

echo "${GREETING}, ${USER_NAME}! It is currently ${LOCAL_TIME}."
