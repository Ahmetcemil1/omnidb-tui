#!/usr/bin/env python3
import json
import os
import sys
import subprocess
import pyte
from PIL import Image, ImageDraw, ImageFont

def get_font():
    # Try system monospace fonts
    font_paths = [
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/LiberationMono-Regular.ttf",
        "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]
    for p in font_paths:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, 16)
            except Exception:
                pass
    return ImageFont.load_default()

def color_to_rgb(color_name, is_bg=False):
    default_fg = (204, 204, 204)
    default_bg = (24, 24, 30)
    
    if color_name == "default":
        return default_bg if is_bg else default_fg
        
    color_map = {
        "black": (20, 20, 20),
        "red": (235, 87, 87),
        "green": (39, 201, 63),
        "yellow": (241, 196, 15),
        "blue": (52, 152, 219),
        "magenta": (155, 89, 182),
        "cyan": (26, 188, 156),
        "white": (236, 240, 241),
        "brightblack": (100, 100, 100),
        "brightred": (255, 100, 100),
        "brightgreen": (100, 255, 100),
        "brightyellow": (255, 255, 100),
        "brightblue": (100, 100, 255),
        "brightmagenta": (255, 100, 255),
        "brightcyan": (100, 255, 255),
        "brightwhite": (255, 255, 255),
    }
    
    if isinstance(color_name, str) and color_name in color_map:
        return color_map[color_name]
        
    # Hex or tuple color
    if isinstance(color_name, str) and len(color_name) == 6:
        try:
            return (int(color_name[0:2], 16), int(color_name[2:4], 16), int(color_name[4:6], 16))
        except ValueError:
            pass
            
    return default_bg if is_bg else default_fg

def render_screen_to_image(screen, font, char_width=11, char_height=24, cols=120, rows=35):
    img_w = cols * char_width + 40
    img_h = rows * char_height + 50
    
    img = Image.new("RGB", (img_w, img_h), (24, 24, 30))
    draw = ImageDraw.Draw(img)
    
    # Draw top window bar (Mac/Linux TUI style)
    draw.rectangle([0, 0, img_w, 30], fill=(35, 35, 45))
    draw.ellipse([15, 10, 25, 20], fill=(255, 95, 86)) # Close
    draw.ellipse([32, 10, 42, 20], fill=(255, 189, 46)) # Minimize
    draw.ellipse([49, 10, 59, 20], fill=(39, 201, 63)) # Maximize
    draw.text((70, 7), "OmniDB TUI Workspace — Alacritty Terminal", fill=(180, 180, 190), font=font)

    margin_x = 20
    margin_y = 40

    for y in range(rows):
        row_cells = screen.buffer[y]
        for x in range(cols):
            cell = row_cells[x]
            char = cell.data
            fg = color_to_rgb(cell.fg)
            bg = color_to_rgb(cell.bg, is_bg=True)
            
            px = margin_x + x * char_width
            py = margin_y + y * char_height
            
            if bg != (24, 24, 30):
                draw.rectangle([px, py, px + char_width, py + char_height], fill=bg)
                
            if char and char != " ":
                draw.text((px, py), char, fill=fg, font=font)
                
    return img

def main():
    cast_path = "/home/zenhor/Masaüstü/proje2/demo.cast"
    output_mp4 = "/home/zenhor/Masaüstü/proje2/omnidb_tui_launch_demo.mp4"
    
    print(f"🎬 Processing {cast_path}...")
    
    with open(cast_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
        
    # Force the display resolution to match the 120x35 PTY window size
    cols = 120
    rows = 35
    
    screen = pyte.Screen(cols, rows)
    screen.resize(rows, cols)
    stream = pyte.Stream(screen)
    
    font = get_font()
    
    # Target framerate of 15 fps.
    fps = 15
    frame_duration = 1.0 / fps
    
    ffmpeg_cmd = [
        "ffmpeg", "-y",
        "-f", "image2pipe",
        "-vcodec", "png",
        "-r", str(fps),
        "-i", "-",
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-crf", "18",
        output_mp4
    ]
    
    proc = subprocess.Popen(ffmpeg_cmd, stdin=subprocess.PIPE)
    
    # Parse events chronologically with accumulated absolute timestamps
    events = []
    current_abs_time = 0.0
    for line in lines[1:]:
        if not line.strip():
            continue
        try:
            event = json.loads(line)
            if len(event) >= 3 and event[1] == "o":
                delta = float(event[0])
                current_abs_time += delta
                events.append((current_abs_time, event[2]))
        except Exception:
            pass
            
    # Sort events by timestamp
    events.sort(key=lambda x: x[0])
    
    if not events:
        print("No output events found!")
        return

    # Re-normalize timestamps
    reordered_events = []
    t0 = events[0][0]
    for ts, data in events:
        reordered_events.append((ts - t0, data))
            
    # Simulate the playback at 15 fps
    total_duration = reordered_events[-1][0]
    total_frames = int(total_duration * fps) + 30 # extra 2 seconds at the end
    
    print(f"📹 Rendering {total_frames} frames ({total_duration:.2f}s) to high-definition MP4...")
    
    event_idx = 0
    for frame_num in range(total_frames):
        target_time = frame_num * frame_duration
        
        # Feed all events that happened up to this target time
        while event_idx < len(reordered_events) and reordered_events[event_idx][0] <= target_time:
            stream.feed(reordered_events[event_idx][1])
            event_idx += 1
            
        img = render_screen_to_image(screen, font, cols=cols, rows=rows)
        img.save(proc.stdin, format="PNG")
        
        if frame_num % 50 == 0:
            print(f"   Rendered {frame_num}/{total_frames} frames...")
            
    proc.stdin.close()
    proc.wait()
    
    if os.path.exists(output_mp4) and os.path.getsize(output_mp4) > 1000:
        print(f"\n🎉 SUCCESS! Ultra-high quality MP4 video generated successfully!")
        print(f"   Output MP4: {output_mp4}")
        print(f"   Size: {os.path.getsize(output_mp4) / 1024 / 1024:.2f} MB")
    else:
        print("\n❌ Failed to generate MP4.")

if __name__ == "__main__":
    main()
