#!/usr/bin/env python3
"""Screen-style demo of Gate Install: 6s transparent logo, then the launcher."""

import subprocess
from pathlib import Path

import imageio_ffmpeg
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path("/home/user/rus-launcher")
ASSETS = ROOT / "gate-install" / "assets"
OUT = ROOT / "install" / "gate-install-demo.mp4"
FRAMES = Path("/tmp/gate-demo-frames")

W, H = 1600, 1000
FPS = 30
DESIGN = (420, 728)
SCALE = 1.35  # window on the desktop, still the real layout
DW, DH = int(DESIGN[0] * SCALE), int(DESIGN[1] * SCALE)
OX = (W - DW) // 2
OY = (H - DH) // 2 - 6

FONT_SB = str(ASSETS / "Inter-SemiBold.ttf")
FONT_MD = str(ASSETS / "Inter-Medium.ttf")
FONT_RG = str(ASSETS / "Inter-Regular.ttf")


def smooth(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3 - 2 * t)


def font(path, px):
    return ImageFont.truetype(path, max(1, int(round(px))))


def tracked(draw, text, xy, fnt, fill, tracking, anchor_center=True):
    widths = [draw.textlength(ch, font=fnt) for ch in text]
    total = sum(widths) + tracking * (len(text) - 1)
    x = xy[0] - (total / 2 if anchor_center else 0)
    y = xy[1]
    for ch, w in zip(text, widths):
        draw.text((x, y), ch, font=fnt, fill=fill, anchor="lt")
        x += w + tracking


def render_window(mode):
    """mode: normal | hover | advanced. Returns RGBA at desktop size."""
    src = Image.open(ASSETS / "splash.png").convert("RGBA")
    ui = src.resize((DW, DH), Image.Resampling.LANCZOS)
    mask = Image.new("L", ui.size, 0)
    radius = max(1, int(round(3 * SCALE)))
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, DW - 1, DH - 1), radius=radius, fill=255)
    ui.putalpha(mask)
    draw = ImageDraw.Draw(ui)

    def at(x, y):
        return (x * SCALE, y * SCALE)

    wm = Image.open(ASSETS / "wordmark.png").convert("RGBA")
    logo_h = int(round(52 * SCALE))
    logo_w = int(round(logo_h * wm.size[0] / wm.size[1]))
    wm = wm.resize((logo_w, logo_h), Image.Resampling.LANCZOS)
    lx = (DW - logo_w) // 2
    ly = int(round(10 * SCALE))
    ui.alpha_composite(wm, (lx, ly))

    stroke = (220, 222, 226, 255)
    sw = max(2, int(round(1.5 * SCALE)))
    # minimize
    mx, my = at(360, 8)
    mw = 26 * SCALE
    y = my + mw / 2
    draw.line((mx + 7 * SCALE, y, mx + mw - 7 * SCALE, y), fill=stroke, width=sw)
    # close
    cx, cy = at(388, 8)
    pad = 7 * SCALE
    draw.line((cx + pad, cy + pad, cx + mw - pad, cy + mw - pad), fill=stroke, width=sw)
    draw.line((cx + mw - pad, cy + pad, cx + pad, cy + mw - pad), fill=stroke, width=sw)

    if mode == "advanced":
        panel = Image.new("RGBA", (DW, DH - int(548 * SCALE)), (10, 12, 18, 235))
        ui.alpha_composite(panel, (0, int(548 * SCALE)))
        draw = ImageDraw.Draw(ui)
        draw.text(
            (DW / 2, 590 * SCALE),
            "Install location",
            font=font(FONT_SB, 15 * SCALE),
            fill=(255, 255, 255, 255),
            anchor="mm",
        )
        fw, fh = 300 * SCALE, 36 * SCALE
        fx, fy = DW / 2 - fw / 2, 634 * SCALE - fh / 2
        draw.rounded_rectangle(
            (fx, fy, fx + fw, fy + fh),
            radius=max(1, int(round(3 * SCALE))),
            fill=(28, 32, 42, 255),
        )
        draw.text(
            (fx + 12 * SCALE, fy + fh / 2),
            "Choose a folder",
            font=font(FONT_RG, 13 * SCALE),
            fill=(186, 190, 198, 255),
            anchor="lm",
        )
        tracked(
            draw,
            "BACK",
            (DW / 2, 684 * SCALE - 8 * SCALE),
            font(FONT_MD, 11 * SCALE),
            (228, 230, 234, 255),
            1.8 * SCALE,
        )
    else:
        bx, by = at(54, 582)
        bw, bh = 312 * SCALE, 68 * SCALE
        fill = (70, 208, 210, 255) if mode == "hover" else (46, 188, 190, 255)
        draw.rounded_rectangle(
            (bx, by, bx + bw, by + bh),
            radius=max(1, int(round(3 * SCALE))),
            fill=fill,
        )
        draw.text(
            (bx + bw / 2, by + bh / 2),
            "Install",
            font=font(FONT_SB, 22 * SCALE),
            fill=(10, 16, 28, 255),
            anchor="mm",
        )
        tracked(
            draw,
            "ADVANCED OPTIONS",
            (DW / 2, 668 * SCALE - 8 * SCALE),
            font(FONT_MD, 11 * SCALE),
            (228, 230, 234, 255),
            1.8 * SCALE,
        )

    draw.text(
        (DW - 14 * SCALE, DH - 12 * SCALE),
        "Gate Install",
        font=font(FONT_RG, 10 * SCALE),
        fill=(148, 154, 166, 255),
        anchor="rb",
    )
    return ui


def desktop():
    img = Image.new("RGB", (W, H), (18, 22, 32))
    px = img.load()
    for y in range(H):
        t = y / (H - 1)
        r = int(22 + 10 * (1 - t))
        g = int(28 + 8 * (1 - t))
        b = int(40 + 6 * (1 - t))
        for x in range(0, W, 4):
            # cheap vertical gradient, filled in bands
            pass
        if y % 2 == 0:
            continue
    # faster gradient
    import numpy as np

    y = np.linspace(0, 1, H)[:, None]
    x = np.linspace(0, 1, W)[None, :]
    base = np.zeros((H, W, 3), np.float32)
    base[:, :, 0] = 28 + 18 * (1 - y) + 8 * x
    base[:, :, 1] = 34 + 14 * (1 - y) + 6 * (1 - x)
    base[:, :, 2] = 48 + 10 * (1 - y)
    # soft pools so a transparent logo has something to float over
    for cx, cy, rad, col in (
        (0.22, 0.30, 0.42, (48, 62, 84)),
        (0.78, 0.72, 0.38, (36, 44, 62)),
        (0.62, 0.18, 0.22, (58, 70, 86)),
    ):
        dx = (x - cx) / rad
        dy = ((y - cy) * (H / W)) / rad
        blob = np.clip(1 - (dx * dx + dy * dy), 0, 1) ** 2
        for i, c in enumerate(col):
            base[:, :, i] += blob * (c - base[:, :, i]) * 0.55
    noise = np.random.default_rng(3).integers(-4, 5, (H, W, 1), dtype=np.int16)
    arr = np.clip(base + noise, 0, 255).astype(np.uint8)
    img = Image.fromarray(arr, "RGB")
    # taskbar
    bar = Image.new("RGB", (W, 46), (12, 14, 20))
    bd = ImageDraw.Draw(bar)
    bd.line((0, 0, W, 0), fill=(40, 46, 58), width=1)
    f = font(FONT_RG, 13)
    bd.text((W - 22, 23), "Tue 3 Nov    16:42", font=f, fill=(198, 202, 210), anchor="rm")
    img.paste(bar, (0, H - 46))
    return img


def logo_sprite():
    wm = Image.open(ASSETS / "wordmark.png").convert("RGBA")
    width = int(480 * SCALE)
    height = int(round(width * wm.size[1] / wm.size[0]))
    return wm.resize((width, height), Image.Resampling.LANCZOS)


def shadow(size):
    w, h = size
    pad = 48
    canvas = Image.new("RGBA", (w + pad * 2, h + pad * 2), (0, 0, 0, 0))
    d = ImageDraw.Draw(canvas)
    d.rounded_rectangle(
        (pad, pad + 10, pad + w, pad + h + 10),
        radius=max(2, int(4 * SCALE)),
        fill=(0, 0, 0, 150),
    )
    return canvas.filter(ImageFilter.GaussianBlur(22))


def cursor():
    im = Image.new("RGBA", (28, 36), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    pts = [(1, 1), (1, 28), (7, 21), (12, 33), (16, 31), (11, 19), (20, 19)]
    d.polygon(pts, fill=(255, 255, 255, 255), outline=(20, 22, 28, 255))
    return im


def with_alpha(im, opacity):
    if opacity >= 0.999:
        return im
    out = im.copy()
    a = out.getchannel("A").point(lambda p: int(p * opacity))
    out.putalpha(a)
    return out


def main():
    FRAMES.mkdir(parents=True, exist_ok=True)
    bg = desktop()
    windows = {
        "normal": render_window("normal"),
        "hover": render_window("hover"),
        "advanced": render_window("advanced"),
    }
    shade = shadow((DW, DH))
    logo = logo_sprite()
    arrow = cursor()
    logo_xy = ((W - logo.size[0]) // 2, (H - 46 - logo.size[1]) // 2)

    # timeline in seconds
    # 0.00-0.35 fade logo in
    # 0.35-6.35 logo holds (6s)
    # 6.35-6.80 crossfade to window
    # 6.80-7.50 hold
    # 7.50-8.25 cursor to Install
    # 8.25-9.05 hover
    # 9.05-9.70 cursor to Advanced Options
    # 9.70 click, panel
    # 9.70-11.10 hold panel
    duration = 11.1
    n = int(round(duration * FPS))

    btn = (OX + 268 * SCALE, OY + (582 + 40) * SCALE)
    adv = (OX + 248 * SCALE, OY + 690 * SCALE)
    start = (W * 0.72, H * 0.78)

    ffmpeg = imageio_ffmpeg.get_ffmpeg_exe()
    cmd = [
        ffmpeg, "-y",
        "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{W}x{H}", "-r", str(FPS),
        "-i", "-",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-crf", "17", "-preset", "medium",
        "-movflags", "+faststart",
        str(OUT),
    ]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)

    shots = {int(1.0 * FPS), int(6.0 * FPS), int(7.2 * FPS), int(8.6 * FPS), int(10.4 * FPS)}

    for i in range(n):
        t = i / FPS
        frame = bg.copy()
        show_cursor = False
        cursor_xy = start

        if t < 6.35:
            op = smooth(t / 0.35) if t < 0.35 else 1.0
            frame.paste(with_alpha(logo, op), logo_xy, with_alpha(logo, op))
        elif t < 6.80:
            u = smooth((t - 6.35) / 0.45)
            frame.paste(with_alpha(logo, 1 - u), logo_xy, with_alpha(logo, 1 - u))
            paste_window(frame, windows["normal"], shade, u)
        else:
            mode = "normal"
            if t >= 9.70:
                mode = "advanced"
                show_cursor = True
                cursor_xy = adv
            elif t >= 9.05:
                u = smooth((t - 9.05) / 0.65)
                show_cursor = True
                cursor_xy = lerp(btn, adv, u)
                mode = "hover" if u < 0.15 else "normal"
            elif t >= 8.25:
                show_cursor = True
                cursor_xy = btn
                mode = "hover"
            elif t >= 7.50:
                u = smooth((t - 7.50) / 0.75)
                show_cursor = True
                cursor_xy = lerp(start, btn, u)
            paste_window(frame, windows[mode], shade, 1.0)

        if show_cursor:
            x, y = int(cursor_xy[0]), int(cursor_xy[1])
            frame.paste(arrow, (x, y), arrow)

        proc.stdin.write(frame.tobytes())
        if i in shots:
            frame.save(FRAMES / f"t{t:.1f}.jpg", quality=90)

    proc.stdin.close()
    err = proc.stderr.read().decode()
    code = proc.wait()
    if code != 0:
        raise SystemExit(err[-2000:])
    print("wrote", OUT, "frames", n)


def lerp(a, b, u):
    return (a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u)


def paste_window(frame, window, shade, opacity):
    # slight rise on appear
    rise = int((1 - opacity) * 10)
    sx = OX - 48
    sy = OY - 48 + rise
    sh = with_alpha(shade, 0.85 * opacity)
    frame.paste(sh, (sx, sy), sh)
    win = with_alpha(window, opacity)
    frame.paste(win, (OX, OY + rise), win)


if __name__ == "__main__":
    main()
