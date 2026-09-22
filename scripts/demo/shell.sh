#!/bin/sh
# The shell inside the demo pane: fish with its own config and history, so
# nothing from the recording machine leaks into the video.
here="$(cd "$(dirname "$0")" && pwd)"
export XDG_CONFIG_HOME="$here/xdg"
export XDG_DATA_HOME="$here/xdg-data"
exec "${DEMO_FISH:-fish}" -i
