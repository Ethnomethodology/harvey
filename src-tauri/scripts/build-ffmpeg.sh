#!/bin/bash
set -e

# This script builds FFmpeg from source for macOS, creating shared libraries (.dylib)
# that are then bundled with the Tauri application.

# Ensure we are in the correct directory
cd "$(dirname "$0")/.."

TAURI_DIR=$(pwd)
FFMPEG_VERSION="5.1.4"
FFMPEG_SOURCE_DIR="${TAURI_DIR}/ffmpeg-src"
FFMPEG_TARBALL="${TAURI_DIR}/ffmpeg-${FFMPEG_VERSION}.tar.gz"
INSTALL_PREFIX="${TAURI_DIR}/binaries/ffmpeg"

# 1. Determine Architecture
ARCH=$(uname -m)
CONFIGURE_OPTS=""

if [ "$ARCH" = "arm64" ]; then
  echo "Detected ARM64 architecture."
  CONFIGURE_OPTS="--arch=arm64"
elif [ "$ARCH" = "x86_64" ]; then
  echo "Detected x86_64 architecture."
  CONFIGURE_OPTS="--arch=x86_64"
else
  echo "Unsupported architecture: $ARCH"
  exit 1
fi

# 2. Check for and install dependencies with Homebrew
echo "Checking for build dependencies..."
for dep in yasm pkg-config nasm; do
  if ! brew list $dep &>/dev/null; then
    echo "$dep not found, installing with Homebrew..."
    brew install $dep
  else
    echo "$dep is already installed."
  fi
done

# 3. Clean up previous builds and downloads
echo "Cleaning up previous build artifacts..."
rm -rf "${FFMPEG_SOURCE_DIR}"
rm -rf "${INSTALL_PREFIX}"
rm -f "${FFMPEG_TARBALL}"
rm -f "${TAURI_DIR}/ffmpeg-n5.1.4.tar.gz" # Explicitly remove old tarball
mkdir -p "${INSTALL_PREFIX}"

# 4. Download and Extract FFmpeg
echo "Downloading FFmpeg ${FFMPEG_VERSION}..."
curl -L "https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.gz" -o "${FFMPEG_TARBALL}"

echo "Extracting FFmpeg source..."
tar -xzf "${FFMPEG_TARBALL}" -C "${TAURI_DIR}"
mv "${TAURI_DIR}/ffmpeg-${FFMPEG_VERSION}" "${FFMPEG_SOURCE_DIR}"

# 5. Configure and Build FFmpeg
cd "${FFMPEG_SOURCE_DIR}"
echo "Configuring FFmpeg for a shared build..."

CPU_COUNT=$(sysctl -n hw.logicalcpu)

./configure \
  --prefix="${INSTALL_PREFIX}" \
  $CONFIGURE_OPTS \
  --enable-shared \
  --disable-static \
  --disable-gpl \
  --disable-nonfree \
  --disable-ffplay \
  --disable-ffprobe \
  --disable-doc \
  --enable-zlib \
  --enable-iconv \
  --enable-videotoolbox

echo "Building FFmpeg (using $CPU_COUNT cores)..."
make -j${CPU_COUNT}
make install

# 6. Clean up source and tarball
echo "Build complete. Cleaning up source files..."
rm -rf "${FFMPEG_SOURCE_DIR}"
rm -f "${FFMPEG_TARBALL}"

echo "FFmpeg shared libraries have been built and installed to ${INSTALL_PREFIX}"
