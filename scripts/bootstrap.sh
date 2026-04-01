#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print header
echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}   Harvey Build Bootstrapper v0.1.1    ${NC}"
echo -e "${BLUE}=======================================${NC}"

# 1. OS Detection
OS="$(uname -s)"
case "${OS}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    *)          MACHINE="UNKNOWN:${OS}"
esac

echo -e "Detected OS: ${GREEN}${MACHINE}${NC}"

# 2. Dependency Checking Functions
check_cmd() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}[MISSING]${NC} $1 is not installed."
        return 1
    else
        echo -e "${GREEN}[FOUND]${NC} $1 is installed."
        return 0
    fi
}

# 3. Check Base Tools
MISSING_DEPS=0
check_cmd "git" || MISSING_DEPS=1
check_cmd "node" || MISSING_DEPS=1
check_cmd "npm" || MISSING_DEPS=1
check_cmd "cargo" || MISSING_DEPS=1

if [ $MISSING_DEPS -eq 1 ]; then
    echo -e "\n${YELLOW}Some core dependencies are missing!${NC}"
    if [ "$MACHINE" == "Mac" ]; then
        echo -e "Please install them using Homebrew:"
        echo -e "  ${BLUE}brew install git node rustup${NC}"
        echo -e "  ${BLUE}rustup-init${NC} (and follow prompts)"
    elif [ "$MACHINE" == "Linux" ]; then
        echo -e "Please install them using your package manager (e.g. apt):"
        echo -e "  ${BLUE}sudo apt update && sudo apt install git nodejs npm curl${NC}"
        echo -e "  ${BLUE}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    fi
    echo -e "\nAfter installation, ${YELLOW}close and relaunch your terminal${NC} then run this script again."
    exit 1
fi

# 4. Linux-Specific Tauri Dependencies
if [ "$MACHINE" == "Linux" ]; then
    echo -e "\nChecking for Linux system libraries..."
    if ! command -v pkg-config &> /dev/null; then
        echo -e "${RED}[MISSING]${NC} pkg-config. Run: ${BLUE}sudo apt install pkg-config${NC}"
        exit 1
    fi

    # List of required packages to check via pkg-config
    # These match the dependencies in AGENTS.md
    LIBS=("webkit2gtk-4.1" "gtk+-3.0" "libsoup-3.0" "ayatana-appindicator3-0.1")
    for lib in "${LIBS[@]}"; do
        if pkg-config --exists "$lib"; then
            echo -e "${GREEN}[FOUND]${NC} $lib"
        else
            echo -e "${RED}[MISSING]${NC} $lib. Run: ${BLUE}sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libayatana-appindicator3-dev${NC}"
            exit 1
        fi
    done
fi

# 5. Build Preparation
echo -e "\n${BLUE}Environment ready!${NC}"
echo -e "This script will clone the 'main' branch and build it from source."
read -p "Enter destination directory for the source code [default: ~/harvey-source]: " DEST
DEST=${DEST:-$HOME/harvey-source}

if [ -d "$DEST" ]; then
    echo -e "${YELLOW}Warning:${NC} Directory $DEST already exists."
    read -p "Do you want to overwrite it? (y/N): " CONFIRM
    if [[ ! $CONFIRM =~ ^[Yy]$ ]]; then
        echo "Aborting."
        exit 0
    fi
    rm -rf "$DEST"
fi

# 6. Execution
echo -e "\n${BLUE}Cloning and Building Harvey...${NC}"
git clone https://github.com/Ethnomethodology/harvey.git "$DEST"
cd "$DEST"

echo -e "\n${BLUE}Installing dependencies...${NC}"
npm install

echo -e "\n${BLUE}Compiling Application (this may take several minutes)...${NC}"
npm run tauri build

echo -e "\n${GREEN}Success!${NC} The build is complete."
echo -e "You can find your executable in: ${BLUE}$DEST/src-tauri/target/release/bundle/...${NC}"
