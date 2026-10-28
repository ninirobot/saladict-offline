"""生成应用/托盘图标（纯标准库，无需 Pillow）。

用法: python scripts/gen-icons.py
产物: src-tauri/icons/{32x32.png,128x128.png,icon.png,icon.ico}
形象: 蓝色渐变圆角方块 + 白色沙拉碗 + 叶片
"""

import math
import os
import struct
import zlib

OUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src-tauri", "icons")

TOP = (86, 132, 250)
BOTTOM = (43, 87, 201)
WHITE = (255, 255, 255)


def _rounded_rect_alpha(x, y, size, radius):
    cx = min(max(x, radius), size - radius)
    cy = min(max(y, radius), size - radius)
    d = math.hypot(x - cx, y - cy)
    return max(0.0, min(1.0, radius - d + 0.5))


def _ellipse(x, y, cx, cy, rx, ry):
    return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0


def _rot_ellipse(x, y, cx, cy, rx, ry, deg):
    rad = math.radians(deg)
    dx, dy = x - cx, y - cy
    u = dx * math.cos(rad) + dy * math.sin(rad)
    v = -dx * math.sin(rad) + dy * math.cos(rad)
    return (u / rx) ** 2 + (v / ry) ** 2 <= 1.0


def render(size):
    scale = size / 256.0
    radius = 58 * scale
    rows = []
    for py in range(size):
        row = bytearray()
        for px in range(size):
            x, y = px + 0.5, py + 0.5
            t = y / size
            r = int(TOP[0] + (BOTTOM[0] - TOP[0]) * t)
            g = int(TOP[1] + (BOTTOM[1] - TOP[1]) * t)
            b = int(TOP[2] + (BOTTOM[2] - TOP[2]) * t)
            a = _rounded_rect_alpha(x, y, size, radius)

            sx, sy = x / scale, y / scale
            # 碗体（下半椭圆 + 平口）
            bowl = _ellipse(sx, sy, 128, 132, 76, 60) and sy >= 132
            # 碗沿
            rim = _ellipse(sx, sy, 128, 132, 84, 12)
            # 叶片
            leaf = _rot_ellipse(sx, sy, 152, 96, 34, 15, -32)
            stem = _ellipse(sx, sy, 132, 112, 5, 22)

            if bowl or rim or leaf:
                # 内部轻微阴影，让白色图形有层次
                k = 1.0 if not bowl else 0.94
                r, g, b = (int(WHITE[0] * k), int(WHITE[1] * k), int(WHITE[2] * k))
            elif stem:
                r, g, b = (int(WHITE[0] * 0.92), int(WHITE[1] * 0.92), int(WHITE[2] * 0.92))

            row += bytes((r, g, b, int(a * 255)))
        rows.append(bytes(row))
    return rows


def write_png(path, size):
    rows = render(size)
    raw = b"".join(b"\x00" + r for r in rows)

    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)
    return png


def write_ico(path, pngs):
    count = len(pngs)
    header = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count
    entries = b""
    for size, data in pngs:
        dim = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    with open(path, "wb") as f:
        f.write(header + entries + b"".join(d for _, d in pngs))


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    small = write_png(os.path.join(OUT_DIR, "32x32.png"), 32)
    mid = write_png(os.path.join(OUT_DIR, "128x128.png"), 128)
    big = write_png(os.path.join(OUT_DIR, "icon.png"), 256)
    ico_parts = [(256, big), (128, mid), (48, write_png(os.path.join(OUT_DIR, "48x48.png"), 48)), (32, small)]
    write_ico(os.path.join(OUT_DIR, "icon.ico"), ico_parts)
    print("icons written to", os.path.normpath(OUT_DIR))


if __name__ == "__main__":
    main()
