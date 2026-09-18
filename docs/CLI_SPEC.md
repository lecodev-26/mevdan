# MEVDAN — CLI Specification

The command-line interface. Everything MEVDAN can do is reachable
from the CLI.

## Binary

```text
mevdan 0.5.0
Global
bash
mevdan --version
mevdan --help
Commands
mevdan init <name>
Create a new project.

bash
mevdan init demo
mevdan init demo --path /tmp
Creates:

<path>/<name>/.mevdan/project.toml

<path>/<name>/.mevdan/mevdan.db

<path>/<name>/.mevdan/artifacts/

<path>/<name>/.mevdan/checkpoints/

<path>/<name>/.mevdan/events/

<path>/<name>/.mevdan/worktrees/

Also inserts a Project, an initial Session, and a
ProjectCreated event.

mevdan status
Show the current project's status.

bash
cd demo
mevdan status
mevdan chat <message>
Send a message to a provider.

bash
mevdan chat "Hello" -m llama3.2 -p ollama

mevdan chat "Hello" -m gpt-4o-mini \
    -p openai-compatible \
    --base-url https://api.openai.com/v1 \
    --api-key-secret openai_api_key
mevdan tasks
List tasks from the project's Work Graph.

bash
mevdan tasks
mevdan tasks --status ready
mevdan tasks --verbose
mevdan audit
Show the audit trail of the project.

bash
mevdan audit
mevdan audit --limit 20
mevdan audit --category lifecycle
mevdan audit --json
mevdan skills list
List installed skills.

bash
mevdan skills list
mevdan skills list --verbose
mevdan mcp list
List configured MCP servers.

bash
mevdan mcp list
mevdan mcp list --verbose
mevdan codeintel map
Show the repository map.

bash
mevdan codeintel map
mevdan codeintel map --json
mevdan route <text>
Show the routing decision for a piece of text.

bash
mevdan route "write tests for the parser"
mevdan route "fix the bug" --json
mevdan worktree list
List worktrees.

bash
mevdan worktree list
mevdan worktree list --verbose
mevdan docs read <path>
Read a document (Markdown, Text, CSV, JSON).

bash
mevdan docs read README.md
mevdan docs read data.json --json
mevdan data summary <path>
Summarize a dataset (CSV, JSON, JSONL).

bash
mevdan data summary people.csv
mevdan data summary data.json --json
mevdan media info <path>
Show media info (image, audio, video).

bash
mevdan media info image.png
mevdan media info song.wav --json
Working directory
All commands except init look for .mevdan/project.toml in the
current directory or any parent. If not found, the command fails
with a clear error.

Exit codes
0 — success.

1 — error (invalid args, missing project, provider error).

Planned commands (V6+)
mevdan memory — manage memory.

mevdan desktop — launch desktop UI.

mevdan android — Android integration.

mevdan import / mevdan export — portable work package.

mevdan doctor — diagnose environment.

Environment
HOME — used to find ~/.config/mevdan/.

No other environment variables are required.
