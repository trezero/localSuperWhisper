## Session Start — Read Current Status First

At the start of **every** session, before doing anything else:

1. Read `docs/currentStatus.md` — this is the authoritative record of where the project stands, what was last worked on, known issues, and next steps.
2. Briefly summarize to the user what the last session accomplished and what is ready to work on next.
3. Then proceed with the Archon KB check below.

<!-- archon-rules-start -->
## Archon Knowledge Base — Ambient Behavio

Archon is a RAG knowledge management system connected via MCP (`archon` server). It provides semantic search across project documentation and shared cross-project knowledge. Use the `/archon-memory` skill for explicit operations.

### Session Start — Always Show Status
At the start of every session, check Archon state and display a **one-liner status** to the user:

1. Check if `.claude/archon-state.json` exists in the project
2. If yes, read it and note the Archon `project_id` and `source_id` for searches
3. Check doc freshness: compute MD5 hashes of docs vs stored hashes (use `md5sum <file> | cut -d' ' -f1` on Linux, `md5 -q` on macOS)
4. Display one of these status lines:

   - **Configured & fresh:** `Archon KB: <project> — <N> docs, synced <relative time>, up to date`
   - **Configured & stale:** `Archon KB: <project> — <N> docs, synced <relative time>, <N> files changed. Run /archon-memory sync`
   - **Not configured:** `Archon KB: not configured. Run /archon-memory ingest to set up.`
   - **Server unreachable:** `Archon KB: server unreachable — search unavailable this session`

This check should be quick (read state file + hash a few files). Do NOT call Archon APIs for this — just use local state.

### During Normal Work
- When needing project context (architecture, patterns, deployment, historic issues):
  PREFER `rag_search_knowledge_base(query, project_id)` over reading raw doc files
- Archon search is faster and uses less context than reading entire files
- Fall back to direct file reads only when Archon search returns no relevant results
- For code pattern questions, also try `rag_search_code_examples(query, project_id)`

### When Docs Are Modified
- If documentation files are modified during a session, Archon knowledge is stale
- Remind user to run `/archon-memory sync` before ending the session

### Cross-Project Knowledge
- Shared knowledge (framework docs, tool patterns) is available via `~/.claude/archon-global.json`
- Search shared KB: `rag_search_knowledge_base(query, project_id=shared_project_id)`
<!-- archon-rules-end -->
