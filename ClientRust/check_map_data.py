import struct
import os

# 读取地图文件
map_file = r"D:\Users\gxh\Documents\GitHub\Crystal\ClientRust\Map\0.map"

if not os.path.exists(map_file):
    print(f"Map file not found: {map_file}")
    exit(1)

with open(map_file, 'rb') as f:
    data = f.read()

print(f"Map file size: {len(data)} bytes")
print(f"First 20 bytes (hex): {data[:20].hex()}")
print()

# Parse header
offset = 0
width = struct.unpack('<H', data[offset:offset+2])[0]
offset += 2
height = struct.unpack('<H', data[offset:offset+2])[0]
offset += 2

print(f"Map size: {width}x{height}")
print()

# Skip to cells (offset 52)
offset = 52

# XOR key for Type1 format
xor_key = 0xAA38

print("Sampling cells near player position (100, 100):")
print("=" * 80)

for y in range(98, 103):
    for x in range(98, 103):
        if x >= width or y >= height:
            continue
        
        # Calculate offset for cell (x, y)
        # C# loops: for x { for y { ... } }
        # So data is arranged as: [x0,y0], [x0,y1], ..., [x0,y(h-1)], [x1,y0], ...
        cell_offset = 52 + (x * height + y) * 14
        
        if cell_offset + 14 > len(data):
            print(f"Cell ({x:3}, {y:3}): OUT OF BOUNDS")
            continue
        
        # Read cell data
        back_image_raw = struct.unpack('<i', data[cell_offset:cell_offset+4])[0]
        back_image = back_image_raw ^ 0xAA38AA38
        
        middle_image_raw = struct.unpack('<h', data[cell_offset+4:cell_offset+6])[0]
        middle_image = middle_image_raw ^ xor_key
        
        front_image_raw = struct.unpack('<h', data[cell_offset+6:cell_offset+8])[0]
        front_image = front_image_raw ^ xor_key
        
        door_index = data[cell_offset+8] & 0x7F
        door_offset = data[cell_offset+9]
        
        marker = " <-- PLAYER" if (x == 100 and y == 100) else ""
        
        print(f"Cell ({x:3}, {y:3}): Back={back_image:5}, Mid={middle_image:5}, Front={front_image:5}, Door={door_index:3}{marker}")
    print()
