"""Build B0 icons from the retained raster and optical SVG sources.

Requires Pillow and rsvg-convert. Run from any working directory.
"""

import base64
from io import BytesIO
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

from PIL import Image


HERE = Path(__file__).resolve().parent
SOURCE = HERE / "source"
SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)
SVG = "http://www.w3.org/2000/svg"
ET.register_namespace("", SVG)


def optical_image(size):
    result = subprocess.run(
        ["rsvg-convert", "-w", str(size * 16), "-h", str(size * 16),
         str(SOURCE / f"argand-{size}.svg")],
        check=True, capture_output=True,
    )
    with Image.open(BytesIO(result.stdout)) as image:
        return image.resize((size, size), Image.Resampling.LANCZOS)


def monochrome_svg(optical_size):
    root = ET.parse(SOURCE / f"argand-{optical_size}.svg").getroot()
    root.set("width", "100")
    root.set("height", "100")
    definitions = root.find(f"{{{SVG}}}defs")
    ribbons = root.find(f"{{{SVG}}}g")
    if definitions is None or ribbons is None:
        raise ValueError("Optical SVG must contain definitions and ribbons")
    for crossing in definitions.iter(f"{{{SVG}}}circle"):
        crossing.set("r", "17")
    for group in ribbons.iter(f"{{{SVG}}}g"):
        paths = group.findall(f"{{{SVG}}}path")
        for outline, fill in zip(paths[::2], paths[1::2]):
            outline.set("stroke", "black")
            outline.set("stroke-width", str(float(fill.attrib["stroke-width"]) + 7.5))
            fill.set("stroke", "white")
    root.remove(ribbons)
    mask = ET.SubElement(definitions, f"{{{SVG}}}mask", {
        "id": "mark", "maskUnits": "userSpaceOnUse", "x": "0", "y": "0",
        "width": "100", "height": "100", "style": "mask-type:luminance",
    })
    mask.append(ribbons)
    ET.SubElement(root, f"{{{SVG}}}rect", {
        "width": "100", "height": "100", "fill": "currentColor", "mask": "url(#mark)",
    })
    return ET.tostring(root, encoding="unicode") + "\n"


def build_monochrome():
    (HERE / "argand-mono.svg").write_text(monochrome_svg(24))
    for size in (16, 32, 64):
        svg = monochrome_svg(16 if size == 16 else 24)
        result = subprocess.run(
            ["rsvg-convert", "-w", str(size * 16), "-h", str(size * 16)],
            input=svg.encode(), check=True, capture_output=True,
        )
        with Image.open(BytesIO(result.stdout)) as image:
            image.resize((size, size), Image.Resampling.LANCZOS).save(
                HERE / f"argand-tray-{size}.png"
            )


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

    # Preserve the approved full-size raster rather than substituting the optical redraw.
    encoded = base64.b64encode((HERE / "argand-1024.png").read_bytes()).decode("ascii")
    (HERE / "argand.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
        'width="1024" height="1024" viewBox="0 0 1024 1024">\n'
        '  <!-- B0: embedded raster matching the approved full-size artwork. -->\n'
        f'  <image width="1024" height="1024" xlink:href="data:image/png;base64,{encoded}"/>\n'
        '</svg>\n'
    )
    build_monochrome()


if __name__ == "__main__":
    build()
