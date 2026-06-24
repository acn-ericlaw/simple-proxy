# Feedback on Agent-Memory Skills Protocol

Here are a few observations from a recent experience creating, deleting, and re-creating a skill (`greeting`) using the agent-memory skills protocol:

## 1. Skill Deletion Path is Unclear
While `SKILLS.md` clearly documents how to author, sync, and adopt skills, it does not explicitly outline how to delete or deprecate a skill. I had to manually delete the directory from `agent-skills/` and then manually delete the vendor-specific adapters (`.claude`, `.github`, `.cursor`, etc.). A documented `delete skill` or `prune skill` operation that cleans up both the source of truth and all generated adapters would be a helpful addition to `SKILLS.md` and the tooling.

## 2. Sync Skill Adapters Tooling Errors
When I tried to run `sync skill adapters` as instructed in `SKILLS.md`:
* The built-in memory-lint approach (`node agent-skills/memory-lint/scripts/memory-lint.mjs sync skill adapters`) just ran the linter instead of syncing the adapters.
* Attempting to run `npm run` or looking for an npm script failed because there is no `package.json` at the repo root.
* Trying to run `npx mcp-agent-memory sync-skills` threw an npm error trying to install the package.
* I ultimately had to manually reverse-engineer the sync script by reading the auto-sync hook (`.github/hooks/agent-memory-autosync.json`) and writing a bash loop to recreate the `.claude/`, `.gemini/`, `.cursor/`, `.kiro/`, and `.github/` adapter files.

It would be helpful if the `sync skill adapters` command had an explicit, unambiguous bash alias or script (e.g., `bash memory/scripts/sync-skills.sh`) guaranteed to work in any environment without relying on `npm` or `npx` (which might fail in a non-Node.js project).

## 3. Clear and Predictable Structure
On the positive side, the actual structure of the skills is fantastic. The separation of the "source of truth" in `agent-skills/` from the gitignored, generated vendor adapters is conceptually very clean. Once I understood the file shape required for the adapters, writing the manual bash script to recreate them was straightforward because the format is highly consistent across vendors.

## 4. Memory/Continuity Integration
Integrating the skill creation/deletion into `continuity.md` worked well. The concept of marking a skill ID as `tier: superseded` when deleting and re-creating it maps naturally to the truth-state editing rules in `DECAY.md`.