#!/bin/bash

# --- CONFIG ---
OLD_NAME="esp32-fx_v2"
NEW_NAME="esp32-fx"
SRC_DIR="./pcb_v2"    # path to the old project folder
DST_DIR="./pcb"    # path for the new project folder


# --- STEP 2: Rename files containing the old project name ---
find "$DST_DIR" -type f -name "*$OLD_NAME*" | while read FILE; do
    NEW_FILE=$(echo "$FILE" | sed "s/$OLD_NAME/$NEW_NAME/g")
    mv "$FILE" "$NEW_FILE"
done

# --- STEP 3: Replace old project name inside text files ---
# Only applies to text-based KiCad files
find "$DST_DIR" -type f -name "*" | while read FILE; do
    if file "$FILE" | grep -q text; then
        sed -i "s/$OLD_NAME/$NEW_NAME/g" "$FILE"
    fi
done

echo "Project copied and renamed successfully from $OLD_NAME to $NEW_NAME."

