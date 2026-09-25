#!/usr/bin/env python3
"""审计：哪些「传了 Arial handle」的文本站点会画到中文。

判据不是变量名叫 font 还是 cjk —— 名字会骗人（friend.rs 里 `let font = shared_cjk_font(..)`）。
这里按**绑定表达式**判定：`shared_cjk_font` → 自带 CJK ✓；`load_ui_font` / `ui_font.0.clone()`
→ Arial handle ✗（Arial 画中文是豆腐，Han 回退不可靠）。
再按调用点附近的字符串判断内容是否可能含中文：CJK 字面量、或非字面量（动态内容）都算风险。
"""
import re
import pathlib

CJK = re.compile(r'[一-鿿]')
BIND = re.compile(r'let (\w+)\s*=\s*(.+)$')
SPAWN = re.compile(r'\b(spawn_\w+|spawn_ui_text|spawn_outlined_label\w*)\s*\(')

def klass(expr):
    if 'shared_cjk_font' in expr or 'load_cjk_font' in expr:
        return 'CJK'
    if 'load_ui_font' in expr or 'ui_font' in expr:
        return 'ARIAL'
    return None

def main():
    risky = []
    for p in sorted(pathlib.Path('Client-Bevy/src').rglob('*.rs')):
        lines = p.read_text(encoding='utf-8').split('\n')
        binds = {}          # name -> 'CJK' | 'ARIAL'
        for i, ln in enumerate(lines):
            m = BIND.search(ln)
            if m:
                k = klass(m.group(2))
                if k:
                    binds[m.group(1)] = k
            if not SPAWN.search(ln):
                continue
            window = '\n'.join(lines[i:i + 10])
            params = re.findall(r'&(\w+)|&mut (\w+)', window)
            used = [a or b for a, b in params]
            for name in used:
                if binds.get(name) != 'ARIAL':
                    continue
                code = '\n'.join(l for l in lines[i:i + 10] if not l.strip().startswith('//'))
                if CJK.search(code):
                    risky.append((str(p), i + 1, name, 'CJK字面量'))
                elif re.search(r'""', code) and re.search(r'\bText::new\(String::new|spawn_label\w*\([^)]*""', code):
                    risky.append((str(p), i + 1, name, '空串(动态填充?)'))
                break
    print("Arial handle 且可能画中文的站点 %d 处：" % len(risky))
    for f, n, name, why in risky:
        print("  %s:%d  (%s, %s)" % (f, n, name, why))

if __name__ == '__main__':
    main()
