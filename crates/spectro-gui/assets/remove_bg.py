from PIL import Image
import sys

def remove_background(input_path, output_path):
    img = Image.open(input_path).convert("RGBA")
    datas = img.getdata()

    new_data = []
    # Threshold for what we consider "background white"
    # We want to be careful not to remove white INSIDE the icon (like the laser beam)
    # But usually, the "background" is pure white (255, 255, 255)
    
    # Let's use a flood fill from corners if possible, or just color thresholding
    # for simplicity, let's try thresholding all pixels that are VERY close to white
    # and see if that works. If it removes the laser beam, we'll need a better approach.
    
    threshold = 245
    for item in datas:
        # If pixel is very bright white
        if item[0] >= threshold and item[1] >= threshold and item[2] >= threshold:
            new_data.append((255, 255, 255, 0))
        else:
            new_data.append(item)

    img.putdata(new_data)
    img.save(output_path, "PNG")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 remove_bg.py input.png output.png")
    else:
        remove_background(sys.argv[1], sys.argv[2])
