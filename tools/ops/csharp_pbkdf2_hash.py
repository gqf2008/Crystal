"""按 **C# 原版口径**算一个密码哈希串（给从 .MirADB 迁过来的账号重置密码用）。

C#（Server/Utils/Crypto.cs:20-23）：
    Rfc2898DeriveBytes(password, salt, Iterations=50, SHA1).GetBytes(24)
    再 `Encoding.UTF8.GetString(bytes)` —— 24 字节被**当成 UTF-8 字符串**存盘
    （非法序列 → U+FFFD，因此落盘字节可能比 24 长）。

用法：
    python csharp_pbkdf2_hash.py <hex_salt> <password>
输出：`pbkdf2_sha1$<b64 salt>$<b64 lossy-hash>`（服务端 verify_password 认这个格式）
"""
import base64
import hashlib
import sys

salt = bytes.fromhex(sys.argv[1])
password = sys.argv[2]

raw = hashlib.pbkdf2_hmac("sha1", password.encode("utf-8"), salt, 50, dklen=24)
lossy = raw.decode("utf-8", errors="replace").encode("utf-8")  # 复刻 C# 的损失转换
print(
    "pbkdf2_sha1${}${}".format(
        base64.b64encode(salt).decode(), base64.b64encode(lossy).decode()
    )
)
