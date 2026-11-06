"""Перерисовывает пиктограммы Taskify из Lucide.

Иконки Lucide — открытый набор (ISC), штриховые, 24×24. Скрипт скачивает
нужные SVG, красит их в белый и складывает PNG в assets/icons. Белый цвет
нужен потому, что egui тонирует монохромные иконки цветом темы.
"""

from __future__ import annotations

import io
import pathlib
import sys

import requests
import resvg_py
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "icons"
RAW = "https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/{name}.svg"

# Ключ PNG -> имя иконки Lucide.
ICONS: dict[str, str] = {
    # Навигация.
    "nav_today": "sun",
    "nav_week": "calendar-days",
    "nav_packs": "package",
    "nav_achievements": "trophy",
    "nav_market": "shopping-basket",
    "nav_stock": "archive",
    "nav_history": "rotate-ccw-clock",
    "nav_settings": "settings",
    # Кнопки.
    "ui_check": "check",
    "ui_cross": "x",
    "ui_skip": "skip-forward",
    "ui_question": "circle-question-mark",
    "ui_star_off": "star",
    "ui_star_on": "star",
    "ui_start": "play",
    "ui_stop": "square",
    "ui_coin": "coins",
    # Пиктограммы заданий.
    "glyph_1F308": "rainbow",
    "glyph_1F319": "moon",
    "glyph_1F31F": "star",
    "glyph_1F32C": "wind",
    "glyph_1F33B": "flower",
    "glyph_1F33F": "leafy-green",
    "glyph_1F340": "clover",
    "glyph_1F343": "leaf",
    "glyph_1F369": "donut",
    "glyph_1F373": "cooking-pot",
    "glyph_1F381": "gift",
    "glyph_1F389": "party-popper",
    "glyph_1F3AF": "target",
    "glyph_1F3C3": "footprints",
    "glyph_1F3CA": "waves-horizontal",
    "glyph_1F431": "cat",
    "glyph_1F451": "crown",
    "glyph_1F45F": "sport-shoe",
    "glyph_1F48A": "pill",
    "glyph_1F48E": "gem",
    "glyph_1F496": "heart",
    "glyph_1F4A7": "droplet",
    "glyph_1F4AA": "biceps-flexed",
    "glyph_1F4B0": "hand-coins",
    "glyph_1F4B3": "credit-card",
    "glyph_1F4C4": "file-text",
    "glyph_1F4C8": "chart-line",
    "glyph_1F4D6": "book-open",
    "glyph_1F4DA": "library",
    "glyph_1F4DE": "smartphone",
    "glyph_1F4F5": "phone-off",
    "glyph_1F525": "flame",
    "glyph_1F5E3": "languages",
    "glyph_1F680": "rocket",
    "glyph_1F6B0": "droplets",
    "glyph_1F6B4": "bike",
    "glyph_1F6B6": "route",
    "glyph_1F6C1": "bath",
    "glyph_1F6D2": "shopping-cart",
    "glyph_1F938": "person-standing",
    "glyph_1F957": "salad",
    "glyph_1F9CA": "refrigerator",
    "glyph_1F9D8": "flower-2",
    "glyph_1F9F3": "luggage",
    "glyph_1F9F9": "broom",
    "glyph_1F9FA": "shirt",
    "glyph_1F9FE": "receipt",
    "glyph_1FA7A": "stethoscope",
    "glyph_1FA9F": "app-window",
    "glyph_23F1": "timer",
    "glyph_2600": "sun",
    "glyph_2604": "cloud-rain",
    "glyph_2615": "coffee",
    "glyph_270D": "pen-line",
    "glyph_2728": "sparkles",
}

FILLED = {"ui_star_on"}
SIZE = 128


def download(name: str) -> str:
    url = RAW.format(name=name)
    response = requests.get(url, timeout=30)
    response.raise_for_status()
    return response.text


def paint(svg: str, filled: bool) -> bytes:
    svg = svg.replace('viewBox="0 0 24 24"', 'viewBox="-1 -1 26 26"')
    svg = svg.replace("currentColor", "#FFFFFF")
    if filled:
        svg = svg.replace('fill="none"', 'fill="#FFFFFF"', 1)
    return resvg_py.svg_to_bytes(
        svg_string=svg,
        width=SIZE,
        height=SIZE,
        background="#00000000",
    )


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    cache: dict[str, str] = {}
    for key, lucide in ICONS.items():
        if lucide not in cache:
            cache[lucide] = download(lucide)
        png = paint(cache[lucide], key in FILLED)
        image = Image.open(io.BytesIO(png)).convert("RGBA")
        target = OUT / f"{key}.png"
        image.save(target)
        print(f"{target.name:24} <- {lucide}")
    print(f"готово: {len(ICONS)} иконок")
    return 0


if __name__ == "__main__":
    sys.exit(main())
