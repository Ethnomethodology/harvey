# Harvey Build Environment Setup

This document outlines the necessary steps to set up the build environment for the Harvey application on a Debian-based Linux system.

## System Dependencies

First, ensure your package list is up to date:

```bash
sudo apt-get update
```

Next, install the required libraries and build tools:

```bash
sudo apt-get install -y build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev
```

## Building the Application

After installing the dependencies, you can build the Tauri application:

```bash
npm run tauri build
```

## Troubleshooting

If the build fails with errors related to `pkg-config` (e.g., "The system library ... was not found"), it may be because the `.pc` files for the libraries are not in the default search path.

You can find the locations of the `.pc` files using the `dpkg -L` command. For example:

```bash
dpkg -L libwebkit2gtk-4.1-dev | grep ".pc"
dpkg -L libsoup-3.0-dev | grep ".pc"
```

Once you have the paths, you may need to set the `PKG_CONFIG_PATH` environment variable before running the build command. For example:

```bash
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
npm run tauri build
```
