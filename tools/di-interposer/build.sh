#!/usr/bin/env bash
# build.sh — build the 32-bit dinput.dll stand-in for the 7.55 exe (#699).
#
# Why gcc/ld and not winegcc (recorded in ~/scratch/di-interposer-findings.md):
# on this host `winegcc -m32` needs gnu/stubs-32.h (lib32-glibc) and
# /usr/lib/wine/i386-unix import libs; neither is installed with Wine 11.22.
# The direct PE path below uses Wine's 32-bit PE import libraries and produces
# a PE32 DLL whose export table is verified with winedump.
set -euo pipefail

TOOL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${1:-$TOOL_DIR/out}"
SOURCE="$TOOL_DIR/dinput-interposer.c"
DEFINITION="$TOOL_DIR/dinput.def"
WINE_IMPORTS="${WINE_IMPORTS:-/usr/lib/wine/i386-windows}"

die() { echo "di-interposer build: $*" >&2; exit 2; }

require_tools() {
    local tool
    for tool in gcc objcopy ld winedump sha256sum; do
        command -v "$tool" >/dev/null || die "$tool not found"
    done
    [ -f "$SOURCE" ] || die "missing $SOURCE"
    [ -f "$DEFINITION" ] || die "missing $DEFINITION"
    [ -d "$WINE_IMPORTS" ] || die "missing Wine PE import dir $WINE_IMPORTS"
}

compile_object() {
    gcc -m32 -c -O2 -fno-pic -fno-pie -fno-stack-protector -fno-builtin \
        -fleading-underscore -Wall -Wextra -Wno-builtin-declaration-mismatch \
        -nostdinc -I/usr/include/wine/msvcrt -I/usr/include/wine/windows \
        -DWIN32 -D_WIN32 -D__WINE__ \
        -D'__stdcall=__attribute__((__stdcall__))' \
        -D'__cdecl=__attribute__((__cdecl__))' \
        -D'__declspec(x)=' \
        -o "$OUT_DIR/dinput-interposer.o" "$SOURCE"
}

# Wine PE import symbols are decorated; GCC emits leading underscores but no
# stdcall suffix. Rename both imports and our two exported/stdcall symbols.
decorate_names() {
    objcopy \
        --remove-section=.comment \
        --redefine-sym _EnterCriticalSection=_EnterCriticalSection@4 \
        --redefine-sym _LeaveCriticalSection=_LeaveCriticalSection@4 \
        --redefine-sym _InitializeCriticalSection=_InitializeCriticalSection@4 \
        --redefine-sym _DeleteCriticalSection=_DeleteCriticalSection@4 \
        --redefine-sym _GetSystemTimeAsFileTime=_GetSystemTimeAsFileTime@4 \
        --redefine-sym _SetEvent=_SetEvent@4 \
        --redefine-sym _Sleep=_Sleep@4 \
        --redefine-sym _CloseHandle=_CloseHandle@4 \
        --redefine-sym _CreateThread=_CreateThread@24 \
        --redefine-sym _DllMain=_DllMain@12 \
        --redefine-sym _DirectInputCreateA=_DirectInputCreateA@16 \
        "$OUT_DIR/dinput-interposer.o"
}

link_dll() {
    ld -m i386pe --dll --kill-at -e _DllMain@12 \
        -o "$OUT_DIR/dinput.dll" \
        "$OUT_DIR/dinput-interposer.o" "$DEFINITION" \
        -L"$WINE_IMPORTS" -lkernel32 -lmsvcrt
}

verify_dll() {
    file "$OUT_DIR/dinput.dll" | grep -q 'PE32 executable.*Intel i386' ||
        die "output is not a 32-bit PE DLL"
    winedump dump -j export "$OUT_DIR/dinput.dll" 2>/dev/null |
        grep -q 'DirectInputCreateA' || die "DirectInputCreateA not exported"
}

main() {
    require_tools
    mkdir -p "$OUT_DIR"
    compile_object
    decorate_names
    link_dll
    verify_dll
    echo "di-interposer: built $OUT_DIR/dinput.dll"
    echo "di-interposer: sha256 $(sha256sum "$OUT_DIR/dinput.dll" | cut -d' ' -f1)"
}

main "$@"
