#!/usr/bin/env python3
"""
Generates the two colour-transform tags embedded in every ICC profile that the
PDF engine writes (`crates/tok-pdf/src/color.rs`):

  * a2b.bin - `mft2` (lut16Type) tag data: CMYK -> Lab (PCS), 9^4 grid
  * b2a.bin - `mft2` (lut16Type) tag data: Lab -> CMYK, 9^3 grid

The model is a generic, deterministic, subtractive approximation (it is NOT a
characterisation of a real press condition such as FOGRA39). Run this script
to regenerate the files; the output is byte-for-byte reproducible.

    python3 gen_icc_luts.py <output directory>
"""
import struct
import sys
from pathlib import Path

A2B_GRID = 9
B2A_GRID = 9

# D50 reference white
XN, YN, ZN = 0.9642, 1.0, 0.8249


def srgb_to_linear(v):
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def linear_to_srgb(v):
    v = min(max(v, 0.0), 1.0)
    return 12.92 * v if v <= 0.0031308 else 1.055 * (v ** (1 / 2.4)) - 0.055


def rgb_to_xyz_d50(r, g, b):
    rl, gl, bl = srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)
    # sRGB (D65) -> XYZ, then Bradford adaptation to D50 (combined matrix, as in the sRGB D50 profile)
    x = 0.4360747 * rl + 0.3850649 * gl + 0.1430804 * bl
    y = 0.2225045 * rl + 0.7168786 * gl + 0.0606169 * bl
    z = 0.0139322 * rl + 0.0971045 * gl + 0.7141733 * bl
    return x, y, z


def xyz_d50_to_rgb(x, y, z):
    rl = 3.1338561 * x - 1.6168667 * y - 0.4906146 * z
    gl = -0.9787684 * x + 1.9161415 * y + 0.0334540 * z
    bl = 0.0719453 * x - 0.2289914 * y + 1.4052427 * z
    return linear_to_srgb(rl), linear_to_srgb(gl), linear_to_srgb(bl)


def f(t):
    return t ** (1 / 3) if t > 216 / 24389 else (24389 / 27 * t + 16) / 116


def finv(t):
    return t ** 3 if t ** 3 > 216 / 24389 else (116 * t - 16) / (24389 / 27)


def xyz_to_lab(x, y, z):
    fx, fy, fz = f(x / XN), f(y / YN), f(z / ZN)
    return 116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)


def lab_to_xyz(l, a, b):
    fy = (l + 16) / 116
    fx, fz = fy + a / 500, fy - b / 200
    return XN * finv(fx), YN * finv(fy), ZN * finv(fz)


def cmyk_to_rgb(c, m, y, k):
    return (1 - c) * (1 - k), (1 - m) * (1 - k), (1 - y) * (1 - k)


def rgb_to_cmyk(r, g, b):
    r, g, b = (min(max(v, 0.0), 1.0) for v in (r, g, b))
    k = 1 - max(r, g, b)
    if k >= 1 - 1e-9:
        return 0.0, 0.0, 0.0, 1.0
    d = 1 - k
    return (1 - r - k) / d, (1 - g - k) / d, (1 - b - k) / d, k


def u16(v):
    return int(min(max(round(v), 0), 65535))


# ICC v2 legacy 16-bit Lab encoding
def enc_l(l):
    return u16(l * 65280 / 100)


def enc_ab(a):
    return u16((a + 128) * 256)


def dec_l(v):
    return v * 100 / 65280


def dec_ab(v):
    return v / 256 - 128


def lut16(in_ch, out_ch, grid, clut):
    """Assembles an `mft2` tag: identity matrix, 2-entry identity input/output tables."""
    out = bytearray()
    out += b"mft2" + bytes(4)
    out += bytes([in_ch, out_ch, grid, 0])
    for i in range(9):
        out += struct.pack(">i", 0x00010000 if i % 4 == 0 else 0)
    out += struct.pack(">HH", 2, 2)
    for _ in range(in_ch):
        out += struct.pack(">HH", 0, 65535)
    out += b"".join(struct.pack(">H", v) for v in clut)
    for _ in range(out_ch):
        out += struct.pack(">HH", 0, 65535)
    return bytes(out)


def build_a2b():
    clut = []
    n = A2B_GRID - 1
    for ci in range(A2B_GRID):
        for mi in range(A2B_GRID):
            for yi in range(A2B_GRID):
                for ki in range(A2B_GRID):
                    r, g, b = cmyk_to_rgb(ci / n, mi / n, yi / n, ki / n)
                    l, a, bb = xyz_to_lab(*rgb_to_xyz_d50(r, g, b))
                    clut += [enc_l(l), enc_ab(a), enc_ab(bb)]
    return lut16(4, 3, A2B_GRID, clut)


def build_b2a():
    clut = []
    n = B2A_GRID - 1
    for li in range(B2A_GRID):
        for ai in range(B2A_GRID):
            for bi in range(B2A_GRID):
                l = dec_l(li / n * 65280)
                a = dec_ab(ai / n * 65280)
                b = dec_ab(bi / n * 65280)
                r, g, bl = xyz_d50_to_rgb(*lab_to_xyz(l, a, b))
                c, m, y, k = rgb_to_cmyk(r, g, bl)
                clut += [u16(c * 65535), u16(m * 65535), u16(y * 65535), u16(k * 65535)]
    return lut16(3, 4, B2A_GRID, clut)


if __name__ == "__main__":
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "a2b.bin").write_bytes(build_a2b())
    (out_dir / "b2a.bin").write_bytes(build_b2a())
