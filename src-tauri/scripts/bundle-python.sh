#!/bin/bash
set -e

# This script downloads and unpacks a self-contained, pre-compiled Python build
# by querying the GitHub API to find the correct download URL.

# --- Configuration ---
PYTHON_VERSION="3.12.12"
PYTHON_BUILD_TAG="20251010"

# --- Platform Detection ---
OS="$(uname -s)"
ARCH=$(uname -m)
TARGET_PLATFORM=""

if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then TARGET_PLATFORM="aarch64-apple-darwin"; fi
    if [ "$ARCH" = "x86_64" ]; then TARGET_PLATFORM="x86_64-apple-darwin"; fi
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "aarch64" ]; then TARGET_PLATFORM="aarch64-unknown-linux-gnu"; fi
    if [ "$ARCH" = "x86_64" ]; then TARGET_PLATFORM="x86_64-unknown-linux-gnu"; fi
elif [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
    # In the CI environment, TARGET_ARCH is set. Locally, we fall back to uname.
    CURRENT_ARCH=${TARGET_ARCH:-$(uname -m)}
    if [ "$CURRENT_ARCH" = "x86_64" ]; then
        TARGET_PLATFORM="x86_64-pc-windows-msvc"
    elif [ "$CURRENT_ARCH" = "arm64" ] || [ "$CURRENT_ARCH" = "aarch64" ]; then
        TARGET_PLATFORM="aarch64-pc-windows-msvc"
    fi
fi

if [ -z "$TARGET_PLATFORM" ]; then
    echo "[ERROR] bundle-python.sh: Unsupported platform: ${OS} ${ARCH}"
    exit 1
fi

# --- Main build logic ---
TAURI_DIR=$(pwd)

PYTHON_DIR="${TAURI_DIR}/python"
TMP_TAR_PATH="${TAURI_DIR}/python-bundle.tar.gz"

# 1. Find Download URL via GitHub API
API_URL="https://api.github.com/repos/astral-sh/python-build-standalone/releases/tags/${PYTHON_BUILD_TAG}"

echo "[INFO] Querying GitHub API for Python ${PYTHON_VERSION} for ${TARGET_PLATFORM}..."

# Use a chain of simple, case-insensitive greps to find the correct URL.
# This is more robust than a single complex regex.
API_CURL_ARGS=("--retry" "3" "--retry-delay" "5" "-sL")
if [ -n "$GITHUB_TOKEN" ]; then
    API_CURL_ARGS+=("-H" "Authorization: Bearer $GITHUB_TOKEN")
fi

DOWNLOAD_URL=$(curl "${API_CURL_ARGS[@]}" "${API_URL}" | \
    grep 'browser_download_url' | \
    grep -i "cpython-${PYTHON_VERSION}" | \
    grep -i "${TARGET_PLATFORM}-install_only.tar.gz" | \
    cut -d '"' -f 4 | \
    head -n 1)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "[ERROR] bundle-python.sh: Could not find a matching Python bundle URL from the GitHub API."
    exit 1
fi

# 2. Clean up previous builds
rm -rf "${PYTHON_DIR}"
rm -f "${TMP_TAR_PATH}"

# 3. Download and Extract Python
echo "[INFO] Found download URL: ${DOWNLOAD_URL}"
echo "[INFO] Downloading Python bundle..."
curl --retry 3 --retry-delay 5 -L "${DOWNLOAD_URL}" -o "${TMP_TAR_PATH}"

echo "[INFO] Extracting Python bundle..."
tar -xzf "${TMP_TAR_PATH}" -C "${TAURI_DIR}"

# 4. Clean up downloaded tarball
rm -f "${TMP_TAR_PATH}"

echo "[INFO] Self-contained Python has been bundled into ${PYTHON_DIR}"
