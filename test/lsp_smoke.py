#!/usr/bin/env python3
"""Smoke test for asp-ls: handshake + definition/hover/references/symbols/diagnostics."""
import json, subprocess, sys, os

SERVER = sys.argv[1]
WS = sys.argv[2]

proc = subprocess.Popen([SERVER], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                        stderr=subprocess.DEVNULL)
seq = 0

def send(method, params, notify=False):
    global seq
    msg = {"jsonrpc": "2.0", "method": method, "params": params}
    if not notify:
        seq += 1
        msg["id"] = seq
    body = json.dumps(msg).encode()
    proc.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    proc.stdin.flush()
    return seq

def recv():
    headers = {}
    while True:
        line = proc.stdout.readline().decode()
        if line in ("\r\n", "\n"):
            break
        k, v = line.split(":", 1)
        headers[k.strip().lower()] = v.strip()
    return json.loads(proc.stdout.read(int(headers["content-length"])))

def request(method, params):
    rid = send(method, params)
    while True:
        msg = recv()
        if msg.get("id") == rid:
            return msg
        # stash server notifications
        notifications.append(msg)

notifications = []
uri = lambda p: "file://" + p

root = os.path.realpath(WS)
default_asp = os.path.join(root, "default.asp")

request("initialize", {
    "processId": None,
    "rootUri": uri(root),
    "capabilities": {},
    "workspaceFolders": [{"uri": uri(root), "name": "ws"}],
})
send("initialized", {}, notify=True)

text = open(default_asp).read()
send("textDocument/didOpen", {"textDocument": {
    "uri": uri(default_asp), "languageId": "asp", "version": 1, "text": text}}, notify=True)

results = {}

# 1. definition on include directive (line 0) -> includes/header.asp
r = request("textDocument/definition", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 0, "character": 30}})
results["def_include"] = r.get("result")

# 2. definition on GetGreeting call (line 5: msg = GetGreeting("world"))
r = request("textDocument/definition", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 5, "character": 10}})
results["def_function"] = r.get("result")

# 3. hover on Response.Write (line 6)
r = request("textDocument/hover", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 6, "character": 12}})
results["hover_builtin"] = r.get("result")

# 4. hover on user function GetGreeting
r = request("textDocument/hover", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 5, "character": 10}})
results["hover_user"] = r.get("result")

# 5. hover on rs.MoveNext (line 9)
r = request("textDocument/hover", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 9, "character": 5}})
results["hover_ado"] = r.get("result")

# 6. references: who includes lib/db.asp (ask from db.asp itself)
db_asp = os.path.join(root, "lib", "db.asp")
r = request("textDocument/references", {
    "textDocument": {"uri": uri(db_asp)}, "position": {"line": 0, "character": 0},
    "context": {"includeDeclaration": False}})
results["references"] = r.get("result")

# 7. workspace/symbol
r = request("workspace/symbol", {"query": "conn"})
results["symbols"] = r.get("result")

# drain pending notifications (diagnostics arrive after didOpen)
import select, time
time.sleep(0.3)
while select.select([proc.stdout], [], [], 0.2)[0]:
    try:
        notifications.append(recv())
    except Exception:
        break

results["diagnostics"] = [n["params"] for n in notifications
                          if n.get("method") == "textDocument/publishDiagnostics"]

print(json.dumps(results, indent=1))
proc.kill()
