#!/usr/bin/env python3
"""
批量重命名 ECS 组件，去掉 Component 后缀
"""
import os
import re
from pathlib import Path

# 组件重命名映射
COMPONENT_RENAMES = {
    'PlayerInput': 'PlayerInput',
    'Prediction': 'Prediction',
    'ServerState': 'ServerState',
    'Interpolation': 'Interpolation',
    'MovementVelocity': 'MovementVelocity',
    'Path': 'Path',
    'MovementState': 'Movement',  # 避免与枚举 MovementState 冲突
    'AnimationState': 'Animation',  # 避免与枚举 AnimationState 冲突
    'SoundTrigger': 'SoundTrigger',
    'PersistentSound': 'PersistentSound',
}

def replace_in_file(filepath):
    """在文件中替换组件名称"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        # 执行替换（需要特殊处理以避免误替换）
        for old_name, new_name in COMPONENT_RENAMES.items():
            if old_name == new_name:
                continue
            
            # 特殊处理: struct MovementState / struct AnimationState
            if old_name in ['MovementState', 'AnimationState']:
                # 只替换 struct 定义和类型引用
                # 1. struct 定义
                pattern = r'pub struct ' + re.escape(old_name) + r'\b'
                content = re.sub(pattern, f'pub struct {new_name}', content)
                
                # 2. impl 块
                pattern = r'impl ' + re.escape(old_name) + r'\b'
                content = re.sub(pattern, f'impl {new_name}', content)
                
                # 3. Default trait
                pattern = r'impl Default for ' + re.escape(old_name) + r'\b'
                content = re.sub(pattern, f'impl Default for {new_name}', content)
                
                # 4. 类型引用（但不包括 :: 后的枚举）
                pattern = r'(?<!::)(?<!enum )\b' + re.escape(old_name) + r'(?=[\s,<>)])}'
                content = re.sub(pattern, new_name, content)
                
                # 5. Option/&/&mut 等引用
                pattern = r'(Option<&(?:mut )?)' + re.escape(old_name) + r'>'
                content = re.sub(pattern, rf'\1{new_name}>', content)
                
                pattern = r'&(?:mut )?' + re.escape(old_name) + r'(?=[\s,>)])'
                content = re.sub(pattern, lambda m: m.group(0).replace(old_name, new_name), content)
        
        # 如果有更改，写回文件
        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    """主函数"""
    # 扫描 src 目录下所有 .rs 文件
    src_dir = Path('src')
    changed_files = []
    
    for rust_file in src_dir.rglob('*.rs'):
        if replace_in_file(rust_file):
            changed_files.append(rust_file)
            print(f"✅ Updated: {rust_file}")
    
    print(f"\n📊 总计修改了 {len(changed_files)} 个文件")
    print("\n修改完成！请运行 'cargo build' 检查是否有遗漏的引用。")

if __name__ == '__main__':
    main()
