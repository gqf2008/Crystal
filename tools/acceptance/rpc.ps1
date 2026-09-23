# rpc.ps1 — 一行调用客户端 control RPC（9000 端口，JSON-RPC over TCP，逐行）
# 用法： powershell -File rpc.ps1 -Method click -Params '{"x":12,"y":78}'
param([Parameter(Mandatory = $true)][string]$Method, [string]$Params = '{}')
$ErrorActionPreference = 'Stop'
$c = New-Object Net.Sockets.TcpClient
$c.Connect('127.0.0.1', 9000)
$s = $c.GetStream()
$req = "{`"jsonrpc`":`"2.0`",`"id`":1,`"method`":`"$Method`",`"params`":$Params}`n"
$b = [Text.Encoding]::UTF8.GetBytes($req)
$s.Write($b, 0, $b.Length)
$s.Flush()
$r = New-Object IO.StreamReader($s)
$line = $r.ReadLine()
$c.Close()
$line
