# MEVDAN — Automation Specification

Automation in MEVDAN is **rules-based**. A rule combines a trigger
with actions.

```text
   Trigger              Actions
 ┌─────────┐          ┌──────────┐
 │ File .rs│  ──►     │ cargo    │
 └─────────┘          │ test     │
                      └──────────┘
Concepts
Trigger
What fires a rule.

rust
pub enum Trigger {
    FileChange { path: String },
    Event { name: String },
    Schedule { every_minutes: u32 },
    Manual,
}
FileChange — a file matching path changed. path supports
exact names (main.rs), extensions (.rs), and prefix wildcards
(src/*).

Event — a named event occurred (task.failed,
refactor.start, ...).

Schedule — every N minutes (checked against a tick).

Manual — user-triggered.

Action
What to do when a rule fires.

rust
pub enum Action {
    RunCommand { program: String, args: Vec<String> },
    CreateCheckpoint { label: Option<String> },
    Handoff { from_role: String, to_role: String, reason: String },
    RunWorkflow { name: String },
    Notify { message: String },
}
The engine does not execute actions. It records firings and
returns matching rules. Execution is the runtime's responsibility.

Rule
A rule is a name + trigger + list of actions.

rust
let rule = Rule::new(
    "test-on-rs-change",
    Trigger::file_change(".rs"),
    vec![Action::run_command_with_args("cargo", ["test"])],
)?
.with_description("Run tests when a Rust file changes");
Rules have:

An enabled flag (can be toggled).

An id (UUID v7).

A created_at timestamp.

An optional description.

RuleEngine
rust
let mut engine = RuleEngine::new();
engine.add_rule(rule)?;

// Evaluate against an input.
let firings = engine.evaluate_and_record(TriggerInput::file_changed("main.rs"));
Operations:

add_rule(rule) — register (fails on duplicate name).

set_rule(rule) — register or replace.

remove_rule(name) — delete.

get_rule(name) / list_rules() — lookup.

evaluate(input) — return matching rules without executing.

evaluate_and_record(input) — return matches and record firings.

firings() / results() — history.

clear_history() — reset firings and results (keeps rules).

to_json() — serialize rules only.

TriggerInput
The engine is driven by TriggerInputs:

rust
pub enum TriggerInput {
    FileChanged { path: PathBuf },
    Event { name: String },
    Tick { elapsed_minutes: u32 },
    Manual,
}
Rules
Rule names are unique per engine.

Rule names are validated (lowercase, digits, -, _).

A rule must have at least one action.

Disabled rules do not match.

Schedule zero never matches.

What this is NOT
No background scheduler. MEVDAN does not run rules
automatically. The runtime feeds TriggerInputs.

No file watcher. FileChange triggers compare against input,
not the filesystem.

No action execution. The engine evaluates; the runtime
executes.

No cron syntax. Only simple intervals (every N minutes).

Design notes
Explicit, not implicit. Every firing is recorded.

No side effects in evaluate. evaluate is pure.

Failures are visible. A rule that can't match just returns
empty.

JSON-serializable. Rules can be persisted and reloaded.
