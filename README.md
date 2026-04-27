# memctl

> Persistent agent memory layer — cross-tool, cross-project, cross-session.

AI agents lose context at every `/clear`. CLAUDE.md is static and tool-private. `memctl` is the layer in between: a topic-based markdown store with timestamped entries, accessible from any Agent that can read `AGENTS.md` and execute a shell.

```bash
# Capture an observation (any Agent, any project)
memctl save --type decision --topic skillctl-design \
  "core/extra naming wins over baseline/on-demand: more human-friendly"

# Read it back from a different project, different Agent, different session:
memctl read --topic skillctl-design --format tsv
memctl search "naming"
```

## Seven types of memory

| Type | Use for |
|---|---|
| `lesson` | Recurring gotchas, hard-won experience |
| `decision` | Design decisions + rationale |
| `fact` | Domain knowledge (non-opinion) |
| `feedback` | Collaboration preferences |
| `reference` | Pointers to external systems |
| `user` | Who the user is |
| `project` | Current project state / context |

## Three scopes

| Scope | Path | Visible to |
|---|---|---|
| `global` (default) | `~/.memctl/global/topics/` | All projects, all Agents |
| `project` | `<project>/.memctl/topics/` | Only this repo (commit it!) |
| `agent:<name>` | `~/.memctl/agents/<name>/topics/` | Only that Agent |

Project-scoped memory commits with your code — new team members get the lore on `git clone`. Something Claude Code / Cursor private memory cannot do.

## Install

Pre-built binaries: see [Releases](https://github.com/agent-rt/memctl/releases).

Homebrew:

```bash
brew install agent-rt/tap/memctl
```

From source (Rust 1.83+):

```bash
git clone https://github.com/agent-rt/memctl
cd memctl
cargo install --path apps/memctl-cli --locked
```

## Quick start in a project

```bash
cd my-project
memctl init                         # creates .memctl/ + AGENTS.md block
memctl save --type fact --topic api-conventions \
  --scope project --no-confirm \
  "POST /payments must include Idempotency-Key header"
git add .memctl AGENTS.md
git commit -m "memctl: capture API conventions"
```

Now any Agent (Claude Code, Codex, Cursor, …) opening this repo will:

1. Read `AGENTS.md` → see the memctl block
2. Run `memctl list --format tsv` → discover topics
3. Run `memctl read --topic api-conventions` → get the convention
4. Apply it without you re-explaining

## Position in the Agent-RT family

| Tool | Layer |
|---|---|
| [`skillctl`](https://github.com/agent-rt/skillctl) | Curated capability bundles |
| [`memctl`](https://github.com/agent-rt/memctl) | Persistent memory (this) |
| [`acpctl`](https://github.com/agent-rt/acpctl) | ACP agent invocation |

Both `skillctl` and `memctl` install their own `AGENTS.md` managed block; they coexist without conflict.

## Documentation

- [`REQ.md`](https://github.com/agent-rt/memctl/blob/main/REQ.md) — product spec

## License

Apache-2.0
