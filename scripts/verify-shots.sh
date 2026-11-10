#!/usr/bin/env bash
#
# Guard the shots PNG oracle against vacuous comparison.
#
# Usage: scripts/verify-shots.sh <before-dir> <after-dir>
#
# A captures pair proves nothing when game data was invisible to the walk:
# without `install-root/` every scene paints its flat fallback (and the
# side shots vanish entirely), so two fallback captures compare equal
# trivially. Before byte-comparing, this script asserts:
#   1. both directories exist with identical, non-empty file-name sets;
#   2. lanking.png is present (side-shot sentinel: side shots are
#      skipped, never fallback-painted, when data is absent);
#   3. room.png is not flat (unique RGB count above threshold: the
#      fallback carries ~22 colors, genuine art carries ~11000).
# Exit 0 only when every file is byte-identical. Any DIFF is listed.
set -euo pipefail

die() {
    printf 'verify-shots: %s\n' "$1" >&2
    exit 1
}

[ "$#" -eq 2 ] || die "usage: scripts/verify-shots.sh <before-dir> <after-dir>"
before=$1
after=$2
[ -d "$before" ] || die "missing directory: $before"
[ -d "$after" ] || die "missing directory: $after"

before_list=$(ls "$before")
after_list=$(ls "$after")
[ -n "$before_list" ] || die "$before holds no captures"
[ "$before_list" = "$after_list" ] || {
    printf 'verify-shots: capture sets differ\n--- before ---\n%s\n--- after ---\n%s\n' \
        "$before_list" "$after_list" >&2
    die "captures are not comparable (one side likely ran without game data)"
}

[ -f "$before/lanking.png" ] || die "lanking.png absent: side shots vanish without game data"

unique_colors() {
    python3 - "$1" <<'EOF'
import struct, sys, zlib
path = sys.argv[1]
data = open(path, 'rb').read()
if data[:8] != b'\x89PNG\r\n\x1a\n':
    raise SystemExit(f'not a PNG: {path}')
pos, width, height, color_type, chunks = 8, 0, 0, 0, b''
while pos < len(data):
    (length,) = struct.unpack('>I', data[pos:pos + 4])
    kind = data[pos + 4:pos + 8]
    body = data[pos + 8:pos + 8 + length]
    if kind == b'IHDR':
        width, height, _, color_type = struct.unpack('>IIBB', body[:10])
    elif kind == b'IDAT':
        chunks += body
    pos += 12 + length
if color_type != 2:
    raise SystemExit(f'not an RGB PNG: {path}')
raw = zlib.decompress(chunks)
stride = width * 3
pixels, previous, at = set(), bytearray(stride), 0
for _ in range(height):
    filt = raw[at]
    line = bytearray(raw[at + 1:at + 1 + stride])
    at += 1 + stride
    if filt == 1:
        for i in range(3, stride):
            line[i] = (line[i] + line[i - 3]) & 255
    elif filt == 2:
        for i in range(stride):
            line[i] = (line[i] + previous[i]) & 255
    elif filt == 3:
        for i in range(stride):
            above = previous[i]
            left = line[i - 3] if i >= 3 else 0
            line[i] = (line[i] + ((left + above) >> 1)) & 255
    elif filt == 4:
        for i in range(stride):
            a = line[i - 3] if i >= 3 else 0
            b = previous[i]
            c = previous[i - 3] if i >= 3 else 0
            p = a + b - c
            pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
            pick = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
            line[i] = (line[i] + pick) & 255
    elif filt != 0:
        raise SystemExit(f'unknown filter {filt} in {path}')
    for i in range(0, stride, 3):
        pixels.add(bytes(line[i:i + 3]))
    previous = line
print(len(pixels))
EOF
}

# Flat fallback carries tens of colors; genuine art carries thousands.
flat_ceiling=1000
for side in "$before" "$after"; do
    count=$(unique_colors "$side/room.png")
    [ "$count" -gt "$flat_ceiling" ] || \
        die "$side/room.png carries $count unique colors (ceiling $flat_ceiling): flat fallback, game data was invisible"
done

failures=0
for file in "$before"/*.png; do
    name=$(basename "$file")
    if cmp -s "$file" "$after/$name"; then
        printf 'SAME %s\n' "$name"
    else
        printf 'DIFF %s\n' "$name"
        failures=$((failures + 1))
    fi
done
[ "$failures" -eq 0 ] || die "$failures file(s) differ"
printf 'verify-shots: all captures byte-identical\n'
