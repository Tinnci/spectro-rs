from PIL import Image, ImageFilter, ImageOps, ImageDraw, ImageChops
import sys
import math
import random

def draw_squircle_mask(size, n=4.8):
    """Generates a high-precision macOS-style 'Squircle' mask."""
    w, h = size
    # Internal high-res drawing for SSAA
    scale = 2 
    sw, sh = w * scale, h * scale
    mask = Image.new("L", (sw, sh), 0)
    draw = ImageDraw.Draw(mask)
    points = []
    a, b = sw / 2.0, sh / 2.0
    steps = 1440
    for i in range(steps):
        theta = (2 * math.pi * i) / steps
        cos_t, sin_t = math.cos(theta), math.sin(theta)
        x = a * (abs(cos_t) ** (2/n)) * (1 if cos_t >= 0 else -1)
        y = b * (abs(sin_t) ** (2/n)) * (1 if sin_t >= 0 else -1)
        points.append((x + sw/2.0, y + sh/2.0))
    draw.polygon(points, fill=255)
    return mask.resize(size, Image.Resampling.LANCZOS)

def add_fine_grain(image, intensity=4):
    """Adds fine noise to prevent banding and increase perceived detail."""
    width, height = image.size
    noise = Image.new('RGB', (width, height))
    pixels = noise.load()
    for y in range(height):
        for x in range(width):
            n = random.randint(-intensity, intensity)
            pixels[x, y] = (n, n, n)
    image_rgb = image.convert("RGB")
    final_rgb = ImageChops.add(image_rgb, noise)
    image.paste(final_rgb, (0,0), image.getchannel('A'))
    return image

def refine_icon_hig(input_path, output_path):
    """
    Refines the icon following macOS Human Interface Guidelines (HIG).
    - Canvas: 1024x1024
    - Main Subject Zone: ~80% (approx 824px)
    - SSAA Pipeline: 2048px -> 1024px using Lanczos
    """
    print("Initializing HIG-Compliant SSAA Pipeline (80% Subject Rule)...")
    RENDER_SIZE = 2048 
    TARGET_SIZE = 1024
    # HIG Recommendation: Subject takes about 80-82% of the canvas
    # Standard macOS icon grid safe-zone is approx 82.4%
    HIG_SCALE = 0.824 
    
    img_orig = Image.open(input_path).convert("RGBA")
    
    # --- Step 1: High-Res Subject Extraction ---
    check_img = img_orig.copy().convert("RGB")
    marker_color = (255, 0, 255)
    for corner in [(0, 0), (img_orig.width-1, 0), (0, img_orig.height-1), (img_orig.width-1, img_orig.height-1)]:
        ImageDraw.floodfill(check_img, corner, marker_color, thresh=70)
    
    s_mask = Image.new("L", img_orig.size, 0)
    cp = check_img.load()
    mp = s_mask.load()
    for y in range(img_orig.height):
        for x in range(img_orig.width):
            if cp[x, y] != marker_color: mp[x, y] = 255
            
    s_mask = s_mask.filter(ImageFilter.MinFilter(5)) 
    s_mask = s_mask.filter(ImageFilter.GaussianBlur(radius=1.5))
    
    bbox = s_mask.getbbox()
    if not bbox: return
    content = img_orig.crop(bbox)
    content.putalpha(s_mask.crop(bbox))
    
    # --- Step 2: Assemble at High-Res (2048px) ---
    # Create the full render canvas (will have transparent margins in the end)
    render_canvas = Image.new("RGBA", (RENDER_SIZE, RENDER_SIZE), (0, 0, 0, 0))
    
    # Calculate the size of the "Card" (the squircle container)
    card_render_size = int(RENDER_SIZE * HIG_SCALE)
    
    # Apply the 115% zoom relative to the CARD size (User likes this crop)
    zoom_factor = 1.15
    subject_render_size = int(card_render_size * zoom_factor)
    
    aspect = content.width / content.height
    new_w, new_h = (subject_render_size, int(subject_render_size/aspect)) if aspect > 1 else (int(subject_render_size*aspect), subject_render_size)
    
    # Scale content
    content_hi = content.resize((new_w, new_h), Image.Resampling.LANCZOS)
    
    # Create an intermediate layer for the card to apply the mask
    card_layer = Image.new("RGBA", (RENDER_SIZE, RENDER_SIZE), (0, 0, 0, 0))
    # Paste subject centered
    paste_pos = ((RENDER_SIZE - new_w) // 2, (RENDER_SIZE - new_h) // 2)
    card_layer.paste(content_hi, paste_pos, content_hi)
    
    # --- Step 3: Apply Texture and HIG Squircle ---
    card_layer = add_fine_grain(card_layer, intensity=5)
    
    # Generate Squircle at CARD size
    squircle_base = draw_squircle_mask((card_render_size, card_render_size), n=4.8)
    
    # Create full-size mask for the card layer
    full_mask = Image.new("L", (RENDER_SIZE, RENDER_SIZE), 0)
    mask_offset = (RENDER_SIZE - card_render_size) // 2
    full_mask.paste(squircle_base, (mask_offset, mask_offset))
    
    # Mask the card layer
    final_alpha = ImageChops.multiply(card_layer.getchannel('A'), full_mask)
    card_layer.putalpha(final_alpha)
    
    # --- Step 4: Final SSAA Downsampling (Lanczos) ---
    print(f"Downsampling to {TARGET_SIZE} via Lanczos (Final SSAA)...")
    final_img = card_layer.resize((TARGET_SIZE, TARGET_SIZE), Image.Resampling.LANCZOS)
    
    # Subtle polish on alpha for perfect transparency transitions
    final_a = final_img.getchannel('A').filter(ImageFilter.GaussianBlur(radius=0.3))
    final_img.putalpha(final_a)
    
    final_img.save(output_path, "PNG")
    print(f"HIG-Compliant Icon saved to {output_path}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 polish_icon.py input.png output.png")
    else:
        refine_icon_hig(sys.argv[1], sys.argv[2])
