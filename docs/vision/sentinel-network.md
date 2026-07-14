# The Sentinel Network — North Star

> "The real sentinel is the one where all installed crew across the world can talk
> to each other. That is the end state." — the vision, 2026-07-13.

This is the long-horizon vision for crew's inter-agent communication. It is
deliberately larger than any single spec. Each phase ships standalone value and
is a strict superset of the last — the engine built in v1 is the same engine
that runs the network in v3. Nothing here requires a rewrite of what came before;
each phase widens two things only: **the resolver** (what an address can name)
and **the transport** (how the envelope travels).

## The through-line

Stop addressing *machines*. Start addressing *questions*.

A crew user runs agents in panes. Those agents get stuck on things a *different*
agent already knows — the schema, the deploy state, an API contract, what another
team's service returns. Today that knowledge is siloed per pane, per machine, per
person. The sentinel network dissolves those walls: any agent can surface a
question and have it answered by whatever agent, anywhere, is best placed to
answer it — visibly, with the asker never blocked and always able to fall back.

The unit of progress is not "connect pane A to pane B." It is: **a question finds
its answer across whatever boundary sits between them** — pane, process, machine,
person, org.

## Phases

### v1 — Targeted, local  *(designed: [inter-pane-ask](../superpowers/specs/2026-07-13-inter-pane-ask-design.md))*

One agent asks one named pane in the same crew instance. Visible in-session
exchange, cooperative sentinel answer boundary, and a liveness-governed wait
(keep waiting while the target genuinely generates; give a clear negative verdict
otherwise). This builds the whole engine once, correctly:
- the request/verdict **envelope** (transport-agnostic),
- the **sentinel** answer protocol (location-independent),
- the **liveness/verdict** model (`ANSWERED` / `NO_ANSWER{reason}`),
- an opaque, **resolver-mediated address**.

### v2 — Broadcast  *(the highest-value step)*

`crew ask --any "<question>"`. The asker stops choosing a target; crew fans the
question out to all eligible idle agent panes and collects verdicts, returning the
best/first real answer. This is the hinge of the whole vision — it converts a
*directory of named panes* into a *network you query by need*. Per-target logic is
unchanged from v1; broadcast is fan-out over N resolved addresses, N verdicts,
one selection. Adds: eligibility/consent (which panes opt in to being asked),
answer selection/ranking, and dedupe.

### v3 — Federated, global

The address widens from `schema` to `schema@alice-crew` to `crew://<instance>/…`,
and the transport widens from a local Unix socket to a network relay between crew
instances. Now `--any` reaches beyond one machine: a question can be answered by
an agent in *someone else's* crew, across the world. The sentinel protocol and the
verdict model are **already** location-independent — v3 is a bigger resolver
(a federated directory: which instances/agents are reachable and asking-eligible)
and a bigger transport (an authenticated relay), wrapped around the identical
per-question engine.

Hard problems v3 introduces (not v1/v2): identity & trust between instances,
consent & rate-limiting (an agent must opt in to being reachable and cap load),
privacy (what a question/answer may reveal), discovery at scale (a federated
directory, not a flat roster), and abuse resistance. These are network-design
problems layered *on top of* a working local engine — which is exactly why v1
builds the engine first and keeps every interface federation-shaped.

## Invariant across all phases

The asker's experience never changes as the network grows:

```
ask a question  →  wait only while a real answer is being generated
                →  get the answer, or a clear reason it won't come
                →  never block forever; always able to fall back
```

Whether the answer comes from one pane over or one continent over is a resolver
and transport detail. Keep it that way.
