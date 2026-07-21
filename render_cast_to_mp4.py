#!/usr/bin/env python3
"""
Cast-to-MP4 Renderer for OmniDB TUI
Converts asciinema .cast files to proper HD MP4 videos using
pyte terminal emulation + Pillow frame rendering + ffmpeg encoding.
"""
import json, os, sys, subprocess
import pyte
from PIL import Image, ImageDraw, ImageFont

# ── Color palette (GitHub Dark) ───────────────────────────────────────────────
BG          = (13,  17,  23)   # canvas background
WIN_BAR     = (22,  27,  34)   # title-bar
WIN_FG      = (139, 148, 158)  # title-bar text
BTN_RED     = (255, 95,  86)
BTN_YEL     = (255, 189, 46)
BTN_GRN     = (39,  201, 63)

COLORS = {
    "default":       (230, 237, 243),
    "black":         (22,  27,  34),
    "red":           (255, 123, 114),
    "green":         (63,  185, 80),
    "yellow":        (210, 153, 34),
    "blue":          (88,  166, 255),
    "magenta":       (188, 140, 255),
    "cyan":          (57,  197, 207),
    "white":         (177, 186, 196),
    "brightblack":   (110, 118, 129),
    "brightred":     (255, 161, 152),
    "brightgreen":   (86,  211, 100),
    "brightyellow":  (227, 179, 65),
    "brightblue":    (121, 192, 255),
    "brightmagenta": (210, 168, 255),
    "brightcyan":    (86,  212, 221),
    "brightwhite":   (240, 246, 252),
}

def hex_to_rgb(h):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))

def resolve(color, is_bg=False):
    if color == "default":
        return BG if is_bg else COLORS["default"]
    if isinstance(color, str) and color in COLORS:
        return COLORS[color]
    if isinstance(color, str) and len(color) == 6:
        try:
            return hex_to_rgb(color)
        except Exception:
            pass
    return BG if is_bg else COLORS["default"]


def get_font(size=15):
    candidates = [
        "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/LiberationMono-Regular.ttf",
        "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]
    for p in candidates:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()


def render_frame(screen, font, cw, ch, cols, rows):
    title = "  OmniDB TUI  v0.1.0  —  Multi-Database Terminal Workspace"
    bar_h = 36
    pad_x = 12
    pad_y = 10
    img_w = cols * cw + pad_x * 2
    img_h = rows * ch + pad_y * 2 + bar_h

    img = Image.new("RGB", (img_w, img_h), BG)
    draw = ImageDraw.Draw(img)

    # Window title bar
    draw.rectangle([0, 0, img_w, bar_h], fill=WIN_BAR)
    # Traffic-light buttons
    for i, col in enumerate([BTN_RED, BTN_YEL, BTN_GRN]):
        cx = 16 + i * 22
        draw.ellipse([cx-6, bar_h//2-6, cx+6, bar_h//2+6], fill=col)
    # Title text
    draw.text((72, bar_h//2 - 7), title, fill=WIN_FG, font=font)
    # Separator line
    draw.rectangle([0, bar_h, img_w, bar_h + 1], fill=(48, 54, 61))

    # Terminal cells
    for row in range(rows):
        cells = screen.buffer[row]
        for col in range(cols):
            cell = cells[col]
            fg = resolve(cell.fg)
            bg = resolve(cell.bg, is_bg=True)
            px = pad_x + col * cw
            py = bar_h + pad_y + row * ch

            if bg != BG:
                draw.rectangle([px, py, px + cw, py + ch], fill=bg)

            ch_str = cell.data
            if cell.bold:
                fg = tuple(min(255, c + 40) for c in fg)
            if ch_str and ch_str != " ":
                draw.text((px, py), ch_str, fill=fg, font=font)

    return img


def cast_to_mp4(cast_path, mp4_path, fps=12, font_size=15, char_w=9, char_h=20):
    print(f"🎬  Input : {cast_path}")
    print(f"📹  Output: {mp4_path}")

    with open(cast_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    header = json.loads(lines[0])
    cols   = header.get("width",  header.get("term", {}).get("cols", 140))
    rows   = header.get("height", header.get("term", {}).get("rows", 38))

    screen = pyte.Screen(cols, rows)
    stream = pyte.Stream(screen)
    font   = get_font(font_size)

    bar_h  = 36
    pad_x, pad_y = 12, 10
    img_w  = cols * char_w + pad_x * 2
    img_h  = rows * char_h + pad_y * 2 + bar_h

    ffmpeg_cmd = [
        "ffmpeg", "-y",
        "-f", "image2pipe",
        "-vcodec", "png",
        "-r", str(fps),
        "-i", "-",
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-crf", "16",
        "-preset", "slow",
        mp4_path
    ]
    proc = subprocess.Popen(ffmpeg_cmd, stdin=subprocess.PIPE,
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    events = []
    for line in lines[1:]:
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
            if len(ev) >= 3 and ev[1] == "o":
                events.append((float(ev[0]), ev[2]))
        except Exception:
            pass

    if not events:
        print("⚠️  No events found in cast file.")
        proc.stdin.close()
        proc.wait()
        return

    frame_count = 0
    prev_t = 0.0
    spf = 1.0 / fps  # seconds per frame

    for ts, data in events:
        stream.feed(data)
        elapsed = ts - prev_t
        # How many frames to hold for this interval?
        n_frames = max(1, round(elapsed / spf))
        img = render_frame(screen, font, char_w, char_h, cols, rows)
        for _ in range(n_frames):
            img.save(proc.stdin, format="PNG")
            frame_count += 1

        prev_t = ts
        if frame_count % 50 == 0:
            print(f"   ↳ {frame_count} frames rendered …", flush=True)

    # Hold last frame for 2 extra seconds
    img = render_frame(screen, font, char_w, char_h, cols, rows)
    for _ in range(fps * 2):
        img.save(proc.stdin, format="PNG")
        frame_count += 1

    proc.stdin.close()
    proc.wait()

    size = os.path.getsize(mp4_path) if os.path.exists(mp4_path) else 0
    if size > 1000:
        print(f"\n✅  Done!  {frame_count} frames → {mp4_path}  ({size/1024/1024:.2f} MB)")
    else:
        print(f"\n❌  Failed — output is only {size} bytes")


if __name__ == "__main__":
    cast_path = sys.argv[1] if len(sys.argv) > 1 else "demo.cast"
    mp4_path  = sys.argv[2] if len(sys.argv) > 2 else cast_path.replace(".cast", ".mp4")
    cast_to_mp4(cast_path, mp4_path)
