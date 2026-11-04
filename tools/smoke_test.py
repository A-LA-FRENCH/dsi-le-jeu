#!/usr/bin/env python3
"""Test de fumée navigateur pour le build web de DSI — Le Jeu.

    python tools/smoke_test.py dist [--shots smoke-shots]

Sert le dossier du build, l'ouvre dans Chromium sans affichage (WebGL logiciel via
SwiftShader) puis :
  1. attend le signal `dsi-ready` du chargeur ;
  2. lance une nouvelle partie (écran titre → création → prise de fonctions) ;
  3. vérifie que la sauvegarde automatique est un JSON valide en janvier de l'année 1 ;
  4. bascule sur le plateau DSI isométrique (touche V) ;
  5. recharge, clique sur « Continuer la partie » et vérifie la reprise ;
  6. échoue à la moindre exception non interceptée ou panique wasm.
Les captures sont écrites dans --shots. Variable CHROME pour un exécutable précis.
"""
import argparse, functools, http.server, json, os, sys, threading, time
from pathlib import Path
from playwright.sync_api import sync_playwright

# Mise en page de référence 1280×720 (egui dessine dans le canvas : pas de DOM à cibler).
NEW_GAME = (640, 397)
TO_MENTOR = (450, 655)
START = (340, 641)
CONTINUE = (640, 372)


class Q(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}

    def log_message(self, *a):
        pass

    def handle(self):
        try:
            super().handle()
        except (BrokenPipeError, ConnectionResetError):
            pass


def serve(root):
    h = functools.partial(Q, directory=str(root))
    s = http.server.ThreadingHTTPServer(("127.0.0.1", 0), h)
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return s.server_address[1]


def click(page, xy):
    x, y = xy
    page.mouse.move(x - 2, y)
    time.sleep(0.15)
    page.mouse.move(x, y)
    time.sleep(0.3)
    page.mouse.down()
    time.sleep(0.25)
    page.mouse.up()
    time.sleep(1.0)


def save(page):
    raw = page.evaluate("localStorage.getItem('dsi-le-jeu:autosave')")
    return json.loads(raw) if raw else None


def wait_ready(page, timeout=120):
    t = time.time()
    while time.time() - t < timeout:
        if page.evaluate("window.__DSI_READY === true"):
            return True
        time.sleep(0.5)
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dist", type=Path)
    ap.add_argument("--shots", type=Path, default=Path("smoke-shots"))
    args = ap.parse_args()
    args.shots.mkdir(parents=True, exist_ok=True)
    port = serve(args.dist.resolve())
    url = f"http://127.0.0.1:{port}/?renderer=webgl2&seed=1234"
    errors, console = [], []
    ok = True

    def check(cond, msg):
        nonlocal ok
        print(("OK   " if cond else "ÉCHEC ") + msg)
        ok &= bool(cond)

    with sync_playwright() as p:
        kw = {"executable_path": os.environ["CHROME"]} if os.environ.get("CHROME") else {}
        b = p.chromium.launch(headless=True, args=["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"], **kw)
        page = b.new_page(viewport={"width": 1280, "height": 720})
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.on("console", lambda m: console.append(f"[{m.type}] {m.text}"))
        page.goto(url)
        check(wait_ready(page), "le jeu signale qu'il est prêt")
        time.sleep(3)
        page.screenshot(path=str(args.shots / "1-title.png"))
        for _ in range(3):
            click(page, NEW_GAME)
            time.sleep(1.5)
            click(page, TO_MENTOR)
            time.sleep(1.5)
            click(page, START)
            time.sleep(2.5)
            if save(page):
                break
        s = save(page)
        check(s is not None and s.get("version") == 1, "sauvegarde automatique après la prise de fonctions")
        if s:
            cal = s["game"]["cal"]
            check(cal == {"year": 1, "month": 1}, f"la partie commence en janvier année 1 ({cal})")
        page.screenshot(path=str(args.shots / "2-siege.png"))
        if s:
            check(s["game"]["tutorial"] == 0, "le tutoriel démarre")
            check(s["game"]["profile"]["mentor"] == "mehdi", "le mentor par défaut est choisi")
        page.keyboard.press("KeyV")
        time.sleep(3)
        page.screenshot(path=str(args.shots / "3-plateau.png"))
        page.reload()
        check(wait_ready(page), "rechargement")
        time.sleep(3)
        click(page, CONTINUE)
        time.sleep(2)
        page.screenshot(path=str(args.shots / "4-continue.png"))
        check(save(page) is not None, "la sauvegarde survit au rechargement")
        fatal = [e for e in errors if "Using exceptions for control flow" not in e]
        check(not fatal, f"aucune erreur JavaScript/wasm ({fatal[:3]})")
        (args.shots / "console.log").write_text("\n".join(console))
        b.close()
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
