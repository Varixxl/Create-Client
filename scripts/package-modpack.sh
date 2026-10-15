#!/usr/bin/env bash
set -e

PROFILE_DIR="/home/varixxl/.local/share/ModrinthApp/profiles/Forge 1.20.1 (1)"
META_DIR="/home/varixxl/.local/share/ModrinthApp/meta"
OUTPUT_DIR="/home/varixxl/create-client/dist-modpack"
VERSION="${1:-1.0.0}"
ZIP_NAME="modpack-v${VERSION}.zip"

echo "=== Packaging Create Server Modpack v${VERSION} ==="

mkdir -p "$OUTPUT_DIR"
TEMP_BUILD=$(mktemp -d)
trap 'rm -rf "$TEMP_BUILD"' EXIT

echo "[1/4] Copying profile mods, configs, and assets..."
cp -r "$PROFILE_DIR/mods" "$TEMP_BUILD/"
cp -r "$PROFILE_DIR/config" "$TEMP_BUILD/"
[ -d "$PROFILE_DIR/defaultconfigs" ] && cp -r "$PROFILE_DIR/defaultconfigs" "$TEMP_BUILD/"
[ -d "$PROFILE_DIR/fancymenu_data" ] && cp -r "$PROFILE_DIR/fancymenu_data" "$TEMP_BUILD/"
[ -d "$PROFILE_DIR/kubejs" ] && cp -r "$PROFILE_DIR/kubejs" "$TEMP_BUILD/"
[ -d "$PROFILE_DIR/tacz" ] && cp -r "$PROFILE_DIR/tacz" "$TEMP_BUILD/"
[ -d "$PROFILE_DIR/resourcepacks" ] && cp -r "$PROFILE_DIR/resourcepacks" "$TEMP_BUILD/"
[ -d "$PROFILE_DIR/shaderpacks" ] && cp -r "$PROFILE_DIR/shaderpacks" "$TEMP_BUILD/"
[ -f "$PROFILE_DIR/servers.dat" ] && cp "$PROFILE_DIR/servers.dat" "$TEMP_BUILD/"

# Clean up unwanted temp files inside mods/config
rm -rf "$TEMP_BUILD/mods_backup" 2>/dev/null || true
rm -f "$TEMP_BUILD/"*.log 2>/dev/null || true

echo "[2/4] Bundling Forge 1.20.1 runtime and libraries..."
mkdir -p "$TEMP_BUILD/libraries"
if [ -d "$META_DIR/libraries" ]; then
    cp -r "$META_DIR/libraries/"* "$TEMP_BUILD/libraries/"
fi

# Copy client jar and version metadata
if [ -f "$META_DIR/versions/1.20.1-47.4.20/1.20.1-47.4.20.jar" ]; then
    cp "$META_DIR/versions/1.20.1-47.4.20/1.20.1-47.4.20.jar" "$TEMP_BUILD/1.20.1.jar"
fi
if [ -f "$META_DIR/versions/1.20.1-47.4.20/1.20.1-47.4.20.json" ]; then
    cp "$META_DIR/versions/1.20.1-47.4.20/1.20.1-47.4.20.json" "$TEMP_BUILD/version.json"
fi

echo "[3/4] Creating zip archive..."
DEST_ZIP="$OUTPUT_DIR/$ZIP_NAME"
rm -f "$DEST_ZIP"
(cd "$TEMP_BUILD" && zip -q -r "$DEST_ZIP" .)

SHA256_HASH=$(sha256sum "$DEST_ZIP" | awk '{print $1}')
FILE_SIZE=$(du -h "$DEST_ZIP" | awk '{print $1}')

echo "[4/4] Generating manifest template..."
cat <<EOF > "$OUTPUT_DIR/manifest.json"
{
  "version": "${VERSION}",
  "modpack_url": "https://github.com/YOUR_GITHUB_USERNAME/create-client/releases/download/v${VERSION}/${ZIP_NAME}",
  "sha256": "${SHA256_HASH}",
  "changelog": "Create Server Modpack release v${VERSION}."
}
EOF

echo ""
echo "=== Packaged Successfully! ==="
echo "Zip File:  $DEST_ZIP ($FILE_SIZE)"
echo "SHA256:    $SHA256_HASH"
echo "Manifest:  $OUTPUT_DIR/manifest.json"
