#!/usr/bin/env python3
"""Generate 80sTerminal app icon - a beige CRT monitor with green terminal text."""

from PIL import Image, ImageDraw, ImageFont
import os
import subprocess
import sys

def draw_icon(size):
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    s = size  # shorthand

    # Padding from edges
    pad = int(s * 0.08)

    # Monitor outer body (beige, rounded rect)
    monitor_rect = [pad, pad, s - pad, s - pad * 2]
    beige = (199, 186, 168)
    beige_dark = (170, 158, 140)
    beige_light = (220, 210, 195)

    # Draw monitor body with rounded corners
    corner_r = int(s * 0.08)
    d.rounded_rectangle(monitor_rect, radius=corner_r, fill=beige, outline=beige_dark, width=max(1, s//128))

    # Top highlight strip (skip if too small)
    highlight_h = int(s * 0.04)
    if highlight_h >= 2:
        highlight_rect = [pad + corner_r, pad + 1, s - pad - corner_r, pad + highlight_h]
        d.rectangle(highlight_rect, fill=beige_light)

    # Screen area (dark, inset)
    screen_margin = int(s * 0.14)
    screen_top = pad + int(s * 0.10)
    screen_bottom = s - pad * 2 - int(s * 0.12)
    screen_rect = [screen_margin, screen_top, s - screen_margin, screen_bottom]
    screen_r = int(s * 0.03)

    # Screen bezel (darker ring around screen)
    bezel_expand = int(s * 0.02)
    bezel_rect = [screen_rect[0] - bezel_expand, screen_rect[1] - bezel_expand,
                  screen_rect[2] + bezel_expand, screen_rect[3] + bezel_expand]
    d.rounded_rectangle(bezel_rect, radius=screen_r + 2, fill=beige_dark)

    # Screen background (dark)
    screen_bg = (8, 10, 8)
    d.rounded_rectangle(screen_rect, radius=screen_r, fill=screen_bg)

    # Draw green terminal text on screen
    green = (51, 255, 51)
    green_dim = (25, 160, 25)

    text_left = screen_rect[0] + int(s * 0.04)
    text_area_top = screen_rect[1] + int(s * 0.03)
    text_area_bottom = screen_rect[3] - int(s * 0.03)
    line_height = int(s * 0.035)

    # Simulated terminal lines
    lines = [
        (green, 0.65),    # $ ls -la
        (green_dim, 0.80),  # total 42
        (green_dim, 0.55),  # drwxr-xr-x
        (green_dim, 0.70),  # -rw-r--r--
        (green_dim, 0.45),  #
        (green, 0.35),    # $ _
    ]

    y = text_area_top
    for color, width_pct in lines:
        if y + line_height > text_area_bottom:
            break
        line_w = int((screen_rect[2] - text_left - int(s * 0.04)) * width_pct)
        bar_h = max(2, int(s * 0.018))
        d.rounded_rectangle(
            [text_left, y, text_left + line_w, y + bar_h],
            radius=max(1, bar_h // 2),
            fill=color
        )
        y += line_height

    # Cursor block on last line
    cursor_w = max(3, int(s * 0.025))
    cursor_h = max(3, int(s * 0.025))
    last_line_x = text_left + int((screen_rect[2] - text_left) * 0.08)
    cursor_y = y - line_height + (line_height - cursor_h) // 2
    if cursor_y > text_area_top:
        d.rectangle([last_line_x, cursor_y - int(s*0.005), last_line_x + cursor_w, cursor_y + cursor_h], fill=green)

    # Monitor stand/base
    stand_w = int(s * 0.25)
    stand_h = int(s * 0.04)
    stand_x = (s - stand_w) // 2
    stand_y = monitor_rect[3]
    d.rounded_rectangle(
        [stand_x, stand_y, stand_x + stand_w, stand_y + stand_h],
        radius=max(1, stand_h // 3),
        fill=beige_dark
    )

    # Base
    base_w = int(s * 0.40)
    base_h = int(s * 0.03)
    base_x = (s - base_w) // 2
    base_y = stand_y + stand_h - 1
    d.rounded_rectangle(
        [base_x, base_y, base_x + base_w, base_y + base_h],
        radius=max(1, base_h // 2),
        fill=beige
    )

    # "80s" label on bezel below screen (only for larger sizes)
    if s >= 128:
        try:
            font_size = max(8, int(s * 0.04))
            font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", font_size)
            label = "80s"
            bbox = d.textbbox((0, 0), label, font=font)
            tw = bbox[2] - bbox[0]
            label_x = (s - tw) // 2
            label_y = screen_rect[3] + int(s * 0.04)
            d.text((label_x, label_y), label, fill=beige_dark, font=font)
        except:
            pass

    return img


def main():
    project_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    iconset_dir = os.path.join(project_dir, "assets", "80sTerminal.iconset")
    os.makedirs(iconset_dir, exist_ok=True)

    # Required sizes for macOS .icns
    sizes = [16, 32, 64, 128, 256, 512, 1024]

    for sz in sizes:
        img = draw_icon(sz)
        # Standard resolution
        if sz <= 512:
            img.save(os.path.join(iconset_dir, f"icon_{sz}x{sz}.png"))
        # @2x (Retina) versions
        if sz >= 32:
            half = sz // 2
            if half in [16, 32, 64, 128, 256, 512]:
                img.save(os.path.join(iconset_dir, f"icon_{half}x{half}@2x.png"))

    # Generate .icns using iconutil
    icns_path = os.path.join(project_dir, "assets", "80sTerminal.icns")
    result = subprocess.run(
        ["iconutil", "-c", "icns", iconset_dir, "-o", icns_path],
        capture_output=True, text=True
    )

    if result.returncode != 0:
        print(f"iconutil error: {result.stderr}")
        sys.exit(1)

    print(f"Created icon: {icns_path}")

    # Clean up iconset
    import shutil
    shutil.rmtree(iconset_dir)


if __name__ == "__main__":
    main()
