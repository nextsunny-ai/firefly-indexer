# Firefly Indexer — app icon generator.
# Identity: a glowing amber firefly spark on a deep night-charcoal tile.
# Deliberately distinct from StoryMaker (different palette + mark).
from PIL import Image, ImageDraw, ImageFilter

S = 1024
RADIUS = 230
CX, CY = S // 2, S // 2

# ── night-charcoal tile (subtle vertical gradient) ────────────────────
bg = Image.new("RGBA", (S, S), (0, 0, 0, 0))
bd = ImageDraw.Draw(bg)
TOP = (40, 38, 58)   # #28263A
BOT = (16, 15, 24)   # #100F18
for y in range(S):
    f = y / S
    bd.line(
        [(0, y), (S, y)],
        fill=(
            int(TOP[0] * (1 - f) + BOT[0] * f),
            int(TOP[1] * (1 - f) + BOT[1] * f),
            int(TOP[2] * (1 - f) + BOT[2] * f),
            255,
        ),
    )

mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=RADIUS, fill=255)

img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
img.paste(bg, (0, 0), mask)


# ── smooth radial halo ────────────────────────────────────────────────
def radial_glow(radius, color, peak=150):
    g = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    gd = ImageDraw.Draw(g)
    steps = 140
    for i in range(steps, 0, -1):
        r = radius * i / steps
        a = int(peak * (1 - i / steps) ** 2.2)
        gd.ellipse([CX - r, CY - r, CX + r, CY + r], fill=color + (a,))
    return g.filter(ImageFilter.GaussianBlur(18))


img.alpha_composite(radial_glow(430, (255, 176, 56)))


# ── four-point spark (firefly) ────────────────────────────────────────
def spark(d, r, color, waist=0.20):
    w = r * waist
    d.polygon([(CX, CY - r), (CX + w, CY), (CX, CY + r), (CX - w, CY)], fill=color)
    d.polygon([(CX - r, CY), (CX, CY + w), (CX + r, CY), (CX, CY - w)], fill=color)


# amber body, faintly soft-edged
body = Image.new("RGBA", (S, S), (0, 0, 0, 0))
spark(ImageDraw.Draw(body), 312, (255, 196, 78, 255))
img.alpha_composite(body.filter(ImageFilter.GaussianBlur(2)))

# bright inner spark — frames the amber tips, lights the centre
inner = Image.new("RGBA", (S, S), (0, 0, 0, 0))
spark(ImageDraw.Draw(inner), 150, (255, 240, 198, 255))
img.alpha_composite(inner.filter(ImageFilter.GaussianBlur(2)))

# tiny white-hot glint at the very centre
glint = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(glint).ellipse([CX - 46, CY - 46, CX + 46, CY + 46],
                              fill=(255, 252, 240, 255))
img.alpha_composite(glint.filter(ImageFilter.GaussianBlur(7)))

# ── re-clip to the rounded tile so nothing bleeds past the edge ───────
out = Image.new("RGBA", (S, S), (0, 0, 0, 0))
out.paste(img, (0, 0), mask)
out.save("app-icon.png")
print("wrote app-icon.png", out.size)
