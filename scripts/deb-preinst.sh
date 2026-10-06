#!/bin/sh
# Preserve user edits to the optional Skill before dpkg retires old owned files.
# DPKG_ROOT also permits isolated package tests; never inspect user tool/library roots.
set -eu
case "${1:-}" in install|upgrade) ;; *) exit 0 ;; esac
root=${DPKG_ROOT:-}
case "$root" in ''|/*) ;; *) exit 1 ;; esac
root=${root%/}
admin=${DPKG_ADMINDIR:-$root/var/lib/dpkg}
manifest="$admin/info/agent-hub.md5sums"
[ -f "$manifest" ] && [ ! -L "$manifest" ] || exit 0
while read -r expected relative; do
    case "$relative" in usr/share/agenthub/skills/agenthub-manager/*) ;; *) continue ;; esac
    case "$relative" in *../*|*/..|*'//'*) continue ;; esac
    source="$root/$relative"
    [ -f "$source" ] && [ ! -L "$source" ] || continue
    parent=${source%/*}
    linked=false
    while [ "$parent" != "$root" ] && [ "$parent" != '/' ] && [ -n "$parent" ]; do
        if [ -L "$parent" ]; then linked=true; break; fi
        parent=${parent%/*}
    done
    [ "$linked" = false ] || continue
    actual=$(md5sum -- "$source")
    actual=${actual%% *}
    [ "$actual" != "$expected" ] || continue
    saved="$source.user-preserved"
    number=0
    while [ -e "$saved" ] || [ -L "$saved" ]; do
        number=$((number+1))
        saved="$source.user-preserved.$number"
    done
    cp -p -- "$source" "$saved"
done < "$manifest"
