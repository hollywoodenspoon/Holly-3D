#!/usr/bin/env python3
"""Packs the MakeHuman system asset pack (CC0) into Ilex's bundled form.

Usage: python3 -I tools/pack_makehuman.py <unzipped makehuman_system_assets dir> <out dir>

Writes two files:
  system.mhp.z    zlib-compressed record pack of .obj / .mhclo / .mhmat text
                  (comments, normals and blank lines stripped)
  textures.mhp    record pack of downscaled diffuse textures: RGB as JPEG,
                  alpha (where the source has any) as a separate grey PNG

Record pack: b"ILXPAK1\\n", then per record: u32 LE name length, name
(UTF-8), u32 LE data length, data.
"""
import io, os, struct, sys, zlib
from PIL import Image

SRC, OUT = sys.argv[1], sys.argv[2]
# (category folder, texture edge in px)
CATS = [("eyebrows", 256), ("eyelashes", 256), ("teeth", 512), ("tongue", 512),
        ("hair", 1024), ("clothes", 1024), ("proxymeshes", 0), ("skins", 1024)]
EYE_EDGE = 512

def clean(text, keep):
    out = []
    for line in text.splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        tag = s.split(None, 1)[0]
        if keep(tag):
            out.append(s)
    return "\n".join(out) + "\n"

# Low-poly game proxies only; the full-resolution generic / muscle
# topologies add 1.9 MB for little gain.
PROXIES = {"proxy741", "female1605", "male1591"}

def round_obj(text):
    out = []
    for line in text.splitlines():
        f = line.split()
        if f[0] in ("v", "vt"):
            line = " ".join([f[0]] + [f"{float(x):.4f}".rstrip("0").rstrip(".") for x in f[1:]])
        out.append(line)
    return "\n".join(out) + "\n"

def obj_keep(tag):
    return tag in ("v", "vt", "f", "g", "usemtl")

def any_keep(_tag):
    return True

def texture(path, edge):
    im = Image.open(path)
    im.load()
    if edge and max(im.size) > edge:
        im = im.resize((edge, edge), Image.LANCZOS)
    recs = []
    rgb = im.convert("RGB")
    b = io.BytesIO(); rgb.save(b, "JPEG", quality=86, optimize=True); recs.append(("rgb", b.getvalue()))
    if im.mode in ("RGBA", "LA"):
        a = im.getchannel("A")
        lo, hi = a.getextrema()
        if lo < 250:
            b = io.BytesIO(); a.save(b, "PNG", optimize=True); recs.append(("alpha", b.getvalue()))
    return recs

geo, tex, seen = [], [], {}
for cat, edge in CATS:
    root = os.path.join(SRC, cat)
    for item in sorted(os.listdir(root)):
        d = os.path.join(root, item)
        if not os.path.isdir(d) or (cat == "proxymeshes" and item not in PROXIES):
            continue
        for f in sorted(os.listdir(d)):
            p = os.path.join(d, f)
            low = f.lower()
            key = f"{cat}/{item}/{f}"
            if low.endswith(".obj"):
                geo.append((key, round_obj(clean(open(p, encoding="utf-8", errors="replace").read(), obj_keep))))
            elif low.endswith((".mhclo", ".proxy", ".mhmat")):
                geo.append((key, clean(open(p, encoding="utf-8", errors="replace").read(), any_keep)))
            elif low.endswith(".png") and edge and not any(k in low for k in ("normal", "_ao", "bump", "spec", "disp")):
                data = open(p, "rb").read()
                h = hash(data)
                if h in seen:  # identical texture (teeth share one): store a link
                    geo.append((key + ".link", seen[h]))
                    continue
                seen[h] = key
                for kind, b in texture(p, edge):
                    tex.append((f"{key}.{kind}", b))
eyes = os.path.join(SRC, "eyes", "materials")
for f in sorted(os.listdir(eyes)):
    p = os.path.join(eyes, f)
    if f.endswith(".mhmat"):
        geo.append((f"eyes/materials/{f}", clean(open(p).read(), any_keep)))
    elif f.endswith(".png"):
        for kind, b in texture(p, EYE_EDGE):
            tex.append((f"eyes/materials/{f}.{kind}", b))

def pack(recs):
    out = bytearray(b"ILXPAK1\n")
    for name, data in recs:
        if isinstance(data, str):
            data = data.encode()
        n = name.encode()
        out += struct.pack("<I", len(n)) + n + struct.pack("<I", len(data)) + data
    return bytes(out)

os.makedirs(OUT, exist_ok=True)
g = pack(geo)
open(os.path.join(OUT, "system.mhp.z"), "wb").write(zlib.compress(g, 9))
open(os.path.join(OUT, "textures.mhp"), "wb").write(pack(tex))
print(f"{len(geo)} text records ({len(g)} bytes raw), {len(tex)} texture records")
