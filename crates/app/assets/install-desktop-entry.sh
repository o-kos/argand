#!/usr/bin/env sh
# Install the desktop entry and icons for a development build.
#
# A desktop environment does not learn an application's name or icon from the
# binary. It matches the window's application id -- io.github.o_kos.argand --
# against an installed desktop entry, and takes the icon from the icon theme by
# the name that entry gives. Without both, the window appears under its
# identifier with whatever placeholder icon the shell uses for something it does
# not recognise.
#
# This installs into the user's own directories, so it needs no privileges and
# affects nobody else. Pass a path to the binary to point the entry at a build
# tree; with no argument it assumes `argand` is on PATH.
set -eu

id=io.github.o_kos.argand
here=$(cd "$(dirname "$0")" && pwd)
apps=${XDG_DATA_HOME:-$HOME/.local/share}/applications
icons=${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor

mkdir -p "$apps"
if [ $# -ge 1 ]; then
    exec_line="Exec=$(cd "$(dirname "$1")" && pwd)/$(basename "$1") %f"
else
    exec_line="Exec=argand %f"
fi
sed "s|^Exec=.*|$exec_line|" "$here/$id.desktop" > "$apps/$id.desktop"

for size in 16 32 48 64 128 256 512 1024; do
    dir="$icons/${size}x${size}/apps"
    mkdir -p "$dir"
    cp "$here/icons/argand-$size.png" "$dir/$id.png"
done
mkdir -p "$icons/scalable/apps"
cp "$here/icons/argand.svg" "$icons/scalable/apps/$id.svg"

# Both caches are advisory: the entry works without them, but a running shell
# may not notice it until they are refreshed.
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$apps" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$icons" >/dev/null 2>&1 || true
fi

echo "installed $id.desktop and its icons under ${XDG_DATA_HOME:-$HOME/.local/share}"
