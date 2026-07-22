#!/usr/bin/env python3
"""Smoke test for asp-ls: handshake + definition/hover/references/symbols/diagnostics.

Exits nonzero when any check fails, so it can gate CI / pre-publish runs.
"""
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

# default.asp (0-indexed): 0-3 includes, 4 '<%', 5 'Dim msg',
# 6 'msg = GetGreeting("world")', 7 'Response.Write msg', 9 'Set rs = ...', 10 'rs.MoveNext'

# 1. definition on include directive (line 0) -> includes/header.asp
r = request("textDocument/definition", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 0, "character": 30}})
results["def_include"] = r.get("result")

# 2. definition on GetGreeting call
r = request("textDocument/definition", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 6, "character": 10}})
results["def_function"] = r.get("result")

# 3. hover on Response.Write
r = request("textDocument/hover", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 7, "character": 12}})
results["hover_builtin"] = r.get("result")

# 4. hover on user function GetGreeting
r = request("textDocument/hover", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 6, "character": 10}})
results["hover_user"] = r.get("result")

# 5. hover on rs.MoveNext
r = request("textDocument/hover", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 10, "character": 5}})
results["hover_ado"] = r.get("result")

# 5a. definition on root-absolute file= include (line 12) -> lib/db.asp
r = request("textDocument/definition", {
    "textDocument": {"uri": uri(default_asp)}, "position": {"line": 12, "character": 25}})
results["def_abs_include"] = r.get("result")

# 6. references: who includes lib/db.asp (ask from db.asp itself)
db_asp = os.path.join(root, "lib", "db.asp")
r = request("textDocument/references", {
    "textDocument": {"uri": uri(db_asp)}, "position": {"line": 0, "character": 0},
    "context": {"includeDeclaration": False}})
results["references"] = r.get("result")

# 7. workspace/symbol
r = request("workspace/symbol", {"query": "conn"})
results["symbols"] = r.get("result")

# The server handles messages in order, so once this final request is answered
# every earlier notification (diagnostics) has been received and stashed.
request("workspace/symbol", {"query": "zzz-flush"})

diags = {}
for n in notifications:
    if n.get("method") == "textDocument/publishDiagnostics":
        diags[n["params"]["uri"]] = n["params"]["diagnostics"]
results["diagnostics"] = diags

failures = []
def check(name, ok):
    if not ok:
        failures.append(name)

check("def_include", results["def_include"])
check("def_function", results["def_function"])
check("hover_builtin", results["hover_builtin"])
check("hover_user", results["hover_user"])
check("hover_ado", results["hover_ado"])
check("def_abs_include", results["def_abs_include"])
check("references", results["references"])
check("symbols", results["symbols"])
# default.asp has two unresolvable includes: missing.asp and 없는파일.asp
check("diagnostics", len(diags.get(uri(default_asp), [])) == 2)

print(json.dumps(results, indent=1))
proc.kill()
if failures:
    print("FAILED: " + ", ".join(failures), file=sys.stderr)
    sys.exit(1)
print("OK: all checks passed", file=sys.stderr)
