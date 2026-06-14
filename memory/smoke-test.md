# Memory Smoke Test — simple-proxy

> A cheap, manual check that the memory layer can actually orient a newcomer. **A fresh
> agent answers these from `memory/` alone** — no source code, no asking the user — then
> marks each ✅ (answerable from memory) or ❌ (gap). An ❌ is a *memory* gap: fix it by
> adding the missing fact, never by softening the question. App-level memory evaluation
> is an unsolved, bespoke problem industry-wide; this is the no-code, markdown version.

## How to run

1. Read **only** `memory/instructions.md`, `memory/continuity.md`, the latest 2–3
   `memory/sessions/`, and `memory/archive/INDEX.md`. Do not read source or ask the user.
2. Answer each question from those alone; mark ✅ or ❌ (with a one-line note on misses).
3. Append a row to the **Result log**. For each ❌, add the missing fact to memory (or
   open a thread to capture it) — then the next run should pass.

Run it **on demand** ("run the memory smoke test"), after a large change, or alongside a
review. Don't edit the questions to make them pass.

## Orientation questions (generic — apply to any repo)

1. What does this project do, and what type is it? *(→ instructions "What This Project Is")*
2. What is the stack — language, key dependencies, versions? *(→ continuity "Stack & Tools")*
3. What are the architectural invariants — things that must never change? *(→ continuity "Architectural Invariants")*
4. What were the last 2–3 key decisions, and **why**? *(→ continuity "Key Decisions" / recent sessions)*
5. What is in progress right now? *(→ continuity "Open Threads")*
6. What conventions should new code follow? *(→ instructions / continuity "Conventions")*
7. Any recorded user preferences or team / agent assignments? *(→ continuity — explicit only)*
8. Has any past decision been reversed or **superseded** — and by what? *(→ continuity superseded facts / `archive/INDEX.md`)*

## Project-specific questions (seeded at enable; grow these as the project does)

9. How does `serve` discover the target VM's IP, and what config makes discovery optional? *(→ continuity "Key Decisions" / instructions)*
10. What gates inbound connections in `serve`, and which subcommand has **no** allow-list by design? *(→ continuity "Architectural Invariants")*
11. Why is the core relay a hand-rolled `tokio::select!` loop rather than `copy_bidirectional`? *(→ continuity "Key Decisions": relay-design)*
12. What does the `event-bus` crate provide, and is it in the proxy's data path? *(→ continuity: event-bus-module)*

## Result log

| Date | Through session | Score (✅/total) | Gaps found → action |
|---|---|---|---|
| 2026-06-14 | (v3.7.0 upgrade) | — | baseline — run the test to populate |
