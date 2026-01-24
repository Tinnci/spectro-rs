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

def refine_icon_v14(input_path, output_path):
    print("Initializing Zoom-In Mode (115% scale)...")
    RENDER_SIZE = 2048 
    TARGET_SIZE = 1024
    
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
    
    # --- Step 2: Assemble at High-Res (Zoomed In) ---
    render_canvas = Image.new("RGBA", (RENDER_SIZE, RENDER_SIZE), (0, 0, 0, 0))
    
    # INCREASING SCALE to 115% of RENDER_SIZE to "cut off" outer parts
    # This creates a "Zoomed In" look where the prism fills the whole squircle
    zoom_factor = 1.15
    fill_size = int(RENDER_SIZE * zoom_factor)
    
    aspect = content.width / content.height
    new_w, new_h = (fill_size, int(fill_size/aspect)) if aspect > 1 else (int(fill_size*aspect), fill_size)
    
    content_hi = content.resize((new_w, new_h), Image.Resampling.LANCZOS)
    render_canvas.paste(content_hi, ((RENDER_SIZE-new_w)//2, (RENDER_SIZE-new_h)//2), content_hi)
    
    # --- Step 3: Apply Texture and Squircle at High-Res ---
    render_canvas = add_fine_grain(render_canvas, intensity=5)
    
    squircle = draw_squircle_mask((RENDER_SIZE, RENDER_SIZE), n=4.8)
    # The mask will now clip the enlarged content
    final_alpha = ImageChops.multiply(render_canvas.getchannel('A'), squircle)
    render_canvas.putalpha(final_alpha)
    
    # --- Step 4: Final SSAA Downsampling ---
    print(f"Downsampling to {TARGET_SIZE} (SSAA)...")
    final_img = render_canvas.resize((TARGET_SIZE, TARGET_SIZE), Image.Resampling.LANCZOS)
    
    final_a = final_img.getchannel('A').filter(ImageFilter.GaussianBlur(radius=0.5))
    final_img.putalpha(final_a)
    
    final_img.save(output_path, "PNG")
    print(f"Zoomed-In Icon V14 saved to {output_path}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 polish_icon.py input.png output.png")
    else:
        refine_icon_v14(sys.argv[1], sys.argv[2])
