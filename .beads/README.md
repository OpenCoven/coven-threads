# Beads task tracking

Use Beads for local project tasks, dependencies, and handoffs. Start with the
repository workflow:

```bash
bd prime
bd ready
```

Inspect and claim an existing task before implementation:

```bash
bd show <issue-id>
bd update <issue-id> --claim
```

Close a task only when its acceptance criteria are met:

```bash
bd close <issue-id> --reason="Describe the completed outcome"
```

## Storage and synchronization

Issues live in a local Dolt database. `.beads/issues.jsonl`, when present, is a
passive export, not the writable source of truth. Do not commit mutable
interaction logs to satisfy task bookkeeping.

Remote synchronization is separate from local tracking. Run `bd dolt push`
or `bd dolt pull` only when the active instructions authorize it; a Git push
does not establish that the issue database was synchronized.

## Repository boundaries

Follow [the root agent guide](../AGENTS.md) for canonical ownership, GitHub
issue requirements, human gates, and commit/push authority. Clean-clone
bootstrap and CI must not depend on a personal Beads database.

See [the Beads documentation](https://github.com/gastownhall/beads) for tracker
installation and command details.
