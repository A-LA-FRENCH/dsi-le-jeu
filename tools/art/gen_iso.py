#!/usr/bin/env python3
"""Pixel art isométrique du « Plateau DSI » (open space de la DSI).

    python3 tools/art/gen_iso.py

Sorties :
  assets/sprites/iso_tiles.png   sols (32×16) : moquette ×2, zone DSI, zone prestataires, parquet
  assets/sprites/iso_objects.png objets et murs, cellules 64×64, ancre au sol (32, 48)
  assets/sprites/iso_people.png  personnages 16×24, ancre (8, 23) ; 5 images par ligne
Les index doivent correspondre aux constantes de crates/dsi-client/src/plateau.rs.
"""
from PIL import Image, ImageDraw
import os, random

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
random.seed(11)

GX = (16, 8)    # vecteur écran d'un pas en x de grille
GY = (-16, 8)   # vecteur écran d'un pas en y de grille
CELL = 64
AX, AY = 32, 48  # ancre (centre de la case au sol) dans une cellule


def shade(c, k):
    return tuple(max(0, min(255, int(v * k))) for v in c[:3])


def pt(cx, cy, ax, by, up=0):
    return (cx + ax * GX[0] + by * GY[0], cy + ax * GX[1] + by * GY[1] - up)


def box(d, cx, cy, ax, by, h, col, z0=0):
    """Pavé isométrique centré sur (cx, cy) au sol ; demi-tailles ax, by (en cases)."""
    top = [pt(cx, cy, -ax, -by, z0 + h), pt(cx, cy, ax, -by, z0 + h), pt(cx, cy, ax, by, z0 + h), pt(cx, cy, -ax, by, z0 + h)]
    right = [pt(cx, cy, ax, -by, z0 + h), pt(cx, cy, ax, by, z0 + h), pt(cx, cy, ax, by, z0), pt(cx, cy, ax, -by, z0)]
    left = [pt(cx, cy, -ax, by, z0 + h), pt(cx, cy, ax, by, z0 + h), pt(cx, cy, ax, by, z0), pt(cx, cy, -ax, by, z0)]
    d.polygon(right, fill=shade(col, 0.72))
    d.polygon(left, fill=shade(col, 0.86))
    d.polygon(top, fill=col)
    d.line(top + [top[0]], fill=shade(col, 1.15))
    return top


def px(d, x, y, c):
    d.point((x, y), fill=c)


# --- Sols -----------------------------------------------------------------------------
def gen_tiles():
    img = Image.new("RGBA", (32 * 5, 16), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cols = [(60, 72, 96), (56, 68, 90), (92, 64, 56), (70, 72, 78), (120, 88, 60)]
    for i, c in enumerate(cols):
        ox = i * 32
        poly = [(ox + 16, 0), (ox + 31, 7), (ox + 16, 15), (ox + 0, 7)]
        d.polygon(poly, fill=c)
        d.line([(ox + 16, 0), (ox + 31, 7)], fill=shade(c, 1.12))
        d.line([(ox + 0, 7), (ox + 16, 0)], fill=shade(c, 1.12))
        for _ in range(5):
            x, y = ox + random.randint(8, 24), random.randint(4, 11)
            px(d, x, y, shade(c, 0.9))
        if i == 4:  # parquet
            for k in range(3):
                d.line([(ox + 4 + k * 8, 7 - k * 2 + 2), (ox + 12 + k * 8, 11 - k * 2 + 2)], fill=shade(c, 0.8))
    img.save(os.path.join(ROOT, "assets/sprites/iso_tiles.png"))


# --- Objets ------------------------------------------------------------------------------
WALL = (66, 76, 100)
WALL_H = 40


def wall_left(d, ox, oy, kind):
    cx, cy = ox + AX, oy + AY
    poly = [(cx - 16, cy), (cx, cy - 8), (cx, cy - 8 - WALL_H), (cx - 16, cy - WALL_H)]
    d.polygon(poly, fill=shade(WALL, 0.9))
    d.line([(cx - 16, cy - WALL_H), (cx, cy - 8 - WALL_H)], fill=shade(WALL, 1.3))
    d.line([(cx - 16, cy), (cx, cy - 8)], fill=shade(WALL, 0.6))
    if kind == "window":
        d.polygon([(cx - 13, cy - 12), (cx - 3, cy - 17), (cx - 3, cy - 37), (cx - 13, cy - 32)], fill=(34, 50, 84))
        for k in range(3):
            px(d, cx - 11 + k * 3, cy - 28 + k, (250, 214, 92))
        d.line([(cx - 8, cy - 14), (cx - 8, cy - 34)], fill=shade(WALL, 1.2))
    elif kind == "door":
        d.polygon([(cx - 12, cy - 2), (cx - 4, cy - 6), (cx - 4, cy - 30), (cx - 12, cy - 26)], fill=(90, 70, 50))
        px(d, cx - 6, cy - 16, (240, 220, 120))
    elif kind == "poster":
        d.polygon([(cx - 12, cy - 18), (cx - 4, cy - 22), (cx - 4, cy - 34), (cx - 12, cy - 30)], fill=(236, 232, 220))
        d.polygon([(cx - 11, cy - 27), (cx - 5, cy - 30), (cx - 5, cy - 32), (cx - 11, cy - 29)], fill=(124, 230, 110))


def wall_right(d, ox, oy, kind):
    cx, cy = ox + AX, oy + AY
    poly = [(cx, cy - 8), (cx + 16, cy), (cx + 16, cy - WALL_H), (cx, cy - 8 - WALL_H)]
    d.polygon(poly, fill=WALL)
    d.line([(cx, cy - 8 - WALL_H), (cx + 16, cy - WALL_H)], fill=shade(WALL, 1.3))
    d.line([(cx, cy - 8), (cx + 16, cy)], fill=shade(WALL, 0.6))
    if kind == "window":
        d.polygon([(cx + 3, cy - 17), (cx + 13, cy - 12), (cx + 13, cy - 32), (cx + 3, cy - 37)], fill=(34, 50, 84))
        for k in range(3):
            px(d, cx + 5 + k * 3, cy - 30 + k, (120, 170, 220))
        d.line([(cx + 8, cy - 14), (cx + 8, cy - 34)], fill=shade(WALL, 1.2))
    elif kind == "kanban":
        d.polygon([(cx + 1, cy - 16), (cx + 15, cy - 9), (cx + 15, cy - 30), (cx + 1, cy - 37)], fill=(236, 236, 228))
        for col in range(3):
            for k in range(random.randint(1, 3)):
                c = random.choice([(250, 214, 92), (246, 162, 60), (124, 230, 110), (110, 220, 225)])
                x = cx + 3 + col * 4
                y = cy - 32 + col * 2 + k * 5
                d.rectangle((x, y, x + 2, y + 2), fill=c)
    elif kind == "screen":
        d.polygon([(cx + 2, cy - 18), (cx + 14, cy - 12), (cx + 14, cy - 30), (cx + 2, cy - 36)], fill=(14, 18, 24))
        for k in range(4):
            h = random.randint(3, 10)
            x = cx + 4 + k * 3
            base = cy - 17 + k * 1.5
            d.line([(x, base), (x, base - h)], fill=[(124, 230, 110), (250, 214, 92), (110, 220, 225), (238, 98, 98)][k])
    elif kind == "sign":
        d.polygon([(cx + 2, cy - 22), (cx + 14, cy - 16), (cx + 14, cy - 26), (cx + 2, cy - 32)], fill=(250, 214, 92))
        for k in range(4):
            px(d, cx + 4 + k * 3, cy - 26 + k * 1.5, (20, 20, 20))


def desk(d, ox, oy, big=False):
    cx, cy = ox + AX, oy + AY
    ax, by = (0.62, 0.36) if big else (0.42, 0.3)
    wood = (120, 84, 56) if big else (150, 112, 74)
    box(d, cx, cy, ax, by, 10, wood)
    # pieds
    for s in (-1, 1):
        x, y = pt(cx, cy, ax * 0.9, s * by * 0.9)
        d.line([(x, y), (x, y - 9)], fill=shade(wood, 0.5))
    # écran vu de dos (face au personnage assis derrière)
    mx, my = pt(cx, cy, 0.18, -0.05, 10)
    d.polygon([(mx - 6, my - 2), (mx + 4, my + 3), (mx + 4, my - 7), (mx - 6, my - 12)], fill=(30, 34, 44))
    d.line([(mx - 1, my + 1), (mx - 1, my + 3)], fill=(60, 64, 74))
    # clavier / tasse / papiers
    kx, ky = pt(cx, cy, -0.15, 0.15, 10)
    d.polygon([(kx - 4, ky), (kx, ky - 2), (kx + 4, ky), (kx, ky + 2)], fill=(200, 200, 196))
    if big:
        tx, ty = pt(cx, cy, -0.45, 0.2, 10)
        d.rectangle((tx, ty - 4, tx + 2, ty - 1), fill=(240, 240, 240))
        nx, ny = pt(cx, cy, 0.5, 0.0, 10)
        d.rectangle((nx - 3, ny - 3, nx + 3, ny - 1), fill=(250, 214, 92))
        px_, py_ = pt(cx, cy, -0.5, -0.25, 10)
        for k in range(3):
            d.rectangle((px_ - 3 + k, py_ - 2 - k * 2, px_ + 2 + k, py_ - 1 - k * 2), fill=(244, 240, 226))


def coffee(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.28, 0.28, 24, (180, 60, 60))
    x, y = pt(cx, cy, 0.28, 0.0, 14)
    d.rectangle((x - 3, y - 4, x + 1, y + 2), fill=(30, 30, 30))
    px(d, x - 1, y, (240, 240, 240))
    x, y = pt(cx, cy, 0.0, 0.28, 20)
    d.rectangle((x - 3, y - 2, x + 1, y + 1), fill=(124, 230, 110))


def plant(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.18, 0.18, 8, (150, 90, 60))
    for _ in range(26):
        x = cx + random.randint(-7, 7)
        y = cy - 10 - random.randint(0, 16) + abs(x - cx) // 2
        d.rectangle((x, y, x + 1, y + 1), fill=random.choice([(70, 150, 80), (44, 104, 58), (100, 180, 90)]))


def rack(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.34, 0.34, 34, (40, 44, 54))
    for k in range(8):
        x, y = pt(cx, cy, 0.0, 0.34, 4 + k * 4)
        d.line([(x - 4, y - 2), (x + 4, y + 2)], fill=(24, 26, 32))
        px(d, x - 3, y - 2, random.choice([(124, 230, 110), (124, 230, 110), (246, 162, 60)]))
        px(d, x - 1, y - 1, random.choice([(124, 230, 110), (110, 160, 240)]))


def table(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.85, 0.55, 9, (110, 78, 52))
    for (a, b) in [(-0.4, -0.2), (0.3, 0.2), (0.3, -0.25)]:
        x, y = pt(cx, cy, a, b, 9)
        d.polygon([(x - 4, y), (x, y - 2), (x + 4, y), (x, y + 2)], fill=(170, 176, 186))
        d.line([(x - 4, y), (x - 4, y - 5)], fill=(60, 64, 74))


def tv(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    d.line([(cx, cy), (cx, cy - 16)], fill=(70, 74, 84))
    d.polygon([(cx - 12, cy - 30), (cx + 10, cy - 19), (cx + 10, cy - 33), (cx - 12, cy - 44)], fill=(16, 20, 28))
    for k in range(5):
        h = random.randint(3, 9)
        x = cx - 9 + k * 4
        base = cy - 30 + k * 2
        d.line([(x, base), (x, base - h)], fill=[(124, 230, 110), (250, 214, 92), (110, 220, 225), (238, 98, 98), (124, 230, 110)][k])


def robot(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.14, 0.14, 10, (150, 160, 180))
    d.rectangle((cx - 4, cy - 20, cx + 4, cy - 13), fill=(180, 190, 210))
    px(d, cx - 2, cy - 17, (110, 220, 225))
    px(d, cx + 2, cy - 17, (110, 220, 225))
    d.line([(cx, cy - 20), (cx, cy - 23)], fill=(150, 160, 180))
    px(d, cx, cy - 24, (238, 98, 98))


def printer(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.3, 0.25, 12, (200, 200, 196))
    x, y = pt(cx, cy, 0.0, 0.0, 12)
    d.polygon([(x - 5, y), (x, y - 3), (x + 5, y), (x, y + 3)], fill=(244, 240, 226))
    x, y = pt(cx, cy, 0.3, 0.0, 8)
    px(d, x, y, (238, 98, 98))


def sofa(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.6, 0.3, 6, (130, 90, 170))
    box(d, cx, cy, 0.6, 0.08, 12, (150, 110, 190), 0)


def context_pillar(d, ox, oy):
    cx, cy = ox + AX, oy + AY
    box(d, cx, cy, 0.22, 0.22, 30, (60, 80, 120))
    for k in range(6):
        x, y = pt(cx, cy, 0.0, 0.22, 4 + k * 4)
        d.line([(x - 3, y - 1), (x + 3, y + 2)], fill=(110, 220, 225))


OBJECTS = [
    ("desk", lambda d, x, y: desk(d, x, y)),            # 0
    ("desk_big", lambda d, x, y: desk(d, x, y, True)),  # 1
    ("coffee", coffee),                                  # 2
    ("plant", plant),                                    # 3
    ("rack", rack),                                      # 4
    ("table", table),                                    # 5
    ("tv", tv),                                          # 6
    ("robot", robot),                                    # 7
    ("printer", printer),                                # 8
    ("sofa", sofa),                                      # 9
    ("context", context_pillar),                         # 10
    ("wall_l", lambda d, x, y: wall_left(d, x, y, "")),          # 11
    ("wall_l_window", lambda d, x, y: wall_left(d, x, y, "window")),  # 12
    ("wall_l_door", lambda d, x, y: wall_left(d, x, y, "door")),      # 13
    ("wall_l_poster", lambda d, x, y: wall_left(d, x, y, "poster")),  # 14
    ("wall_r", lambda d, x, y: wall_right(d, x, y, "")),             # 15
    ("wall_r_window", lambda d, x, y: wall_right(d, x, y, "window")),  # 16
    ("wall_r_kanban", lambda d, x, y: wall_right(d, x, y, "kanban")),  # 17
    ("wall_r_screen", lambda d, x, y: wall_right(d, x, y, "screen")),  # 18
    ("wall_r_sign", lambda d, x, y: wall_right(d, x, y, "sign")),      # 19
]


def gen_objects():
    cols = 8
    rows = (len(OBJECTS) + cols - 1) // cols
    img = Image.new("RGBA", (CELL * cols, CELL * rows), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for i, (_, f) in enumerate(OBJECTS):
        f(d, (i % cols) * CELL, (i // cols) * CELL)
    img.save(os.path.join(ROOT, "assets/sprites/iso_objects.png"))


# --- Personnages ------------------------------------------------------------------------------
SHIRTS = [
    (60, 160, 80),    # 0 junior
    (60, 100, 190),   # 1 senior
    (130, 90, 190),   # 2 staff
    (50, 140, 140),   # 3 architecte
    (220, 130, 50),   # 4 SRE
    (190, 60, 60),    # 5 sécurité
    (60, 170, 190),   # 6 data
    (210, 100, 160),  # 7 ML
    (210, 180, 60),   # 8 platform
    (110, 114, 124),  # 9 prestataire ESN
    (34, 44, 80),     # 10 DSI
]
VARIANTS = [((246, 206, 170), (60, 40, 25), False), ((176, 120, 84), (20, 16, 14), True), ((222, 170, 128), (150, 90, 40), False)]


def person(d, ox, oy, shirt, skin, hair, long_hair, frame, lanyard, style=None):
    # 16×24, pieds en (8, 23)
    pants = (40, 44, 60)
    sk = skin
    # jambes
    if frame in (0, 1):  # assis : jambes masquées par le bureau, on les dessine courtes
        d.rectangle((ox + 5, oy + 17, ox + 10, oy + 19), fill=pants)
    elif frame == 2:
        d.rectangle((ox + 5, oy + 16, ox + 6, oy + 23), fill=pants)
        d.rectangle((ox + 9, oy + 16, ox + 10, oy + 21), fill=pants)
    elif frame == 3:
        d.rectangle((ox + 5, oy + 16, ox + 6, oy + 21), fill=pants)
        d.rectangle((ox + 9, oy + 16, ox + 10, oy + 23), fill=pants)
    else:
        d.rectangle((ox + 5, oy + 16, ox + 6, oy + 23), fill=pants)
        d.rectangle((ox + 9, oy + 16, ox + 10, oy + 23), fill=pants)
    # buste
    d.rectangle((ox + 4, oy + 9, ox + 11, oy + 16), fill=shirt)
    d.rectangle((ox + 7, oy + 9, ox + 8, oy + 10), fill=(236, 236, 228))
    if lanyard:
        d.line([(ox + 6, oy + 9), (ox + 7, oy + 13)], fill=(250, 214, 92))
        d.rectangle((ox + 7, oy + 13, ox + 8, oy + 14), fill=(236, 236, 228))
    # bras
    if frame == 0:
        d.rectangle((ox + 3, oy + 10, ox + 3, oy + 13), fill=shirt)
        d.rectangle((ox + 12, oy + 10, ox + 12, oy + 12), fill=shirt)
        px(d, ox + 4, oy + 14, sk)
        px(d, ox + 11, oy + 13, sk)
    elif frame == 1:
        d.rectangle((ox + 3, oy + 10, ox + 3, oy + 12), fill=shirt)
        d.rectangle((ox + 12, oy + 10, ox + 12, oy + 13), fill=shirt)
        px(d, ox + 4, oy + 13, sk)
        px(d, ox + 11, oy + 14, sk)
    else:
        d.rectangle((ox + 3, oy + 10, ox + 3, oy + 15), fill=shirt)
        d.rectangle((ox + 12, oy + 10, ox + 12, oy + 15), fill=shirt)
        px(d, ox + 3, oy + 16, sk)
        px(d, ox + 12, oy + 16, sk)
    # tête
    if style == "afro":
        d.ellipse((ox + 3, oy - 1, ox + 12, oy + 7), fill=hair)
    if style == "hijab":
        d.rectangle((ox + 4, oy + 1, ox + 11, oy + 10), fill=hair)
        d.rectangle((ox + 6, oy + 3, ox + 9, oy + 8), fill=sk)
        px(d, ox + 6, oy + 5, (20, 20, 24))
        px(d, ox + 9, oy + 5, (20, 20, 24))
        return
    d.rectangle((ox + 5, oy + 2, ox + 10, oy + 8), fill=sk)
    if style != "bald":
        d.rectangle((ox + 5, oy + 1, ox + 10, oy + 3), fill=hair)
        px(d, ox + 4, oy + 3, hair)
        px(d, ox + 11, oy + 3, hair)
    if style in ("bun",):
        d.rectangle((ox + 7, oy - 1, ox + 8, oy + 0), fill=hair)
    if long_hair:
        d.rectangle((ox + 4, oy + 3, ox + 4, oy + 9), fill=hair)
        d.rectangle((ox + 11, oy + 3, ox + 11, oy + 9), fill=hair)
    px(d, ox + 6, oy + 5, (20, 20, 24))
    px(d, ox + 9, oy + 5, (20, 20, 24))
    px(d, ox + 7, oy + 7, shade(sk, 0.75))
    px(d, ox + 8, oy + 7, shade(sk, 0.75))


def avatar_rows():
    """Une ligne de sprites par avatar de DSI (même peau, mêmes cheveux que le portrait)."""
    import sys
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import gen_art
    out = []
    for skin, hair, style, _cloth, _acc, _gl, _beard, _f in gen_art.AVATARS:
        long_hair = style in ("long", "bob", "braids", "locs", "pony", "curly")
        out.append((gen_art.C[skin], hair, long_hair, style))
    return out


def gen_people():
    fw, fh, frames = 16, 24, 5
    avatars = avatar_rows()
    rows = len(SHIRTS) * len(VARIANTS) + len(avatars)
    img = Image.new("RGBA", (fw * frames, fh * rows), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for si, shirt in enumerate(SHIRTS):
        for vi, (skin, hair, long_hair) in enumerate(VARIANTS):
            row = si * len(VARIANTS) + vi
            for f in range(frames):
                person(d, f * fw, row * fh, shirt, skin, hair, long_hair, f, si == 9)
    base = len(SHIRTS) * len(VARIANTS)
    for k, (skin, hair, long_hair, style) in enumerate(avatars):
        for f in range(frames):
            person(d, f * fw, (base + k) * fh, (34, 44, 80), skin, hair, long_hair, f, True, style)
    img.save(os.path.join(ROOT, "assets/sprites/iso_people.png"))


def gen_fx_extra():
    """Ajoute des bulles à fx.png : 5 café, 6 code, 7 nuage gris, 8 ampoule."""
    p = os.path.join(ROOT, "assets/sprites/fx.png")
    base = Image.open(p).convert("RGBA").crop((0, 0, 13 * 5, 14))
    img = Image.new("RGBA", (13 * 9, 14), (0, 0, 0, 0))
    img.paste(base, (0, 0))
    d = ImageDraw.Draw(img)
    ink = (16, 20, 24)

    def bubble(i, col):
        ox = i * 13
        d.rectangle((ox + 1, 0, ox + 11, 9), fill=ink)
        d.rectangle((ox + 2, 1, ox + 10, 8), fill=col)
        d.rectangle((ox + 5, 10, ox + 7, 11), fill=ink)
        px(d, ox + 6, 12, ink)
        return ox

    ox = bubble(5, (236, 236, 228))
    d.rectangle((ox + 4, 3, ox + 7, 6), fill=(120, 80, 50))
    px(d, ox + 8, 4, (120, 80, 50))
    ox = bubble(6, (30, 40, 50))
    for k, x in enumerate([3, 5, 7, 9]):
        px(d, ox + x, 4 + (k % 2), (124, 230, 110))
    ox = bubble(7, (140, 148, 166))
    d.rectangle((ox + 4, 3, ox + 8, 5), fill=(90, 96, 110))
    ox = bubble(8, (250, 214, 92))
    d.rectangle((ox + 5, 2, ox + 7, 5), fill=(255, 255, 220))
    d.rectangle((ox + 5, 6, ox + 7, 7), fill=(110, 110, 110))
    img.save(p)


if __name__ == "__main__":
    gen_tiles()
    gen_objects()
    gen_people()
    gen_fx_extra()
    print("iso ok")
