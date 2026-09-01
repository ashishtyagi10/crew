# Sidecars

An engine crew did not compile in, running crew's tasks over a JSON-line
protocol on stdin/stdout. It is opt-in in every direction: unset by default,
probed before it is spawned, and reported by `/doctor` either way. On a machine
that cannot run it, crew runs natively and nothing else changes.

```sh
export CREW_SIDECAR="python3 $(pwd)/crew_sidecar.py"
crew                       # /doctor now says: ✓ sidecar: … — swarm tasks run there
```

`crew_sidecar.py` is the whole protocol in one readable file — read it before
writing your own. The two rules it exists to demonstrate:

* **A sidecar never holds a credential.** It asks crew to run a tool by name;
  crew's gate decides, crew's ledger records it, and the output comes back.
* **`state` is the sidecar's.** crew returns whatever the sidecar last handed
  it, verbatim, and never looks inside — which is where a graph engine keeps
  the checkpoint that makes it resumable.

A LangGraph graph goes where `run_task` is: the loop above it is all crew needs
you to speak.
