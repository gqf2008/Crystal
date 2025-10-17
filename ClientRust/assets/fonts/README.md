# 字体说明

## 需要的字体文件

将以下字体文件放置在此目录:

1. `NotoSansSC-Regular.ttf` - 思源黑体(简体中文)
   - 下载地址: https://fonts.google.com/noto/specimen/Noto+Sans+SC
   - 或使用系统字体: C:\Windows\Fonts\msyh.ttc (微软雅黑)

## 临时解决方案

如果没有字体文件,程序会使用 Bevy 默认字体(不支持中文)。
建议从 Windows 字体目录复制一个:

```powershell
Copy-Item "C:\Windows\Fonts\msyh.ttc" -Destination ".\NotoSansSC-Regular.ttf"
```

## 字体许可

- Noto Sans SC: SIL Open Font License 1.1
- Microsoft YaHei: 仅限 Windows 系统使用
