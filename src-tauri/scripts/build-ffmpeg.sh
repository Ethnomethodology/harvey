#!/bin/bash
set -e

# This script builds FFmpeg from source for macOS, Linux, and Windows,
# creating shared libraries that are then bundled with the Tauri application.

# --- Platform-specific setup ---
OS="$(uname -s)"
ARCH=$(uname -m)
CONFIGURE_OPTS=""

check_linux_deps() {
    echo "Checking for Linux build dependencies..."
    for dep in yasm nasm pkg-config gcc; do
        if ! command -v $dep &> /dev/null; then
            echo "Error: $dep is not installed. Please install it using your package manager."
            echo "On Debian/Ubuntu: sudo apt-get install $dep zlib1g-dev"
            exit 1
        fi
    done
    echo "All Linux dependencies are present."
}

check_macos_deps() {
    echo "Checking for macOS build dependencies..."
    for dep in yasm pkg-config nasm; do
        if ! brew list $dep &>/dev/null; then
            echo "$dep not found, installing with Homebrew..."
            brew install $dep
        else
            echo "$dep is already installed."
        fi
done
}

check_windows_deps() {
    echo "Checking for Windows build dependencies (MSYS2)..."
    for dep in make gcc yasm nasm pkg-config; do
        if ! command -v $dep &> /dev/null; then
            echo "Error: $dep is not installed. Please install it in your MSYS2 environment."
            echo "Example: pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-yasm mingw-w64-x86_64-nasm"
            exit 1
        fi
    done
    echo "All Windows dependencies are present."
}

if [ "$OS" = "Darwin" ]; then
    echo "Detected macOS."
    check_macos_deps
    if [ "$ARCH" = "arm64" ]; then CONFIGURE_OPTS="--arch=arm64 --enable-videotoolbox"; fi
    if [ "$ARCH" = "x86_64" ]; then CONFIGURE_OPTS="--arch=x86_64 --enable-videotoolbox"; fi
elif [ "$OS" = "Linux" ]; then
    echo "Detected Linux."
    # check_linux_deps
    if [ "$ARCH" = "aarch64" ]; then CONFIGURE_OPTS="--arch=aarch64 --enable-vfp"; fi
    if [ "$ARCH" = "x86_64" ]; then CONFIGURE_OPTS="--arch=x86_64"; fi
elif [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
    echo "Detected Windows (MSYS/MINGW)."
    check_windows_deps

    # Check if we are cross-compiling for arm64
    if [ "$TARGET_ARCH" = "arm64" ]; then
        echo "Setting up for Windows ARM64 cross-compilation..."
        CONFIGURE_OPTS="--target-os=win64 --arch=aarch64 --cc=clang --cxx=clang++ --ar=llvm-ar --ranlib=llvm-ranlib --enable-cross-compile --disable-asm"
    else
        echo "Setting up for Windows x64 native compilation..."
        CONFIGURE_OPTS="--target-os=win64 --disable-asm"
    fi
else
    echo "Unsupported operating system: $OS"
    exit 1
fi

# --- Main build logic ---

# Ensure we are in the correct directory (src-tauri)
cd "$(dirname "$0")/.."

TAURI_DIR=$(pwd)
FFMPEG_VERSION="5.1.4"
FFMPEG_SOURCE_DIR="${TAURI_DIR}/ffmpeg-src"
FFMPEG_TARBALL="${TAURI_DIR}/ffmpeg-${FFMPEG_VERSION}.tar.gz"
INSTALL_PREFIX="${TAURI_DIR}/binaries/ffmpeg"

# 1. Clean up previous builds and downloads
echo "Cleaning up previous build artifacts..."
rm -rf "${FFMPEG_SOURCE_DIR}"
rm -rf "${INSTALL_PREFIX}"
rm -f "${FFMPEG_TARBALL}"
rm -f "${TAURI_DIR}/ffmpeg-n5.1.4.tar.gz" # Explicitly remove old tarball
mkdir -p "${INSTALL_PREFIX}"

# 2. Download and Extract FFmpeg
echo "Downloading FFmpeg ${FFMPEG_VERSION}..."
curl --retry 3 --retry-delay 5 -L "https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.gz" -o "${FFMPEG_TARBALL}"

echo "Extracting FFmpeg source..."
tar -xzf "${FFMPEG_TARBALL}" -C "${TAURI_DIR}"
mv "${TAURI_DIR}/ffmpeg-${FFMPEG_VERSION}" "${FFMPEG_SOURCE_DIR}"

# 3. Configure and Build FFmpeg
cd "${FFMPEG_SOURCE_DIR}"
echo "Configuring FFmpeg for a shared build..."

CPU_COUNT=$(nproc 2>/dev/null || sysctl -n hw.logicalcpu)

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
  --enable-iconv

echo "Building FFmpeg (using $CPU_COUNT cores)..."
make -j${CPU_COUNT}
make install

# 4. Clean up source and tarball
echo "Build complete. Cleaning up source files..."
rm -rf "${FFMPEG_SOURCE_DIR}"
rm -f "${FFMPEG_TARBALL}"

echo "FFmpeg shared libraries have been built and installed to ${INSTALL_PREFIX}"