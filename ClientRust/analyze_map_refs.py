#!/usr/bin/env python3
"""分析0.map引用了哪些MapLibs索引"""

import struct

map_file = r"d:\Users\gxh\Documents\GitHub\Crystal\ClientRust\Map\0.map"

with open(map_file, 'rb') as f:
    data = f.read()

# 读取宽高
width, height = struct.unpack('<HH', data[4:8])

print(f"地图尺寸: {width}x{height}\n")

# 统计Middle层的LibIndex分布
middle_index_stats = {}

offset = 8
for x in range(width):
    for y in range(height):
        # 跳过BackIndex(2) + BackImage(4)
        offset += 6
        
        # MiddleIndex (2字节)
        middle_index = struct.unpack('<h', data[offset:offset+2])[0]
        offset += 2
        
        # MiddleImage (2字节)
        middle_image = struct.unpack('<h', data[offset:offset+2])[0]
        offset += 2
        
        # 跳过剩余14字节
        offset += 14
        
        # 统计非零的MiddleIndex
        if middle_image != 0 and middle_index >= 0:
            if middle_index not in middle_index_stats:
                middle_index_stats[middle_index] = 0
            middle_index_stats[middle_index] += 1

print(f"Middle层引用的MapLibs索引统计：\n")
print(f"{'索引':<10} {'引用次数':<10} {'对应文件路径'}")
print("="*80)

# MapLibs映射规则
def get_lib_path(index):
    if index == 0:
        return "Map/WemadeMir2/Tiles.Lib"
    elif index == 1:
        return "Map/WemadeMir2/Smtiles.Lib"
    elif index == 2:
        return "Map/WemadeMir2/Objects.Lib"
    elif 3 <= index <= 29:
        return f"Map/WemadeMir2/Objects{index-1}.Lib"
    elif index == 90:
        return "Map/WemadeMir2/Objects_32bit.Lib"
    elif index == 100:
        return "Map/ShandaMir2/Tiles.Lib"
    elif 101 <= index <= 109:
        return f"Map/ShandaMir2/Tiles{index-99}.Lib"
    elif index == 110:
        return "Map/ShandaMir2/SmTiles.Lib"
    elif 111 <= index <= 119:
        return f"Map/ShandaMir2/SmTiles{index-109}.Lib"
    elif index == 120:
        return "Map/ShandaMir2/Objects.Lib"
    elif 121 <= index <= 150:
        return f"Map/ShandaMir2/Objects{index-119}.Lib"
    elif index == 190:
        return "Map/ShandaMir2/AniTiles1.Lib"
    elif 200 <= index < 400:
        # Mir3 libraries
        return f"Map/WemadeMir3/... (索引{index})"
    else:
        return f"未知索引 {index}"

for idx in sorted(middle_index_stats.keys())[:20]:  # 只显示前20个
    count = middle_index_stats[idx]
    path = get_lib_path(idx)
    print(f"{idx:<10} {count:<10} {path}")

if len(middle_index_stats) > 20:
    print(f"... 还有 {len(middle_index_stats) - 20} 个其他索引")

print(f"\n总共引用了 {len(middle_index_stats)} 个不同的MapLibs索引")
