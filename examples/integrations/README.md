# Integration manifests

An HTTP API becomes a set of crew tools from one file — no Rust, no recompile.
Copy a manifest into either directory and it is live on the next task:

* `~/.config/crew/integrations/` — yours, on every project.
* `.crew/integrations/` — this project's; a manifest here replaces a user one
  of the same name.

`weather.json` needs no account and is the one to copy first: ask a `/crew`
pane *"what is the weather at 59.91, 10.75"* and watch the tool card.

The format is documented in [`docs/CREW.md`](../../docs/CREW.md) — search for
**Reaching an API is one file**. Two rules worth repeating here:

* **A manifest never holds a secret.** `auth` names an environment variable.
* **A tool with no `tier` is irreversible**, so it asks before it acts.
