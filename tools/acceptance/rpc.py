#!/usr/bin/env python3
"""通用 control RPC 调用：rpc.py <method> [json params]"""
import json, socket, sys

def rpc(method, params=None, port=9000):
    req = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params or {}}
    with socket.create_connection(("127.0.0.1", port), timeout=10) as s:
        s.sendall((json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8"))
        f = s.makefile("r", encoding="utf-8", errors="replace")
        return json.loads(f.readline().strip())

if __name__ == "__main__":
    m = sys.argv[1]
    p = json.loads(sys.argv[2]) if len(sys.argv) > 2 else {}
    print(json.dumps(rpc(m, p), ensure_ascii=False, indent=1))
