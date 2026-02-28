#!/bin/bash
# Launch all driftwm dashboard widgets as Alacritty terminals.
# Each gets its own app_id for window rule matching.

DIR="$(cd "$(dirname "$0")" && pwd)"

launch() {
    local name="$1" cols="$2" lines="$3" script="$4"
    alacritty --class "drift-${name}" \
        -o "window.dimensions.columns=${cols}" \
        -o "window.dimensions.lines=${lines}" \
        -o "window.padding.x=8" \
        -o "window.padding.y=8" \
        -o "window.decorations=\"None\"" \
        -e uv run --project "$DIR" python "$DIR/${script}" &
}

launch main      35 19  main_pane.py

wait
