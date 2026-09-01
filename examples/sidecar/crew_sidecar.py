#!/usr/bin/env python3
"""A crew sidecar in one file: the smallest thing that speaks crew's bridge.

Run it by setting, in the environment crew's broker sees:

    export CREW_SIDECAR="python3 /path/to/crew_sidecar.py"

Then every task in a `/crew` swarm is executed HERE instead of by crew's own
agent, and `/doctor` says so. Nothing else changes: crew still plans the graph,
still owns the tools, still gates and ledgers every call.

The protocol is one JSON object per line, both ways:

    crew  -> {"kind":"task","task":1,"prompt":"…","tools":[…],"state":…}
    you   -> {"kind":"delta","text":"thinking…"}          (0 or more)
    you   -> {"kind":"call","id":"c1","tool":"sys:run","args":"{\"cmd\":\"ls\"}"}
    crew  -> {"kind":"result","id":"c1","output":"…","ok":true}
    you   -> {"kind":"done","task":1,"output":"…","success":true,
              "input_tokens":0,"output_tokens":0,"state":{…}}

Two things worth knowing before you write a real one:

* **You never hold a credential.** Ask crew to run the tool; crew's gate
  decides, crew's ledger records it, and you get the output. A sidecar with its
  own API key would be a second, unaudited way for crew to reach the world.
* **`state` is yours.** crew hands back whatever you last returned and never
  looks inside it. That is where a graph engine keeps its checkpoint.

This example answers by listing what it was given and running one tool, which
is enough to see the whole loop in the pane. Replace `run_task` with a
LangGraph graph, a workflow engine, or whatever the job actually wants.
"""
import json
import sys


def say(msg):
    print(json.dumps(msg), flush=True)


def run_task(task, ask):
    """Answer one task. `ask(tool, args)` runs a crew tool and returns (output, ok)."""
    say({"kind": "delta", "text": "sidecar: thinking about %r\n" % task["prompt"][:60]})
    tools = [t["name"] for t in task.get("tools", [])]
    lines = ["ran in a sidecar (%d tool(s) offered)" % len(tools)]
    if "sys:list_dir" in tools:
        output, ok = ask("sys:list_dir", json.dumps({"path": "."}))
        lines.append("sys:list_dir %s: %s" % ("ok" if ok else "failed", output[:200]))
    step = (task.get("state") or {}).get("step", 0) + 1
    return "\n".join(lines), {"step": step}


def main():
    pending = {}
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        msg = json.loads(line)
        if msg["kind"] == "task":
            task = msg
            calls = []

            def ask(tool, args):
                cid = "c%d" % (len(calls) + 1)
                calls.append(cid)
                say({"kind": "call", "id": cid, "tool": tool, "args": args})
                reply = json.loads(sys.stdin.readline())
                if reply.get("kind") != "result":
                    return ("crew answered something else", False)
                return (reply["output"], reply["ok"])

            try:
                output, state = run_task(task, ask)
                ok = True
            except Exception as e:  # a sidecar that dies takes the graph with it
                output, state, ok = "sidecar error: %s" % e, None, False
            say({
                "kind": "done",
                "task": task.get("task", 0),
                "output": output,
                "success": ok,
                "input_tokens": 0,
                "output_tokens": 0,
                "state": state,
            })
        elif msg["kind"] == "result":
            # Only reachable if a result arrives with nothing waiting for it.
            pending[msg.get("id")] = msg


if __name__ == "__main__":
    main()
