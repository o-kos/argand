"""Build colored B0 icons from the retained raster and optical SVG sources.

Requires Pillow and rsvg-convert. Run from any working directory.
"""

import base64
from io import BytesIO
from pathlib import Path
import subprocess

from PIL import Image


HERE = Path(__file__).resolve().parent
SOURCE = HERE / "source"
SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)


def optical_image(size):
    result = subprocess.run(
        ["rsvg-convert", "-w", str(size * 16), "-h", str(size * 16),
         str(SOURCE / f"argand-{size}.svg")],
        check=True, capture_output=True,
    )
    with Image.open(BytesIO(result.stdout)) as image:
        return image.resize((size, size), Image.Resampling.LANCZOS)


def build():
    with Image.open(SOURCE / "argand.png") as original:
        frames = {
            size: optical_image(size) if size in (16, 24)
            else original.resize((size, size), Image.Resampling.LANCZOS)
            for size in SIZES
        }
    for size, frame in frames.items():
        frame.save(HERE / f"argand-{size}.png")
    frames[256].save(HERE / "argand.ico", sizes=[(s, s) for s in SIZES if s <= 256],
                     append_images=[frames[s] for s in SIZES if s < 256])
    frames[1024].save(HERE / "argand.icns", append_images=list(frames.values()))

    # Keep scalable theme lookup identical to the selected raster until the vector master exists.
    encoded = base64.b64encode((HERE / "argand-1024.png").read_bytes()).decode("ascii")
    (HERE / "argand.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
        'width="1024" height="1024" viewBox="0 0 1024 1024">\n'
        '  <!-- B0 application trial: embedded raster; final vector master pending. -->\n'
        f'  <image width="1024" height="1024" xlink:href="data:image/png;base64,{encoded}"/>\n'
        '</svg>\n'
    )


if __name__ == "__main__":
    build()
