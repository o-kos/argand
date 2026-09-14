"""Build size previews from the unchanged B0/B1 raster sketches."""

from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parent
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024)
SOURCES = {"B0": "b0-closed-return-v2.png", "B1": "b1-mutual-weave-v2.png"}
THEMES = {"light": ("#ffffff", "#273244"), "dark": ("#242424", "#eeeeee")}
FONT = ImageFont.truetype("DejaVuSans.ttf", 14)


def build():
    images = {}
    for variant, source in SOURCES.items():
        with Image.open(ROOT.parent / "images" / source) as original:
            for size in SIZES:
                icon = original.convert("RGBA").resize((size, size), Image.Resampling.LANCZOS)
                icon.save(ROOT / f"{variant.lower()}-{size}-transparent.png")
                for theme, (background, _) in THEMES.items():
                    preview = Image.new("RGBA", icon.size, background)
                    preview.alpha_composite(icon)
                    preview = preview.convert("RGB")
                    preview.save(ROOT / f"{variant.lower()}-{size}-{theme}.png")
                    images[variant, size, theme] = preview

    compact_sizes = SIZES[:9]
    heights = [max(size + 38, 72) for size in compact_sizes]
    sheet = Image.new("RGB", (1200, 62 + sum(heights)), "white")
    draw = ImageDraw.Draw(sheet)
    columns = [(variant, theme) for theme in THEMES for variant in SOURCES]
    for column, (variant, theme) in enumerate(columns):
        background, foreground = THEMES[theme]
        x = column * 300
        draw.rectangle((x, 0, x + 299, sheet.height), fill=background)
        draw.text((x + 20, 20), f"{variant} / {theme}", font=FONT, fill=foreground)
        y = 62
        for size, height in zip(compact_sizes, heights):
            draw.text((x + 16, y + 4), f"{size} px", font=FONT, fill=foreground)
            sheet.paste(images[variant, size, theme], (x + (300 - size) // 2, y + 28))
            y += height
    sheet.save(ROOT / "comparison-16-256.png")

    html = ['<!doctype html><html lang="en"><meta charset="utf-8">',
            '<meta name="viewport" content="width=device-width, initial-scale=1">',
            '<title>B0 and B1 size previews</title>',
            '<style>body{font:14px system-ui;background:#e7e9ed;color:#273244;margin:24px}',
            'section{display:flex;gap:12px;width:max-content;margin-bottom:20px}',
            '.panel{padding:16px;background:white}.dark{background:#242424;color:#eee}',
            'img{display:block;max-width:none}h2{font-size:16px}h3{font-size:14px;margin:0 0 16px}',
            'a{color:inherit}</style><h1>B0 and B1: size previews</h1>',
            '<p><a href="../small-b0-v3/index.html">New: B0 optical sizes, 16 and 24 px, compared with the original</a></p>',
            '<p>Unchanged v2 sketches, downsampled with Lanczos. No small-size optical corrections.</p>',
            '<p>View at 100% zoom. Dimensions are CSS pixels; physical pixels depend on display scaling. Scroll horizontally for large sizes.</p>',
            '<p><a href="previews.zip">Download all PNG sizes</a> · <a href="comparison-16-256.png">Compact comparison sheet</a></p>']
    for size in SIZES:
        html.append(f'<h2>{size} × {size}</h2><section>')
        for variant, theme in columns:
            filename = f"{variant.lower()}-{size}-{theme}.png"
            html.append(f'<div class="panel {theme}"><h3>{variant} / {theme}</h3>'
                        f'<a href="{filename}"><img src="{filename}" width="{size}" height="{size}" alt="{variant}, {size} px, {theme} background"></a></div>')
        html.append('</section>')
    html.append('</html>')
    (ROOT / "index.html").write_text("\n".join(html) + "\n")
    with ZipFile(ROOT / "previews.zip", "w", ZIP_DEFLATED) as archive:
        for path in sorted(ROOT.glob("*.png")):
            archive.write(path, path.name)
        archive.write(ROOT / "index.html", "index.html")


if __name__ == "__main__":
    build()
