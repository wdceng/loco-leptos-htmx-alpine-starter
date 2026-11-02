#!/usr/bin/env bash
# The Linux app, built where it can be: inside a Docker box of Ubuntu 22.04
# (linux/Dockerfile), since Tauri's Linux bundlers run only on Linux and the
# app links against Linux's WebKitGTK and GTK. One AppImage, the program
# alone as the Mac's .app and Windows' .exe are, copied out to
# src-tauri/target/linux/<arch>/. x86_64 by default, most Linux desktops',
# emulated on an Apple-silicon Mac and slow for it; LINUX_ARCH=aarch64
# builds arm64 at full speed there. The box is built on the first run and
# kept; the crates, the build's target folder and Tauri's AppImage tools
# are kept in Docker volumes between runs. Docker must be running.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
arch=${LINUX_ARCH:-x86_64}
case $arch in
  x86_64) platform=linux/amd64 ;;
  aarch64) platform=linux/arm64 ;;
  *)
    echo "LINUX_ARCH is x86_64 (the default) or aarch64" >&2
    exit 2
    ;;
esac
image="app-native-linux-build:$arch"
out="$here/target/linux/$arch"

echo "== the build box, $image"
docker build --platform "$platform" -t "$image" "$here/linux"

# The crate's folder mounted as is; the build's target folder in a volume,
# not on the mount, where cargo's fresh output was not yet visible to the
# next rustc under Docker Desktop's file sharing ("can't find crate" for a
# crate compiled a moment before, 2026-10-29); the AppImage copied out at
# the end. Root inside the box: the bundler keeps its downloaded tools
# under root's cache, where the box's unpacked AppImage plugin goes first
# under the name the bundler looks for (linux/Dockerfile).
echo "== building the Linux app, $arch"
docker run --rm --platform "$platform" \
  -v "$here:/app" \
  -v "app-native-cargo-$arch:/usr/local/cargo/registry" \
  -v "app-native-target-$arch:/target" \
  -v "app-native-tauri-cache-$arch:/root/.cache/tauri" \
  -e CARGO_TARGET_DIR=/target \
  "$image" sh -c "mkdir -p /root/.cache/tauri && cp /opt/linuxdeploy-plugin-appimage.sh /root/.cache/tauri/linuxdeploy-plugin-appimage.AppImage && cargo tauri build --bundles appimage && mkdir -p /app/target/linux/$arch && cp /target/release/bundle/appimage/*.AppImage /app/target/linux/$arch/"

echo "== the app: $out"
ls -la "$out"/*.AppImage
