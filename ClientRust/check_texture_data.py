#!/usr/bin/env python3
"""检查Tiles.Lib中的第一张图片的原始数据"""

import struct
import gzip
from pathlib import Path

def check_lib_image(lib_path, image_index=0):
    """检查图库文件中的图像数据"""
    print(f"\n📁 检查文件: {lib_path}")
    
    with open(lib_path, 'rb') as f:
        # 读取文件头
        version = struct.unpack('<i', f.read(4))[0]
        count = struct.unpack('<i', f.read(4))[0]
        
        print(f"版本: {version}")
        print(f"图像数量: {count}")
        
        if version >= 3:
            frame_seek = struct.unpack('<i', f.read(4))[0]
        else:
            frame_seek = 0
        
        # 读取索引表
        indices = []
        for i in range(count):
            offset = struct.unpack('<i', f.read(4))[0]
            indices.append(offset)
        
        # 读取指定图像
        if image_index >= count:
            print(f"❌ 图像索引 {image_index} 超出范围 (max: {count-1})")
            return
        
        offset = indices[image_index]
        print(f"\n🖼️  图像 #{image_index} 偏移: {offset}")
        
        # 跳转到图像数据
        f.seek(offset)
        
        # 读取图像头信息 (17字节)
        width = struct.unpack('<h', f.read(2))[0]
        height = struct.unpack('<h', f.read(2))[0]
        x = struct.unpack('<h', f.read(2))[0]
        y = struct.unpack('<h', f.read(2))[0]
        shadow_x = struct.unpack('<h', f.read(2))[0]
        shadow_y = struct.unpack('<h', f.read(2))[0]
        shadow = struct.unpack('B', f.read(1))[0]
        length = struct.unpack('<i', f.read(4))[0]
        
        print(f"  尺寸: {width}x{height}")
        print(f"  偏移: ({x}, {y})")
        print(f"  Shadow: ({shadow_x}, {shadow_y}) flag={shadow}")
        print(f"  压缩长度: {length} 字节")
        
        has_mask = (shadow >> 7) == 1
        print(f"  Has Mask: {has_mask}")
        
        # 读取压缩数据
        compressed_data = f.read(length)
        print(f"  实际读取: {len(compressed_data)} 字节")
        
        # 解压数据
        try:
            decompressed = gzip.decompress(compressed_data)
            expected_size = width * height * 4
            print(f"  解压后: {len(decompressed)} 字节 (预期: {expected_size})")
            
            if len(decompressed) >= 16:
                # 显示前4个像素 (BGRA格式)
                print(f"\n  前4个像素 (BGRA格式):")
                for i in range(min(4, len(decompressed) // 4)):
                    b = decompressed[i*4]
                    g = decompressed[i*4+1]
                    r = decompressed[i*4+2]
                    a = decompressed[i*4+3]
                    print(f"    像素{i}: B={b:3d} G={g:3d} R={r:3d} A={a:3d} " + 
                          f"-> RGBA=({r:3d},{g:3d},{b:3d},{a:3d})")
                
                # 统计颜色分布
                black_pixels = 0
                transparent_pixels = 0
                opaque_pixels = 0
                
                for i in range(0, len(decompressed), 4):
                    b = decompressed[i]
                    g = decompressed[i+1]
                    r = decompressed[i+2]
                    a = decompressed[i+3]
                    
                    if a == 0:
                        transparent_pixels += 1
                    elif a == 255:
                        opaque_pixels += 1
                    
                    if r == 0 and g == 0 and b == 0:
                        black_pixels += 1
                
                total_pixels = len(decompressed) // 4
                print(f"\n  统计 (共{total_pixels}像素):")
                print(f"    透明 (A=0): {transparent_pixels} ({transparent_pixels*100//total_pixels}%)")
                print(f"    不透明 (A=255): {opaque_pixels} ({opaque_pixels*100//total_pixels}%)")
                print(f"    黑色 (RGB=0,0,0): {black_pixels} ({black_pixels*100//total_pixels}%)")
        
        except Exception as e:
            print(f"  ❌ 解压失败: {e}")

if __name__ == '__main__':
    # 检查Tiles.Lib的前几张图片
    lib_path = Path("Data/Map/WemadeMir2/Tiles.Lib")
    
    if lib_path.exists():
        for i in [0, 1, 10, 100]:
            check_lib_image(lib_path, i)
    else:
        print(f"❌ 文件不存在: {lib_path}")
