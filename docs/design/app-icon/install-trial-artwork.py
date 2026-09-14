"""Reproduce the B0 application trial without changing the archived sketches."""

import base64
from pathlib import Path

from PIL import Image


HERE = Path(__file__).resolve().parent
ASSETS = HERE.parents[2] / "crates" / "app" / "assets" / "icons"
SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)


def build():
    with Image.open(HERE / "images" / "b0-closed-return-v2.png") as original:
        original.load()
        frames = {}
        for size in SIZES:
            if size in (16, 24):
                with Image.open(HERE / "small-b0-v3" / f"b0-optical-{size}.png") as optical:
                    frames[size] = optical.convert("RGBA")
            else:
                frames[size] = original.resize((size, size), Image.Resampling.LANCZOS)
            frames[size].save(ASSETS / f"argand-{size}.png")

    frames[256].save(ASSETS / "argand.ico", sizes=[(s, s) for s in SIZES if s <= 256],
                     append_images=[frames[s] for s in SIZES if s < 256])
    frames[1024].save(ASSETS / "argand.icns", append_images=list(frames.values()))

    # Keep scalable theme lookup visually identical to the approved raster trial.
    # A final vector master is still pending; this wrapper is deliberately temporary.
    encoded = base64.b64encode((ASSETS / "argand-1024.png").read_bytes()).decode("ascii")
    (ASSETS / "argand.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
        'width="1024" height="1024" viewBox="0 0 1024 1024">\n'
        '  <!-- B0 application trial: embedded raster; final vector master pending. -->\n'
        f'  <image width="1024" height="1024" xlink:href="data:image/png;base64,{encoded}"/>\n'
        '</svg>\n'
    )


if __name__ == "__main__":
    build()
