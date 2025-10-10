#!/usr/bin/env python3
"""快速检查地图文件的各层数据"""

import struct
import os
from pathlib import Path

def check_map_type_100(filepath):
    """检查Type 100格式地图"""
    with open(filepath, 'rb') as f:
        data = f.read()
    
    # 检查魔术字节
    if data[2] != 0x43 or data[3] != 0x23:
        return None
    
    # 读取宽高
    width, height = struct.unpack('<HH', data[4:8])
    
    print(f"\n{'='*80}")
    print(f"📄 文件: {os.path.basename(filepath)}")
    print(f"📐 尺寸: {width}x{height}")
    print(f"📦 格式: Type 100 (C# 自定义)")
    print(f"{'='*80}")
    
    # 统计各层
    back_count = 0
    middle_count = 0
    front_count = 0
    
    offset = 8  # 数据从offset 8开始
    cell_size = 24  # 每格24字节
    
    for x in range(width):
        for y in range(height):
            # BackIndex (2字节)
            back_index = struct.unpack('<h', data[offset:offset+2])[0]
            offset += 2
            
            # BackImage (4字节)
            back_image = struct.unpack('<i', data[offset:offset+4])[0]
            offset += 4
            
            # MiddleIndex (2字节)
            middle_index = struct.unpack('<h', data[offset:offset+2])[0]
            offset += 2
            
            # MiddleImage (2字节)
            middle_image = struct.unpack('<h', data[offset:offset+2])[0]
            offset += 2
            
            # FrontIndex (2字节)
            front_index = struct.unpack('<h', data[offset:offset+2])[0]
            offset += 2
            
            # FrontImage (2字节)
            front_image = struct.unpack('<h', data[offset:offset+2])[0]
            offset += 2
            
            # 跳过剩余12字节
            offset += 12
            
            # 统计非空格子
            if back_image != 0:
                back_count += 1
            if middle_image != 0:
                middle_count += 1
            if front_image != 0:
                front_count += 1
            
            # 显示第一个格子的详细信息
            if x == 0 and y == 0:
                print(f"\n📍 第一个格子 (0,0):")
                print(f"   BackIndex: {back_index}, BackImage: {back_image} (0x{back_image:08x})")
                print(f"   MiddleIndex: {middle_index}, MiddleImage: {middle_image}")
                print(f"   FrontIndex: {front_index}, FrontImage: {front_image}")
    
    total_cells = width * height
    print(f"\n📊 各层统计:")
    print(f"   Back层:   {back_count:6d} / {total_cells} ({back_count*100.0/total_cells:5.1f}%)")
    print(f"   Middle层: {middle_count:6d} / {total_cells} ({middle_count*100.0/total_cells:5.1f}%)")
    print(f"   Front层:  {front_count:6d} / {total_cells} ({front_count*100.0/total_cells:5.1f}%)")
    
    if middle_count == 0:
        print(f"\n⚠️  警告: Middle层完全为空！这可能导致看不到建筑物/树木！")
    
    return {
        'width': width,
        'height': height,
        'back': back_count,
        'middle': middle_count,
        'front': front_count
    }

def main():
    # 查找所有Type 100地图文件
    map_dirs = [
        r"d:\Users\gxh\Documents\GitHub\Crystal\ClientRust\Map",
        r"d:\Users\gxh\Documents\GitHub\Crystal\Build\Client\Map",
    ]
    
    print("\n" + "█"*80)
    print("🔍 检查地图文件的各层数据")
    print("█"*80)
    
    type100_maps = []
    
    for map_dir in map_dirs:
        if not os.path.exists(map_dir):
            continue
        
        print(f"\n📁 扫描目录: {map_dir}")
        
        for filepath in Path(map_dir).glob("*.map"):
            with open(filepath, 'rb') as f:
                header = f.read(4)
            
            # 检查是否是Type 100
            if len(header) >= 4 and header[2] == 0x43 and header[3] == 0x23:
                type100_maps.append(str(filepath))
    
    if not type100_maps:
        print("\n❌ 未找到Type 100格式的地图文件！")
        return
    
    print(f"\n✅ 找到 {len(type100_maps)} 个Type 100地图文件")
    
    for mapfile in type100_maps:
        try:
            check_map_type_100(mapfile)
        except Exception as e:
            print(f"\n❌ 解析失败: {mapfile}")
            print(f"   错误: {e}")
    
    print("\n" + "█"*80)

if __name__ == '__main__':
    main()
