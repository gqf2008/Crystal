#!/usr/bin/env python3
"""检查Middle层图像的实际尺寸"""

import struct
import os
from pathlib import Path

def read_lib_image_size(lib_file, image_index):
    """读取.Lib文件中指定图像的尺寸"""
    try:
        with open(lib_file, 'rb') as f:
            # 读取文件头
            header = f.read(8)
            if len(header) < 8:
                return None
            
            count = struct.unpack('<I', header[:4])[0]
            
            if image_index >= count:
                return None
            
            # 定位到图像索引
            f.seek(8 + image_index * 12)  # 每个索引12字节
            index_data = f.read(12)
            
            offset, length, width, height = struct.unpack('<IIHH', index_data)
            
            return (width, height)
    except:
        return None

def check_0map_middle_layer():
    """检查0.map的Middle层图像尺寸"""
    
    map_file = r"d:\Users\gxh\Documents\GitHub\Crystal\ClientRust\Map\0.map"
    data_dir = r"d:\Users\gxh\Documents\GitHub\Crystal\ClientRust\Data"
    
    if not os.path.exists(map_file):
        print(f"❌ 找不到地图文件: {map_file}")
        return
    
    # 读取地图
    with open(map_file, 'rb') as f:
        data = f.read()
    
    # 读取宽高
    width, height = struct.unpack('<HH', data[4:8])
    
    print(f"\n{'='*80}")
    print(f"🗺️  分析 0.map 的 Middle 层")
    print(f"{'='*80}")
    print(f"地图尺寸: {width}x{height}")
    
    # 收集Middle层的所有LibIndex和ImageIndex
    middle_refs = {}  # {lib_index: {image_index: count}}
    
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
            
            # 统计Middle层引用
            if middle_image != 0 and middle_index >= 0:
                if middle_index not in middle_refs:
                    middle_refs[middle_index] = {}
                
                img_idx = middle_image - 1
                if img_idx not in middle_refs[middle_index]:
                    middle_refs[middle_index][img_idx] = 0
                middle_refs[middle_index][img_idx] += 1
    
    print(f"\n📊 Middle层引用统计:")
    print(f"引用了 {len(middle_refs)} 个图库")
    
    # 检查每个引用的图库和图像
    for lib_idx in sorted(middle_refs.keys()):
        lib_file = os.path.join(data_dir, f"Tiles{lib_idx}.Lib")
        
        if not os.path.exists(lib_file):
            print(f"\n⚠️  图库不存在: Tiles{lib_idx}.Lib")
            continue
        
        print(f"\n📚 Tiles{lib_idx}.Lib - {len(middle_refs[lib_idx])} 个不同图像被引用")
        
        # 检查前5个最常用的图像
        sorted_images = sorted(middle_refs[lib_idx].items(), key=lambda x: x[1], reverse=True)[:5]
        
        for img_idx, count in sorted_images:
            size = read_lib_image_size(lib_file, img_idx)
            if size:
                width_img, height_img = size
                is_valid = (width_img == 48 and height_img == 32) or (width_img == 96 and height_img == 64)
                status = "✅" if is_valid else "❌"
                print(f"   {status} 图像#{img_idx}: {width_img}x{height_img} (引用{count}次)")
            else:
                print(f"   ⚠️  图像#{img_idx}: 无法读取尺寸 (引用{count}次)")
    
    print(f"\n{'='*80}")

if __name__ == '__main__':
    check_0map_middle_layer()
