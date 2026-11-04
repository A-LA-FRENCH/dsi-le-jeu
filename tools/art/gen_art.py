#!/usr/bin/env python3
"""Génère tout le pixel art original de DSI — Le Jeu (aucun asset externe).

    python3 tools/art/gen_art.py

Sorties :
  assets/sprites/sky.png        fond (ciel nocturne + skyline)
  assets/sprites/building.png   le siège en coupe : 15 salles sur 3 étages
  assets/sprites/people.png     salariés qui se déplacent (8 looks × 4 images)
  assets/sprites/fx.png         bulles, voyants, alerte
  assets/ui/portraits.png       16 portraits 32×32 (personnages + DSI)
  web/favicon.png
Les coordonnées des salles doivent correspondre à `ROOM_*` dans crates/dsi-client/src/world.rs.
"""
from PIL import Image, ImageDraw
import os, random

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
random.seed(7)

# --- Géométrie (partagée avec le client) -------------------------------------
RW, RH = 56, 40          # intérieur d'une salle
WALL = 2                 # épaisseur des murs et dalles
COLS, FLOORS = 5, 3
MARGIN_X = 12
ROOF = 24                # hauteur au-dessus du dernier étage (toit + enseigne)
STREET = 10
BW = COLS * RW + (COLS + 1) * WALL
BH = FLOORS * RH + (FLOORS + 1) * WALL
W = BW + 2 * MARGIN_X
H = ROOF + BH + STREET


def room_origin(col, floor):
    """Coin haut-gauche de l'intérieur d'une salle (floor 0 = étage du haut)."""
    x = MARGIN_X + WALL + col * (RW + WALL)
    y = ROOF + WALL + floor * (RH + WALL)
    return x, y


# --- Palette -------------------------------------------------------------------
C = {
    "ink": (16, 20, 24), "wall": (58, 66, 82), "wall_d": (40, 46, 60), "wall_l": (88, 98, 118),
    "floor": (74, 62, 56), "floor_l": (96, 82, 72), "carpet_b": (52, 70, 104), "carpet_r": (110, 52, 58),
    "wood": (140, 96, 62), "wood_d": (102, 68, 44), "white": (232, 232, 224), "paper": (244, 240, 226),
    "grey": (130, 138, 150), "grey_d": (90, 96, 108), "screen": (22, 34, 40), "green": (124, 255, 82),
    "cyan": (90, 220, 230), "blue": (90, 140, 230), "red": (230, 80, 80), "orange": (246, 162, 60),
    "yellow": (250, 214, 92), "purple": (170, 110, 210), "plant": (70, 150, 80), "plant_d": (44, 104, 58),
    "glass": (120, 170, 200), "metal": (150, 156, 166), "black": (10, 12, 14), "rack": (34, 38, 46),
    "skin0": (252, 224, 198), "skin1": (246, 206, 170), "skin2": (222, 170, 128), "skin3": (176, 120, 84), "skin4": (120, 80, 56), "skin5": (88, 56, 40),
}


def px(d, x, y, c):
    d.point((x, y), fill=c)


def rect(d, x, y, w, h, c):
    if w <= 0 or h <= 0:
        return
    d.rectangle((x, y, x + w - 1, y + h - 1), fill=c)


def outline(d, x, y, w, h, c):
    d.rectangle((x, y, x + w - 1, y + h - 1), outline=c)


# --- Mobilier --------------------------------------------------------------------
def desk(d, x, y, w=16, screen=True, code=False):
    rect(d, x, y, w, 3, C["wood"])
    rect(d, x, y + 3, w, 1, C["wood_d"])
    rect(d, x + 1, y + 4, 1, 6, C["wood_d"])
    rect(d, x + w - 2, y + 4, 1, 6, C["wood_d"])
    if screen:
        sx = x + w // 2 - 5
        rect(d, sx, y - 8, 10, 7, C["black"])
        rect(d, sx + 1, y - 7, 8, 5, C["screen"])
        if code:
            for i in range(3):
                rect(d, sx + 2, y - 6 + i * 2, random.randint(2, 6), 1, C["green"])
        else:
            rect(d, sx + 2, y - 6, 5, 1, C["cyan"])
            rect(d, sx + 2, y - 4, 3, 1, C["white"])
        rect(d, sx + 4, y - 1, 2, 1, C["grey_d"])


def chair(d, x, y, c=None):
    c = c or C["grey_d"]
    rect(d, x, y, 4, 4, c)
    rect(d, x, y + 4, 4, 1, C["black"])
    rect(d, x + 1, y + 5, 2, 3, C["black"])


def plant(d, x, y):
    rect(d, x + 1, y + 6, 4, 4, C["wood_d"])
    for (dx, dy) in [(2, 0), (1, 1), (3, 1), (0, 2), (2, 2), (4, 2), (1, 3), (3, 3), (2, 4), (0, 4), (4, 4), (2, 5)]:
        px(d, x + dx, y + dy, C["plant"] if (dx + dy) % 2 else C["plant_d"])


def window(d, x, y, w=12, h=10):
    rect(d, x, y, w, h, C["wall_l"])
    rect(d, x + 1, y + 1, w - 2, h - 2, (30, 44, 74))
    for i in range(3):
        px(d, x + 2 + i * 3, y + 2 + (i % 2), C["yellow"])
    rect(d, x + w // 2, y + 1, 1, h - 2, C["wall_l"])


def poster(d, x, y, c, w=8, h=10):
    rect(d, x, y, w, h, C["paper"])
    rect(d, x + 1, y + 1, w - 2, 3, c)
    for i in range(2):
        rect(d, x + 1, y + 5 + i * 2, w - 3, 1, C["grey"])


def whiteboard(d, x, y, w=22, h=13, kind="archi"):
    rect(d, x, y, w, h, C["metal"])
    rect(d, x + 1, y + 1, w - 2, h - 2, C["white"])
    if kind == "archi":
        for (bx, by) in [(3, 2), (12, 2), (7, 7)]:
            outline(d, x + bx, y + by, 6, 4, C["blue"])
        d.line((x + 9, y + 4, x + 12, y + 4), fill=C["red"])
        d.line((x + 6, y + 6, x + 8, y + 7), fill=C["red"])
        d.line((x + 15, y + 6, x + 12, y + 7), fill=C["red"])
    elif kind == "kanban":
        for col in range(3):
            rect(d, x + 2 + col * 7, y + 1, 1, h - 2, C["grey"])
            for i in range(random.randint(1, 3)):
                c = [C["yellow"], C["orange"], C["green"], C["cyan"]][random.randint(0, 3)]
                rect(d, x + 3 + col * 7, y + 2 + i * 3, 4, 2, c)
    elif kind == "chart":
        pts = [(2, 9), (6, 7), (10, 8), (14, 4), (18, 3)]
        for a, b in zip(pts, pts[1:]):
            d.line((x + a[0], y + a[1], x + b[0], y + b[1]), fill=C["green"])
        rect(d, x + 2, y + 2, 6, 1, C["grey_d"])


def screen_wall(d, x, y, n, kind="dash", w=11, h=8):
    for i in range(n):
        sx = x + i * (w + 1)
        rect(d, sx, y, w, h, C["black"])
        rect(d, sx + 1, y + 1, w - 2, h - 2, C["screen"])
        if kind == "alert" and i % 2 == 0:
            rect(d, sx + 2, y + 2, w - 4, 2, C["red"])
            rect(d, sx + 2, y + 5, 4, 1, C["orange"])
        elif kind == "map":
            for k in range(4):
                px(d, sx + random.randint(2, w - 3), y + random.randint(2, h - 3), C["red"] if k == 0 else C["green"])
        else:
            for k in range(3):
                bh = random.randint(1, h - 3)
                rect(d, sx + 2 + k * 3, y + h - 1 - bh, 2, bh, [C["green"], C["cyan"], C["yellow"]][k])


def server_rack(d, x, y, h=30):
    rect(d, x, y, 10, h, C["rack"])
    outline(d, x, y, 10, h, C["black"])
    for i in range(2, h - 2, 3):
        rect(d, x + 1, y + i, 8, 2, (48, 54, 64))
        px(d, x + 2, y + i, C["green"] if random.random() < 0.7 else C["orange"])
        px(d, x + 4, y + i, C["green"] if random.random() < 0.5 else C["blue"])


def table(d, x, y, w, c=None):
    c = c or C["wood"]
    rect(d, x, y, w, 4, c)
    rect(d, x, y + 4, w, 1, C["wood_d"])
    rect(d, x + 2, y + 5, 1, 5, C["wood_d"])
    rect(d, x + w - 3, y + 5, 1, 5, C["wood_d"])


def papers(d, x, y, n=3):
    for i in range(n):
        rect(d, x + i, y - i * 2, 6, 2, C["paper"])
        px(d, x + i + 1, y - i * 2, C["grey"])


def coffee(d, x, y):
    rect(d, x, y, 3, 3, C["white"])
    px(d, x + 3, y + 1, C["white"])
    px(d, x + 1, y, (90, 60, 40))


def label(d, x, y, text, col):
    """Micro-texte 3×5 (lettres capitales) pour les plaques de porte."""
    font = {
        "A": ["010", "101", "111", "101", "101"], "B": ["110", "101", "110", "101", "110"], "C": ["011", "100", "100", "100", "011"],
        "D": ["110", "101", "101", "101", "110"], "E": ["111", "100", "110", "100", "111"], "F": ["111", "100", "110", "100", "100"],
        "G": ["011", "100", "101", "101", "011"], "H": ["101", "101", "111", "101", "101"], "I": ["111", "010", "010", "010", "111"],
        "J": ["001", "001", "001", "101", "010"], "K": ["101", "101", "110", "101", "101"], "L": ["100", "100", "100", "100", "111"],
        "M": ["101", "111", "111", "101", "101"], "N": ["110", "101", "101", "101", "101"], "O": ["010", "101", "101", "101", "010"],
        "P": ["110", "101", "110", "100", "100"], "Q": ["010", "101", "101", "110", "011"], "R": ["110", "101", "110", "101", "101"],
        "S": ["011", "100", "010", "001", "110"], "T": ["111", "010", "010", "010", "010"], "U": ["101", "101", "101", "101", "111"],
        "V": ["101", "101", "101", "101", "010"], "W": ["101", "101", "111", "111", "101"], "X": ["101", "101", "010", "101", "101"],
        "Y": ["101", "101", "010", "010", "010"], "Z": ["111", "001", "010", "100", "111"], " ": ["000"] * 5, "-": ["000", "000", "111", "000", "000"],
    }
    cx = x
    for ch in text:
        g = font.get(ch, font[" "])
        for r, row in enumerate(g):
            for k, bit in enumerate(row):
                if bit == "1":
                    px(d, cx + k, y + r, col)
        cx += 4


# --- Salles ---------------------------------------------------------------------------
def draw_room(d, key, x, y):
    floor_y = y + RH - 6
    # Mur du fond et sol.
    rect(d, x, y, RW, RH, (68, 78, 98))
    rect(d, x, y, RW, 2, (78, 88, 110))
    rect(d, x, floor_y, RW, 6, C["floor"])
    for i in range(0, RW, 6):
        rect(d, x + i, floor_y, 1, 6, C["floor_l"])
    rect(d, x, floor_y - 1, RW, 1, C["wall_d"])
    tag = {
        "comex": "COMEX", "dsi": "DSI", "finance": "DAF", "rh": "RH", "achats": "ACHATS", "archi": "ARCHI", "devs": "DEVS",
        "projet": "PROJETS", "data": "DATA", "cloud": "CLOUD", "vendor": "VENDOR", "crise": "CRISE", "cyber": "CYBER",
        "soc": "SOC", "datacenter": "DC",
    }[key]
    if key == "comex":
        rect(d, x, floor_y, RW, 6, C["carpet_r"])
        window(d, x + 4, y + 6, 14, 11)
        screen_wall(d, x + 34, y + 6, 1, "dash", 16, 11)
        table(d, x + 10, floor_y - 9, 36, C["wood_d"])
        for i in range(4):
            chair(d, x + 12 + i * 8, floor_y - 6, C["black"])
        plant(d, x + RW - 7, floor_y - 10)
    elif key == "dsi":
        window(d, x + 30, y + 5, 14, 11)
        poster(d, x + 6, y + 6, C["green"])
        desk(d, x + 12, floor_y - 9, 20, True, False)
        chair(d, x + 20, floor_y - 6, C["blue"])
        coffee(d, x + 14, floor_y - 12)
        plant(d, x + RW - 8, floor_y - 10)
        papers(d, x + 26, floor_y - 10, 3)
    elif key == "finance":
        window(d, x + 4, y + 5, 12, 10)
        screen_wall(d, x + 22, y + 6, 2, "dash", 12, 9)
        desk(d, x + 20, floor_y - 9, 22, True, False)
        papers(d, x + 34, floor_y - 10, 5)
        chair(d, x + 28, floor_y - 6, C["grey_d"])
        rect(d, x + 6, floor_y - 12, 8, 12, C["grey"])  # armoire
        for i in range(3):
            rect(d, x + 7, floor_y - 11 + i * 4, 6, 1, C["grey_d"])
    elif key == "rh":
        poster(d, x + 6, y + 6, C["purple"])
        poster(d, x + 16, y + 6, C["orange"])
        rect(d, x + 30, floor_y - 7, 18, 4, C["purple"])  # canapé
        rect(d, x + 30, floor_y - 10, 18, 3, (140, 90, 180))
        rect(d, x + 30, floor_y - 3, 1, 3, C["black"])
        rect(d, x + 47, floor_y - 3, 1, 3, C["black"])
        desk(d, x + 6, floor_y - 9, 18, True, False)
        coffee(d, x + 8, floor_y - 12)
    elif key == "achats":
        window(d, x + 36, y + 5, 12, 10)
        desk(d, x + 8, floor_y - 9, 22, True, False)
        papers(d, x + 10, floor_y - 10, 6)
        chair(d, x + 18, floor_y - 6)
        # balance
        rect(d, x + 40, floor_y - 12, 1, 8, C["metal"])
        rect(d, x + 36, floor_y - 12, 9, 1, C["metal"])
        rect(d, x + 35, floor_y - 11, 3, 1, C["yellow"])
        rect(d, x + 43, floor_y - 10, 3, 1, C["yellow"])
        rect(d, x + 38, floor_y - 4, 5, 1, C["metal"])
    elif key == "archi":
        whiteboard(d, x + 6, y + 5, 26, 15, "archi")
        table(d, x + 34, floor_y - 9, 16)
        papers(d, x + 36, floor_y - 10, 2)
        chair(d, x + 40, floor_y - 6)
        plant(d, x + 2, floor_y - 10)
    elif key == "devs":
        for i in range(3):
            desk(d, x + 2 + i * 18, floor_y - 9, 16, True, True)
            chair(d, x + 8 + i * 18, floor_y - 6, [C["green"], C["blue"], C["orange"]][i])
        rect(d, x + 4, y + 6, 10, 6, C["paper"])  # post-its
        px(d, x + 46, floor_y - 11, C["yellow"])  # canard
        px(d, x + 47, floor_y - 11, C["yellow"])
        px(d, x + 47, floor_y - 12, C["yellow"])
        px(d, x + 48, floor_y - 12, C["orange"])
    elif key == "projet":
        whiteboard(d, x + 4, y + 5, 24, 15, "kanban")
        whiteboard(d, x + 32, y + 6, 18, 12, "chart")
        table(d, x + 12, floor_y - 9, 30)
        for i in range(3):
            chair(d, x + 16 + i * 8, floor_y - 6)
    elif key == "data":
        screen_wall(d, x + 4, y + 6, 2, "dash", 12, 9)
        rect(d, x + 32, y + 8, 18, 10, (40, 90, 140))  # « lake »
        for i in range(4):
            rect(d, x + 33 + i * 4, y + 10 + (i % 2) * 3, 3, 1, C["cyan"])
        desk(d, x + 6, floor_y - 9, 20, True, False)
        chair(d, x + 14, floor_y - 6, C["cyan"])
        server_rack(d, x + 42, floor_y - 18, 18)
    elif key == "cloud":
        screen_wall(d, x + 3, y + 4, 4, "dash", 11, 9)
        # nuage
        for (cx, cy, r) in [(24, 22, 4), (29, 20, 5), (34, 22, 4)]:
            d.ellipse((x + cx - r, y + cy - r, x + cx + r, y + cy + r), fill=C["white"])
        desk(d, x + 6, floor_y - 9, 40, True, False)
        chair(d, x + 22, floor_y - 6, C["blue"])
    elif key == "vendor":
        rect(d, x, floor_y, RW, 6, C["carpet_b"])
        rect(d, x + 6, y + 5, 18, 12, C["orange"])  # bannière « partenaire stratégique »
        rect(d, x + 7, y + 6, 16, 10, (255, 196, 110))
        rect(d, x + 9, y + 8, 12, 2, C["white"])
        rect(d, x + 9, y + 12, 8, 1, C["wood_d"])
        rect(d, x + 34, y + 8, 12, 14, C["white"])  # paperboard
        rect(d, x + 39, y + 22, 1, 10, C["metal"])
        table(d, x + 8, floor_y - 9, 24)
        coffee(d, x + 12, floor_y - 12)
        coffee(d, x + 24, floor_y - 12)
        chair(d, x + 14, floor_y - 6)
        chair(d, x + 24, floor_y - 6)
    elif key == "crise":
        rect(d, x, y, RW, RH - 6, (88, 52, 60))
        screen_wall(d, x + 6, y + 6, 3, "alert", 12, 9)
        table(d, x + 8, floor_y - 9, 40, C["wood_d"])
        for i in range(4):
            chair(d, x + 10 + i * 9, floor_y - 6, C["red"])
        rect(d, x + 48, y + 4, 5, 5, C["white"])  # horloge
        px(d, x + 50, y + 5, C["black"])
        px(d, x + 50, y + 6, C["black"])
        px(d, x + 51, y + 6, C["black"])
    elif key == "cyber":
        poster(d, x + 6, y + 6, C["red"])
        # cadenas
        rect(d, x + 20, y + 10, 8, 6, C["yellow"])
        outline(d, x + 21, y + 6, 6, 5, C["metal"])
        desk(d, x + 6, floor_y - 9, 20, True, True)
        desk(d, x + 30, floor_y - 9, 20, True, False)
        chair(d, x + 14, floor_y - 6, C["black"])
        chair(d, x + 38, floor_y - 6, C["black"])
    elif key == "soc":
        rect(d, x, y, RW, RH - 6, (40, 48, 62))
        screen_wall(d, x + 2, y + 4, 4, "map", 12, 9)
        screen_wall(d, x + 8, y + 14, 3, "alert", 12, 6)
        desk(d, x + 6, floor_y - 9, 44, False)
        chair(d, x + 18, floor_y - 6, C["red"])
        chair(d, x + 34, floor_y - 6, C["red"])
    elif key == "datacenter":
        rect(d, x, y, RW, RH - 6, (36, 40, 52))
        for i in range(4):
            server_rack(d, x + 3 + i * 12, y + 4, 30)
        rect(d, x + RW - 6, y + 6, 5, 14, C["white"])  # clim
        for i in range(4):
            rect(d, x + RW - 5, y + 8 + i * 3, 3, 1, C["grey"])
    # Plaque de porte.
    w = len(tag) * 4 + 1
    rect(d, x + 1, y + 1, w + 1, 7, C["black"])
    label(d, x + 2, y + 2, tag, C["yellow"])


ROOMS = ["comex", "dsi", "finance", "rh", "achats", "archi", "devs", "projet", "data", "cloud", "vendor", "crise", "cyber", "soc", "datacenter"]


def gen_building():
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    bx = MARGIN_X
    # Trottoir.
    rect(d, 0, H - STREET, W, STREET, (46, 50, 56))
    rect(d, 0, H - STREET, W, 1, (90, 94, 100))
    for i in range(0, W, 14):
        rect(d, i, H - 4, 7, 1, (70, 74, 80))
    # Structure.
    rect(d, bx, ROOF, BW, BH, C["wall"])
    rect(d, bx - 2, ROOF - 3, BW + 4, 3, C["wall_l"])
    # Toit : enseigne et antenne.
    rect(d, bx + 70, 4, BW - 140, 14, C["ink"])
    outline(d, bx + 70, 4, BW - 140, 14, C["wall_l"])
    rect(d, bx + 90, 18, 2, ROOF - 21, C["metal"])
    rect(d, bx + BW - 92, 18, 2, ROOF - 21, C["metal"])
    rect(d, bx + 20, 2, 1, ROOF - 5, C["metal"])  # antenne
    px(d, bx + 20, 1, C["red"])
    rect(d, bx + 16, 8, 9, 1, C["metal"])
    rect(d, bx + BW - 40, ROOF - 12, 14, 9, C["grey_d"])  # groupe froid
    for i in range(3):
        rect(d, bx + BW - 38 + i * 4, ROOF - 10, 2, 5, C["grey"])
    for f in range(FLOORS):
        for c in range(COLS):
            x, y = room_origin(c, f)
            draw_room(d, ROOMS[f * COLS + c], x, y)
    # Dalles, poteaux.
    for f in range(FLOORS + 1):
        y = ROOF + f * (RH + WALL)
        rect(d, bx, y, BW, WALL, C["wall_d"])
    for c in range(COLS + 1):
        x = bx + c * (RW + WALL)
        rect(d, x, ROOF, WALL, BH, C["wall_d"])
    # Porte d'entrée (rez-de-chaussée, à gauche).
    rect(d, bx - 8, H - STREET - 14, 8, 14, (60, 70, 90))
    rect(d, bx - 7, H - STREET - 13, 6, 13, C["glass"])
    # Arbres.
    for tx in [2, W - 10]:
        rect(d, tx + 3, H - STREET - 6, 2, 6, C["wood_d"])
        d.ellipse((tx, H - STREET - 16, tx + 8, H - STREET - 6), fill=C["plant_d"])
        d.ellipse((tx + 1, H - STREET - 15, tx + 6, H - STREET - 9), fill=C["plant"])
    img.save(os.path.join(ROOT, "assets/sprites/building.png"))


def gen_sky():
    w, h = 480, 270
    img = Image.new("RGBA", (w, h))
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / h
        c = (int(14 + 20 * t), int(18 + 22 * t), int(34 + 30 * t))
        d.line((0, y, w, y), fill=c)
    for _ in range(90):
        x, y = random.randint(0, w - 1), random.randint(0, h // 2)
        px(d, x, y, (200, 210, 230) if random.random() < 0.3 else (120, 130, 160))
    # Skyline.
    x = 0
    while x < w:
        bw = random.randint(14, 34)
        bh = random.randint(40, 130)
        col = (24 + random.randint(0, 10), 28 + random.randint(0, 10), 44 + random.randint(0, 12))
        rect(d, x, h - bh, bw, bh, col)
        for wy in range(h - bh + 4, h - 6, 6):
            for wx in range(x + 3, x + bw - 3, 5):
                if random.random() < 0.25:
                    rect(d, wx, wy, 2, 2, (220, 190, 110) if random.random() < 0.7 else (120, 170, 220))
        x += bw + random.randint(0, 6)
    # Lune.
    d.ellipse((400, 30, 414, 44), fill=(236, 230, 200))
    d.ellipse((404, 28, 418, 42), fill=(20, 24, 42))
    img.save(os.path.join(ROOT, "assets/sprites/sky.png"))


# --- Salariés (10×16, 4 images : marche ×2, debout, assis) ------------------------
LOOKS = [
    ((34, 50, 90), (40, 30, 20), "skin1"), ((120, 40, 50), (20, 20, 20), "skin3"), ((40, 90, 60), (150, 90, 40), "skin2"),
    ((70, 70, 80), (200, 200, 190), "skin1"), ((90, 60, 130), (30, 20, 10), "skin4"), ((200, 120, 40), (90, 50, 20), "skin2"),
    ((30, 30, 34), (220, 180, 90), "skin1"), ((50, 120, 160), (40, 25, 15), "skin3"),
]


def gen_people():
    fw, fh = 10, 16
    img = Image.new("RGBA", (fw * 4, fh * len(LOOKS)), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for i, (shirt, hair, skin) in enumerate(LOOKS):
        for f in range(4):
            ox, oy = f * fw, i * fh
            sk = C[skin]
            # tête
            rect(d, ox + 3, oy + 1, 4, 4, sk)
            rect(d, ox + 3, oy, 4, 2, hair)
            px(d, ox + 2, oy + 1, hair)
            px(d, ox + 6, oy + 3, C["ink"])
            # corps
            rect(d, ox + 2, oy + 5, 6, 6, shirt)
            px(d, ox + 4, oy + 5, C["white"])
            # bras
            if f in (0, 1):
                rect(d, ox + 1, oy + 6 + f, 1, 4, sk)
                rect(d, ox + 8, oy + 7 - f, 1, 4, sk)
            else:
                rect(d, ox + 1, oy + 6, 1, 4, shirt)
                rect(d, ox + 8, oy + 6, 1, 4, shirt)
            # jambes
            leg = (40, 44, 60)
            if f == 0:
                rect(d, ox + 3, oy + 11, 1, 5, leg)
                rect(d, ox + 6, oy + 11, 1, 4, leg)
            elif f == 1:
                rect(d, ox + 3, oy + 11, 1, 4, leg)
                rect(d, ox + 6, oy + 11, 1, 5, leg)
            elif f == 2:
                rect(d, ox + 3, oy + 11, 1, 5, leg)
                rect(d, ox + 6, oy + 11, 1, 5, leg)
            else:
                rect(d, ox + 3, oy + 11, 4, 2, leg)
                rect(d, ox + 6, oy + 13, 1, 3, leg)
            px(d, ox + 3, oy + 15, C["black"])
            px(d, ox + 6, oy + 15, C["black"])
    img.save(os.path.join(ROOT, "assets/sprites/people.png"))


def gen_fx():
    # 0 : bulle « ! » jaune, 1 : bulle rouge (incident), 2 : bulle bleue (fournisseur), 3 : voyant vert, 4 : voyant rouge
    img = Image.new("RGBA", (13 * 5, 14), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for i, col in enumerate([C["yellow"], C["red"], C["blue"]]):
        ox = i * 13
        rect(d, ox + 1, 0, 11, 10, C["ink"])
        rect(d, ox + 2, 1, 9, 8, col)
        rect(d, ox + 5, 10, 3, 2, C["ink"])
        px(d, ox + 6, 12, C["ink"])
        rect(d, ox + 6, 2, 1, 4, C["ink"])
        px(d, ox + 6, 7, C["ink"])
    rect(d, 39 + 5, 5, 3, 3, C["green"])
    rect(d, 52 + 5, 5, 3, 3, C["red"])
    img.save(os.path.join(ROOT, "assets/sprites/fx.png"))


# --- Portraits 32×32 -----------------------------------------------------------------
PORTRAITS = [
    # (peau, cheveux, style, vêtement, accent, lunettes, barbe, femme)
    ("skin1", (214, 196, 150), "bob", (40, 40, 52), (240, 240, 240), False, False, True),       # 0 DG
    ("skin1", (150, 150, 150), "bald", (60, 60, 70), (250, 214, 92), True, False, False),       # 1 DAF
    ("skin3", (30, 22, 18), "pony", (24, 24, 30), (230, 80, 80), False, False, True),            # 2 RSSI
    ("skin2", (90, 60, 30), "slick", (40, 70, 140), (240, 240, 240), False, False, False),      # 3 Dir. commercial
    ("skin1", (190, 80, 40), "curly", (120, 70, 140), (200, 160, 220), True, False, True),      # 4 DRH
    ("skin2", (20, 20, 24), "messy", (40, 110, 60), (124, 255, 82), False, False, False),       # 5 Dev senior
    ("skin1", (220, 190, 120), "slick", (30, 36, 70), (200, 200, 230), False, False, False),    # 6 Consultant
    ("skin2", (60, 40, 20), "slick", (70, 70, 80), (246, 162, 60), False, False, False),        # 7 Commercial
    ("skin1", (110, 70, 40), "short", (90, 110, 140), (250, 214, 92), False, True, False),      # 8 ESN
    ("skin1", (170, 170, 165), "short", (50, 120, 120), (159, 211, 199), True, True, False),    # 9 Architecte
    ("skin2", (20, 20, 24), "bob", (60, 120, 60), (199, 224, 123), False, False, True),         # 10 Achats
    ("skin4", (20, 16, 14), "curly", (130, 40, 40), (255, 123, 123), False, False, False),      # 11 SOC
    ("skin1", (110, 70, 40), "bun", (40, 140, 150), (123, 224, 224), True, False, True),        # 12 Data
    None,                                                                                       # 13 notification
    ("skin2", (40, 28, 20), "long", (30, 40, 70), (240, 240, 240), False, False, True),         # 14 DSI femme (anciennes sauvegardes)
    ("skin1", (40, 30, 20), "short", (30, 40, 70), (240, 240, 240), False, False, False),       # 15 DSI homme (anciennes sauvegardes)
]

# Avatars du ou de la DSI (slots 16–31) : 0–7 directrices, 8–15 directeurs.
# (peau, cheveux, style, vêtement, accent, lunettes, barbe, femme) — partagé avec gen_iso.py.
AVATARS = [
    ("skin1", (54, 36, 24), "long", (30, 40, 70), (240, 240, 240), False, False, True),
    ("skin4", (24, 18, 14), "afro", (110, 40, 60), (250, 214, 92), False, False, True),
    ("skin2", (40, 110, 120), "hijab", (40, 50, 80), (240, 240, 240), False, False, True),
    ("skin0", (226, 196, 120), "bob", (60, 60, 72), (240, 240, 240), True, False, True),
    ("skin5", (20, 16, 14), "braids", (40, 100, 70), (250, 214, 92), False, False, True),
    ("skin3", (40, 26, 18), "bun", (90, 60, 110), (240, 240, 240), True, False, True),
    ("skin1", (170, 70, 36), "curly", (40, 70, 110), (240, 240, 240), False, False, True),
    ("skin2", (18, 18, 22), "pony", (70, 40, 40), (240, 240, 240), True, False, True),
    ("skin1", (54, 36, 24), "short", (30, 40, 70), (240, 240, 240), False, False, False),
    ("skin4", (20, 16, 14), "short", (60, 60, 72), (240, 240, 240), False, True, False),
    ("skin3", (24, 18, 14), "curly", (40, 90, 70), (240, 240, 240), False, True, False),
    ("skin0", (226, 196, 120), "slick", (40, 40, 52), (110, 160, 230), True, False, False),
    ("skin5", (20, 16, 14), "bald", (90, 50, 50), (240, 240, 240), False, True, False),
    ("skin2", (18, 18, 22), "messy", (40, 60, 100), (240, 240, 240), True, False, False),
    ("skin1", (168, 168, 160), "short", (70, 70, 80), (240, 240, 240), True, True, False),
    ("skin3", (24, 18, 14), "locs", (110, 80, 40), (240, 240, 240), False, False, False),
]
PORTRAITS = PORTRAITS + AVATARS


def portrait(d, ox, spec, bg):
    rect(d, ox, 0, 32, 32, bg)
    for i in range(0, 32, 4):
        rect(d, ox, i, 32, 1, tuple(max(0, v - 8) for v in bg))
    if spec is None:
        # Icône de messagerie.
        rect(d, ox + 6, 7, 20, 14, (110, 130, 230))
        rect(d, ox + 7, 8, 18, 12, (140, 160, 250))
        rect(d, ox + 9, 21, 4, 3, (110, 130, 230))
        for i in range(3):
            rect(d, ox + 10 + i * 5, 13, 3, 3, C["white"])
        return
    skin, hair, style, cloth, accent, glasses, beard, female = spec
    sk = C[skin]
    shade = tuple(max(0, v - 30) for v in sk)
    # Épaules / vêtement.
    rect(d, ox + 5, 25, 22, 7, cloth)
    rect(d, ox + 3, 28, 26, 4, cloth)
    rect(d, ox + 13, 24, 6, 3, sk)
    if female:
        rect(d, ox + 12, 25, 8, 3, accent)
    else:
        rect(d, ox + 13, 25, 6, 2, C["white"])
        rect(d, ox + 15, 26, 2, 6, accent)
    # Tête.
    rect(d, ox + 10, 8, 12, 15, sk)
    rect(d, ox + 9, 11, 1, 8, sk)
    rect(d, ox + 22, 11, 1, 8, sk)
    rect(d, ox + 11, 22, 10, 2, shade)
    px(d, ox + 8, 14, sk)
    px(d, ox + 23, 14, sk)
    # Yeux, bouche.
    rect(d, ox + 12, 14, 2, 2, C["ink"])
    rect(d, ox + 18, 14, 2, 2, C["ink"])
    px(d, ox + 12, 14, C["white"])
    px(d, ox + 18, 14, C["white"])
    rect(d, ox + 14, 19, 4, 1, (150, 70, 70))
    px(d, ox + 16, 17, shade)
    if glasses:
        for gx in (11, 17):
            rect(d, ox + gx, 13, 4, 4, (190, 210, 225))
            outline(d, ox + gx, 13, 4, 4, (40, 40, 48))
            px(d, ox + gx + 1, 14, C["ink"])
            px(d, ox + gx + 2, 15, C["ink"])
        rect(d, ox + 15, 14, 2, 1, (40, 40, 48))
    if beard:
        rect(d, ox + 10, 19, 12, 4, hair)
        rect(d, ox + 14, 19, 4, 1, (150, 70, 70))
    # Cheveux.
    if style == "bald":
        rect(d, ox + 9, 10, 2, 5, hair)
        rect(d, ox + 21, 10, 2, 5, hair)
    elif style in ("short", "slick", "messy"):
        rect(d, ox + 9, 6, 14, 4, hair)
        rect(d, ox + 9, 9, 2, 4, hair)
        rect(d, ox + 21, 9, 2, 4, hair)
        if style == "slick":
            rect(d, ox + 10, 5, 12, 1, hair)
            px(d, ox + 20, 10, hair)
        if style == "messy":
            for k in range(6):
                px(d, ox + 10 + k * 2, 5, hair)
    elif style == "bob":
        rect(d, ox + 9, 6, 14, 4, hair)
        rect(d, ox + 8, 9, 3, 12, hair)
        rect(d, ox + 21, 9, 3, 12, hair)
    elif style == "long":
        rect(d, ox + 9, 6, 14, 4, hair)
        rect(d, ox + 7, 9, 4, 17, hair)
        rect(d, ox + 21, 9, 4, 17, hair)
    elif style == "pony":
        rect(d, ox + 9, 6, 14, 4, hair)
        rect(d, ox + 9, 9, 2, 3, hair)
        rect(d, ox + 22, 8, 4, 10, hair)
    elif style == "bun":
        rect(d, ox + 9, 6, 14, 4, hair)
        rect(d, ox + 13, 2, 6, 4, hair)
        rect(d, ox + 9, 9, 2, 4, hair)
        rect(d, ox + 21, 9, 2, 4, hair)
    elif style == "afro":
        d.ellipse((ox + 4, 0, ox + 27, 17), fill=hair)
        rect(d, ox + 6, 10, 4, 9, hair)
        rect(d, ox + 22, 10, 4, 9, hair)
        # Le visage repasse devant la chevelure.
        rect(d, ox + 10, 11, 12, 12, sk)
        rect(d, ox + 11, 22, 10, 2, shade)
        rect(d, ox + 12, 14, 2, 2, C["ink"])
        rect(d, ox + 18, 14, 2, 2, C["ink"])
        px(d, ox + 12, 14, C["white"])
        px(d, ox + 18, 14, C["white"])
        rect(d, ox + 14, 19, 4, 1, (150, 70, 70))
        px(d, ox + 16, 17, shade)
    elif style == "braids":
        rect(d, ox + 9, 6, 14, 4, hair)
        for bx in (7, 9, 21, 23):
            for by in range(9, 27, 2):
                px(d, ox + bx, by, hair)
                px(d, ox + bx + 1, by + 1, tuple(min(255, v + 30) for v in hair))
    elif style == "locs":
        rect(d, ox + 9, 5, 14, 5, hair)
        for bx in (8, 10, 20, 22, 12):
            rect(d, ox + bx, 8, 2, 14 if bx != 12 else 4, hair)
    elif style == "hijab":
        scarf = hair
        d.ellipse((ox + 7, 3, ox + 24, 20), fill=scarf)
        rect(d, ox + 7, 12, 18, 14, scarf)
        rect(d, ox + 5, 22, 22, 6, scarf)
        rect(d, ox + 11, 9, 10, 13, sk)
        rect(d, ox + 12, 14, 2, 2, C["ink"])
        rect(d, ox + 18, 14, 2, 2, C["ink"])
        px(d, ox + 12, 14, C["white"])
        px(d, ox + 18, 14, C["white"])
        rect(d, ox + 14, 19, 4, 1, (150, 70, 70))
        if glasses:
            for gx in (11, 17):
                outline(d, ox + gx, 13, 4, 4, (40, 40, 48))
    elif style == "curly":
        for k in range(8):
            d.ellipse((ox + 8 + k * 2, 4 + (k % 2), ox + 11 + k * 2, 8 + (k % 2)), fill=hair)
        rect(d, ox + 8, 8, 3, 8, hair)
        rect(d, ox + 21, 8, 3, 8, hair)
    # Badge (DSI) / collier (DG).
    if accent == (240, 240, 240) and female:
        for k in range(4):
            px(d, ox + 13 + k * 2, 27, C["white"])


def gen_portraits():
    n = len(PORTRAITS)
    n = 32
    img = Image.new("RGBA", (32 * n, 32))
    d = ImageDraw.Draw(img)
    bgs = [(70, 58, 40), (70, 66, 36), (70, 36, 40), (36, 50, 76), (60, 44, 70), (34, 66, 40), (50, 54, 80), (76, 56, 36),
           (72, 66, 40), (40, 70, 66), (52, 66, 36), (76, 40, 40), (36, 66, 70), (44, 48, 70), (34, 46, 70), (34, 46, 70)]
    for i, spec in enumerate(PORTRAITS):
        bg = bgs[i % len(bgs)] if i < 16 else (34, 46, 70)
        portrait(d, i * 32, spec, bg)
    img.save(os.path.join(ROOT, "assets/ui/portraits.png"))
    # Favicon : le portrait DSI.
    fav = img.crop((14 * 32, 0, 15 * 32, 32)).resize((64, 64), Image.NEAREST)
    fav.save(os.path.join(ROOT, "web/favicon.png"))


if __name__ == "__main__":
    for p in ["assets/sprites", "assets/ui", "web"]:
        os.makedirs(os.path.join(ROOT, p), exist_ok=True)
    gen_sky()
    gen_building()
    gen_people()
    gen_fx()
    gen_portraits()
    print(f"building {W}x{H}, salle {RW}x{RH}, origine salle 0 = {room_origin(0, 0)}")
