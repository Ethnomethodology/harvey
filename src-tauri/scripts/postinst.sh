#!/bin/bash
# Post-installation script for Harvey .deb package

set -e

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if ffmpeg is installed
if command_exists ffmpeg; then
    echo "ffmpeg is already installed."
    exit 0
fi

echo "ffmpeg not found. Attempting to install..."

# Detect package manager and install ffmpeg
if command_exists apt-get; then
    echo "Using apt-get to install ffmpeg."
    sudo apt-get update
    sudo apt-get install -y ffmpeg
elif command_exists dnf; then
    echo "Using dnf to install ffmpeg."
    sudo dnf install -y ffmpeg
elif command_exists pacman; then
    echo "Using pacman to install ffmpeg."
    sudo pacman -Syu --noconfirm ffmpeg
else
    echo "Could not find a supported package manager (apt, dnf, pacman)."
    echo "Please install ffmpeg manually."
    # We exit with 0 to avoid failing the installation, as this is a soft dependency.
    exit 0
fi

echo "ffmpeg installation complete."