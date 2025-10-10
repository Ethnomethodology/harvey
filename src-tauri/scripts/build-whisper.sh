#!/bin/bash
set -e

# This script builds whisper.cpp (whisper-cli and whisper-stream) from source
# for macOS, Linux, and Windows, creating executables that are then bundled
# with the Tauri application.

# --- Platform-specific setup ---
OS="$(uname -s)"
ARCH=$(uname -m)
CPU_COUNT=$(nproc 2>/dev/null || sysctl -n hw.logicalcpu)

TARGET_TRIPLE=""
if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        TARGET_TRIPLE="aarch64-apple-darwin"
    elif [ "$ARCH" = "x86_64" ]; then
        TARGET_TRIPLE="x86_64-apple-darwin"
    fi
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "aarch64" ]; then
        TARGET_TRIPLE="aarch64-unknown-linux-gnu"
    elif [ "$ARCH" = "x86_64" ]; then
        TARGET_TRIPLE="x86_64-unknown-linux-gnu"
    fi
elif [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
    if [ "$ARCH" = "arm64" ]; then # Assuming MSYS2 CLANGARM64 environment
        TARGET_TRIPLE="aarch64-pc-windows-msvc"
    elif [ "$ARCH" = "x86_64" ]; then # Assuming MSYS2 CLANG64 environment
        TARGET_TRIPLE="x86_64-pc-windows-msvc"
    fi
fi

CMAKE_COMMON_FLAGS="-DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DWHISPER_SDL2=ON -DBUILD_EXAMPLES=OFF"
SDL2_DIR_ENV="" # To store the path to SDL2 installation

# Ensure we are in the correct directory (src-tauri)
cd "$(dirname "$0")/.."

TAURI_DIR=$(pwd)
WHISPER_CPP_VERSION="v1.6.2" # As per GitHub workflow
WHISPER_CPP_REPO="https://github.com/ggerganov/whisper.cpp.git"
WHISPER_CPP_SOURCE_DIR="${TAURI_DIR}/whisper.cpp-src"
INSTALL_PREFIX="${TAURI_DIR}/binaries/whisper.cpp"
SDL2_INSTALL_PATH="${TAURI_DIR}/third_party_libs/SDL2" # Centralized SDL2 install path

# --- Helper functions for dependency checks and installations ---

check_and_install_macos_deps() {
    echo "Checking for macOS build dependencies..."
    if ! command -v brew &> /dev/null; then
        echo "Homebrew not found. Please install Homebrew first: /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        exit 1
    fi
    for dep in cmake; do
        if ! brew list $dep &>/dev/null; then
            echo "$dep not found, installing with Homebrew..."
            brew install $dep
        else
            echo "$dep is already installed."
        fi
    done
    # SDL2 is built from source for consistency and static linking
    echo "CMake and other dependencies are present."
}

check_and_install_linux_deps() {
    echo "Checking for Linux build dependencies..."
    sudo apt-get update -y
    sudo apt-get install -y build-essential cmake curl
    echo "All Linux dependencies are present."
}

check_and_install_windows_deps() {
    echo "Checking for Windows build dependencies (MSYS2)..."
    # Assuming MSYS2 environment is already set up with pacman
    # The user will need to ensure these are installed in their MSYS2 environment
    echo "Please ensure 'make', 'cmake', and a mingw-w64-clang toolchain (e.g., mingw-w64-clang-x86_64-toolchain) are installed in your MSYS2 environment."
    echo "Example: pacman -S make cmake mingw-w64-clang-x86_64-toolchain mingw-w64-clang-x86_64-pkg-config mingw-w64-clang-x86_64-zlib mingw-w64-clang-x86_64-libiconv"
    # We can't automatically install via pacman from a generic bash script without knowing the msystem
    # So, we'll just check for existence of cmake and make
    for dep in make cmake; do
        if ! command -v $dep &> /dev/null; then
            echo "Error: $dep is not installed. Please install it in your MSYS2 environment."
            exit 1
        fi
    done
    echo "Basic Windows (MSYS2) dependencies are present."
}

# --- Build SDL2 from source ---
build_sdl2() {
    local SDL2_VERSION="2.30.5"
    local SDL2_SOURCE_DIR="${TAURI_DIR}/SDL2-src"
    local CMAKE_SDL2_FLAGS=""

    echo "Cleaning up previous SDL2 build artifacts..."
    rm -rf "${SDL2_SOURCE_DIR}"
    rm -rf "${SDL2_INSTALL_PATH}"
    mkdir -p "${SDL2_INSTALL_PATH}"

    echo "Downloading SDL2 source ${SDL2_VERSION}..."
    curl --retry 3 --retry-delay 5 -sL "https://github.com/libsdl-org/SDL/releases/download/release-${SDL2_VERSION}/SDL2-${SDL2_VERSION}.tar.gz" | tar xz -C "${TAURI_DIR}"
    mv "${TAURI_DIR}/SDL2-${SDL2_VERSION}" "${SDL2_SOURCE_DIR}"

    cd "${SDL2_SOURCE_DIR}"
    mkdir -p build && cd build

    if [ "$OS" = "Darwin" ]; then
        echo "Configuring and building SDL2 for macOS..."
        CMAKE_SDL2_FLAGS="-DCMAKE_INSTALL_PREFIX=${SDL2_INSTALL_PATH} -DSDL_STATIC=ON -DSDL_SHARED=OFF"
        if [ "$ARCH" = "arm64" ]; then
            CMAKE_SDL2_FLAGS+=" -DCMAKE_OSX_ARCHITECTURES=arm64"
        elif [ "$ARCH" = "x86_64" ]; then
            CMAKE_SDL2_FLAGS+=" -DCMAKE_OSX_ARCHITECTURES=x86_64"
        fi
        cmake .. ${CMAKE_SDL2_FLAGS}
        make -j"${CPU_COUNT}"
        make install
    elif [ "$OS" = "Linux" ]; then
        if [ "$ARCH" = "aarch64" ]; then
            echo "Configuring and building SDL2 for Linux aarch64 (cross-compilation)..."
            # Assuming TOOLCHAIN_DIR is set for cross-compilation
            if [ -z "$TOOLCHAIN_DIR" ]; then
                echo "Error: TOOLCHAIN_DIR not set for aarch64 cross-compilation."
                exit 1
            fi
            CMAKE_SDL2_FLAGS="-DCMAKE_C_COMPILER=${TOOLCHAIN_DIR}/bin/aarch64-none-linux-gnu-gcc -DCMAKE_CXX_COMPILER=${TOOLCHAIN_DIR}/bin/aarch64-none-linux-gnu-g++ -DCMAKE_INSTALL_PREFIX=${SDL2_INSTALL_PATH} -DSDL_STATIC=ON -DSDL_SHARED=OFF"
        else # x86_64
            echo "Configuring and building SDL2 for Linux x86_64..."
            CMAKE_SDL2_FLAGS="-DCMAKE_INSTALL_PREFIX=${SDL2_INSTALL_PATH} -DSDL_STATIC=ON -DSDL_SHARED=OFF"
        fi
        cmake .. ${CMAKE_SDL2_FLAGS}
        make -j"${CPU_COUNT}"
        make install
    elif [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
        echo "Configuring and building SDL2 for Windows (MSYS/MINGW)..."
        # For Windows, we need to ensure the correct toolchain is picked up by cmake
        # The GitHub workflow uses -G "Unix Makefiles" and relies on the MSYS2 environment
        # We'll try to replicate that.
        CMAKE_SDL2_FLAGS="-DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=${SDL2_INSTALL_PATH} -DSDL_STATIC=ON -DSDL_SHARED=OFF -G \"Unix Makefiles\""
        cmake .. ${CMAKE_SDL2_FLAGS}
        make -j"${CPU_COUNT}"
        make install
    else
        echo "Unsupported OS for SDL2 build: $OS"
        exit 1
    fi

    cd "${TAURI_DIR}"
    rm -rf "${SDL2_SOURCE_DIR}"
    SDL2_DIR_ENV="${SDL2_INSTALL_PATH}"
    echo "SDL2 built and installed to ${SDL2_INSTALL_PATH}"
}

# --- Main build logic ---

echo "Cleaning up previous whisper.cpp build artifacts..."
rm -rf "${WHISPER_CPP_SOURCE_DIR}"
rm -rf "${INSTALL_PREFIX}"
mkdir -p "${INSTALL_PREFIX}"

# 1. Handle platform-specific dependencies and SDL2 build
if [ "$OS" = "Darwin" ]; then
    echo "Detected macOS."
    check_and_install_macos_deps
    build_sdl2
elif [ "$OS" = "Linux" ]; then
    echo "Detected Linux."
    check_and_install_linux_deps
    if [ "$ARCH" = "aarch64" ]; then
        echo "Setting up ARM64 cross-compilation environment..."
        TOOLCHAIN_DIR="${TAURI_DIR}/arm-gnu-toolchain"
        TOOLCHAIN_URL="https://developer.arm.com/-/media/Files/downloads/gnu/13.2.rel1/binrel/arm-gnu-toolchain-13.2.rel1-x86_64-aarch64-none-linux-gnu.tar.xz"
        echo "Downloading toolchain..."
        curl --retry 3 --retry-delay 5 -L -o "${TAURI_DIR}/toolchain.tar.xz" "${TOOLCHAIN_URL}"
        echo "Extracting toolchain..."
        mkdir -p "${TOOLCHAIN_DIR}"
        tar -xJf "${TAURI_DIR}/toolchain.tar.xz" -C "${TOOLCHAIN_DIR}" --strip-components=1
        rm "${TAURI_DIR}/toolchain.tar.xz"
        export TOOLCHAIN_DIR # Make it available for build_sdl2
    fi
    build_sdl2
elif [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
    echo "Detected Windows (MSYS/MINGW)."
    check_and_install_windows_deps
    build_sdl2
else
    echo "Unsupported operating system: $OS"
    exit 1
fi

# 2. Clone whisper.cpp
echo "Cloning whisper.cpp ${WHISPER_CPP_VERSION}..."
git clone --depth 1 --branch "${WHISPER_CPP_VERSION}" "${WHISPER_CPP_REPO}" "${WHISPER_CPP_SOURCE_DIR}"

# 3. Configure and Build whisper.cpp
cd "${WHISPER_CPP_SOURCE_DIR}"
mkdir -p build && cd build

echo "Configuring and building whisper.cpp..."

if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        CMAKE_COMMON_FLAGS+=" -DCMAKE_OSX_ARCHITECTURES=arm64"
    elif [ "$ARCH" = "x86_64" ]; then
        CMAKE_COMMON_FLAGS+=" -DCMAKE_OSX_ARCHITECTURES=x86_64"
    fi
    cmake .. ${CMAKE_COMMON_FLAGS} -DCMAKE_INSTALL_PREFIX="${INSTALL_PREFIX}" -DCMAKE_PREFIX_PATH="${SDL2_DIR_ENV}"
    cmake --build . --config Release --target install
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "aarch64" ]; then
        cmake .. ${CMAKE_COMMON_FLAGS} \
            -DCMAKE_INSTALL_PREFIX="${INSTALL_PREFIX}" \
            -DCMAKE_SYSTEM_NAME=Linux \
            -DCMAKE_SYSTEM_PROCESSOR=aarch64 \
            -DCMAKE_C_COMPILER="${TOOLCHAIN_DIR}/bin/aarch64-none-linux-gnu-gcc" \
            -DCMAKE_CXX_COMPILER="${TOOLCHAIN_DIR}/bin/aarch64-none-linux-gnu-g++" \
            -DCMAKE_PREFIX_PATH="${SDL2_DIR_ENV}" \
            -DCMAKE_EXE_LINKER_FLAGS="-static"
    else # x86_64
        cmake .. ${CMAKE_COMMON_FLAGS} \
            -DCMAKE_INSTALL_PREFIX="${INSTALL_PREFIX}" \
            -DCMAKE_PREFIX_PATH="${SDL2_DIR_ENV}" \
            -DCMAKE_EXE_LINKER_FLAGS="-static"
    fi
    make -j"${CPU_COUNT}"
    make install
elif [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
    # For Windows, the GitHub workflow uses -G "Unix Makefiles" and specific linker flags
    CMAKE_WINDOWS_FLAGS="-G \"Unix Makefiles\" -DCMAKE_CXX_STANDARD=17 -DCMAKE_CXX_STANDARD_REQUIRED=ON -DCMAKE_CXX_FLAGS=\"std=c++17\" -DCMAKE_EXE_LINKER_FLAGS=\"static -static-libgcc -static-libstdc++\""
    cmake .. ${CMAKE_COMMON_FLAGS} -DCMAKE_INSTALL_PREFIX="${INSTALL_PREFIX}" -DCMAKE_PREFIX_PATH="${SDL2_DIR_ENV}" ${CMAKE_WINDOWS_FLAGS}
    make -j"${CPU_COUNT}"
    make install
else
    echo "Unsupported OS for whisper.cpp build: $OS"
    exit 1
fi

# 4. Copy built executables to the final install prefix
echo "Copying built executables to ${INSTALL_PREFIX}..."
mkdir -p "${INSTALL_PREFIX}"/bin
if [[ "$OS" == "MINGW64"* || "$OS" == "MSYS"* ]]; then
    cp "${WHISPER_CPP_SOURCE_DIR}/build/bin/main.exe" "${INSTALL_PREFIX}/bin/whisper-cli-${TARGET_TRIPLE}.exe"
    cp "${WHISPER_CPP_SOURCE_DIR}/build/bin/stream.exe" "${INSTALL_PREFIX}/bin/whisper-stream-${TARGET_TRIPLE}.exe"
else
    cp "${WHISPER_CPP_SOURCE_DIR}/build/bin/main" "${INSTALL_PREFIX}/bin/whisper-cli-${TARGET_TRIPLE}"
    cp "${WHISPER_CPP_SOURCE_DIR}/build/bin/stream" "${INSTALL_PREFIX}/bin/whisper-stream-${TARGET_TRIPLE}"
fi

# 5. Clean up source files
echo "Build complete. Cleaning up source files..."
rm -rf "${WHISPER_CPP_SOURCE_DIR}"
rm -rf "${TAURI_DIR}/SDL2-src" # Clean up SDL2 source as well

echo "whisper.cpp executables have been built and installed to ${INSTALL_PREFIX}/bin"
