"""Render experimental B0 optical sizes and compare with the raster sketch."""

from io import BytesIO
from pathlib import Path
import subprocess

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parent
CYAN = "M42 23 C25 5 10 21 14 34 C17 45 36 50 46 64 C53 76 59 88 72 86 C89 84 93 68 79 56 Z"
RED = "M58 23 C75 5 90 21 86 34 C83 45 64 50 54 64 C47 76 41 88 28 86 C11 84 7 68 21 56 Z"
THEMES = {"light": ("#ffffff", "#273244"), "dark": ("#242424", "#eeeeee")}
FONT = ImageFont.truetype("DejaVuSans.ttf", 14)


def ribbon(path, color, width, border):
    return (f'<path d="{path}" stroke="#061b54" stroke-width="{width + border * 2}"/>'
            f'<path d="{path}" stroke="{color}" stroke-width="{width}"/>')


def build():
    for size, width, border in [(16, 10.4, 1.45), (24, 11.6, 1.25)]:
        red = ribbon(RED, "#ff6756", width, border)
        cyan = ribbon(CYAN, "#06dbdf", width, border)
        # Group opacity isolates the clipped strokes to prevent antialiasing seams.
        svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 100 100">'
               '<defs><clipPath id="over"><circle cx="30" cy="48" r="13"/>'
               '<circle cx="70" cy="48" r="13"/></clipPath></defs>'
               f'<g fill="none" stroke-linejoin="round" stroke-linecap="round">{red}{cyan}'
               f'<g clip-path="url(#over)" opacity="0.999">{red}</g></g></svg>')
        source = ROOT / f"b0-optical-{size}.svg"
        source.write_text(svg + "\n")
        render = subprocess.run(['rsvg-convert', '-w', str(size * 16), '-h', str(size * 16), str(source)],
                                check=True, capture_output=True)
        with Image.open(BytesIO(render.stdout)) as large:
            optical = large.resize((size, size), Image.Resampling.LANCZOS)
            optical.save(ROOT / f"b0-optical-{size}.png")
        with Image.open(ROOT.parent / 'size-preview-v2' / f'b0-{size}-transparent.png') as original:
            for label, icon in [('original', original), ('optical', optical)]:
                for theme, (background, _) in THEMES.items():
                    preview = Image.new('RGBA', (size, size), background)
                    preview.alpha_composite(icon)
                    preview.convert('RGB').save(ROOT / f'b0-{label}-{size}-{theme}.png')

    sheet = Image.new('RGB', (960, 660), 'white')
    draw = ImageDraw.Draw(sheet)
    html = ['<!doctype html><html lang="en"><meta charset="utf-8"><title>B0 optical sizes</title>',
            '<meta name="viewport" content="width=device-width, initial-scale=1">',
            '<style>body{font:14px system-ui;background:#e7e9ed;color:#273244;margin:24px}',
            'section{display:flex;gap:16px;flex-wrap:wrap}.panel{padding:20px;background:white}',
            '.dark{background:#242424;color:#eee}.pair{display:flex;gap:36px;align-items:start}',
            'figure{margin:0}figcaption{margin:12px 0}img{display:block}.zoom{image-rendering:pixelated;margin-top:20px}</style>',
            '<h1>B0: experimental optical sizes</h1>',
            '<p>Original raster reduction versus a vector redraw with flat colors, narrower ribbons and wider openings. Not an approved production master.</p>',
            '<p>Each sample appears at its target CSS size and at 8× nearest-neighbor enlargement. Use 100% browser zoom.</p>']
    for row, size in enumerate((16, 24)):
        html.append(f'<h2>{size} × {size}</h2><section>')
        for column, (theme, (background, foreground)) in enumerate(THEMES.items()):
            x, y = column * 480, row * 330
            draw.rectangle((x, y, x + 479, y + 329), fill=background)
            draw.text((x + 20, y + 16), f'{size} px / {theme}', fill=foreground, font=FONT)
            html.append(f'<div class="panel {theme}"><div class="pair">')
            for index, label in enumerate(('original', 'optical')):
                filename = f'b0-{label}-{size}-{theme}.png'
                with Image.open(ROOT / filename) as preview:
                    px = x + 20 + index * 240
                    draw.text((px, y + 48), label.capitalize(), fill=foreground, font=FONT)
                    sheet.paste(preview, (px, y + 76))
                    sheet.paste(preview.resize((size * 8, size * 8), Image.Resampling.NEAREST), (px, y + 116))
                html.append(f'<figure><figcaption>{label.capitalize()}</figcaption><img src="{filename}" width="{size}" height="{size}" alt="{label}, {size} px">'
                            f'<img class="zoom" src="{filename}" width="{size * 8}" height="{size * 8}" alt="8× pixel enlargement"></figure>')
            html.append('</div></div>')
        html.append('</section>')
    sheet.save(ROOT / 'comparison.png')
    (ROOT / 'index.html').write_text('\n'.join(html) + '\n</html>\n')


if __name__ == '__main__':
    build()
