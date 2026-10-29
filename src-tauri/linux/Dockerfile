# The Linux app's build box, used by src-tauri/linux.sh: Tauri's Linux
# bundlers run only on Linux, and the app links against Linux's WebKitGTK
# and GTK, so a Mac cannot build it as it builds the Windows .exe. Ubuntu
# 22.04 is the oldest base Tauri's docs name: an app built here runs on it,
# on Debian 12 and on everything newer (glibc 2.35; docs/development.md,
# "The app"). Built once an architecture; the slow steps are cached.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
# Tauri's own list for Debian and Ubuntu (v2.tauri.app/start/prerequisites),
# plus the certificates curl needs.
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
    libssl-dev libayatana-appindicator3-dev librsvg2-dev ca-certificates \
  && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
# The same CLI as the Mac's (docs/prerequisites.md, 7). Compiled here, which
# takes a while once, longer still emulated.
RUN cargo install --locked tauri-cli --version 2.12.4

# linuxdeploy's AppImage plugin, unpacked: Docker Desktop's x86_64 emulation
# refused to run the plugin's own AppImage ("Exec format error", 2026-10-29)
# while running linuxdeploy's, an identical runtime; unpacked, the plugin
# runs. The same file Tauri's bundler fetches, at the same address;
# linux.sh puts the wrapper where the bundler looks, so the bundler uses it
# and fetches nothing. The offset is the squashfs payload's, after the
# runtime's section headers.
RUN apt-get update && apt-get install -y squashfs-tools && rm -rf /var/lib/apt/lists/* \
  && curl -sSL -o /tmp/plugin.AppImage "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-$(uname -m).AppImage" \
  && offset=$(readelf -hW /tmp/plugin.AppImage | awk '/Start of section headers/ {s=$5} /Size of section headers/ {z=$5} /Number of section headers/ {n=$5} END {print s + z * n}') \
  && unsquashfs -q -o "$offset" -d /opt/linuxdeploy-plugin-appimage /tmp/plugin.AppImage \
  && rm /tmp/plugin.AppImage \
  && printf '#!/bin/sh\nexec /opt/linuxdeploy-plugin-appimage/AppRun "$@"\n' > /opt/linuxdeploy-plugin-appimage.sh \
  && chmod +x /opt/linuxdeploy-plugin-appimage.sh

WORKDIR /app
