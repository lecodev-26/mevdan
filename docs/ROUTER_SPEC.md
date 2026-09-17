# MEVDAN — Router Specification

The **Router** decides, given a task, **which agent** and **which
model** should handle it.

Two questions, one abstraction:

- `AgentRouter` — which role?
- `ModelRouter` — which model?

**No ML. Rules only.** Every decision is auditable and testable.

## TaskKind

First step: classify the task.

```rust
use mevdan_router::TaskKind;

assert_eq!(TaskKind::classify("write tests for the parser"), TaskKind::Testing);
assert_eq!(TaskKind::classify("fix the bug in main"),       TaskKind::Debugging);
assert_eq!(TaskKind::classify("refactor the auth module"),  TaskKind::Refactoring);
assert_eq!(TaskKind::classify("plan the architecture"),     TaskKind::Planning);
Kinds:

Planning, Coding, Review, Research, Testing,
Documentation, DataAnalysis, Refactoring, Debugging, Other.

Classification is keyword-based. Order matters: more specific keywords
win over more generic ones. For example, "debug and test the bug"
classifies as Debugging (not Testing), because debug/bug are
checked first.

AgentRouter
Maps a TaskKind to an AgentRole.

TaskKind	Role
Planning	Planner
Coding, Refactoring, Debugging	Coder
Review	Reviewer
Testing	Tester
Research, DataAnalysis	Researcher
Documentation	Documenter
Other	Coder (fallback)
rust
use mevdan_router::AgentRouter;
use mevdan_agent::AgentRole;

let router = AgentRouter::new();
let decision = router.route_text("write tests for the parser")?;

assert_eq!(decision.task_kind, TaskKind::Testing);
assert_eq!(decision.role, AgentRole::Tester);
ModelRouter
Chooses a model from a list of candidates, given the task and a policy.

rust
use mevdan_router::{ModelCandidate, ModelRouter, RouterPolicy};

let router = ModelRouter::new();
let candidates = vec![
    ModelCandidate::new("gpt-4o", 128_000).with_tools().with_cost(2.5, 10.0),
    ModelCandidate::new("gpt-4o-mini", 128_000).with_tools().with_cost(0.15, 0.6),
    ModelCandidate::new("llama3.2", 32_000).with_tools(),
];

let decision = router.route(TaskKind::Coding, &candidates, RouterPolicy::Automatic)?;
ModelCandidate
A ModelCandidate is a simplified view of a model with the fields
that matter for routing:

rust
pub struct ModelCandidate {
    pub name: String,
    pub context_window: u32,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub input_cost_per_mtok: Option<f64>,   // USD per 1M input tokens
    pub output_cost_per_mtok: Option<f64>,  // USD per 1M output tokens
}
Helpers:

is_free() — no cost data or zero cost (typical for local models).

combined_cost() — input + output cost, or 0 if unknown.

Requirements
Each task has default requirements:

Task	Min context	Needs tools	Needs vision
Planning	32k	yes	no
Coding	64k	yes	no
Review	32k	no	no
Research	128k	no	no
Testing	32k	yes	no
Documentation	16k	no	no
DataAnalysis	64k	yes	no
Refactoring	64k	yes	no
Debugging	64k	yes	no
Other	—	no	no
Candidates that fail the requirements are excluded.

The requirements can be tuned:

rust
use mevdan_router::ModelRequirements;

let req = ModelRequirements::new()
    .with_min_context(100_000)
    .with_tools()
    .with_max_cost(1.0)
    .prefer_free();
Policy
rust
pub enum RouterPolicy {
    Manual,     // user picks explicitly (API respects this)
    Automatic,  // pick the best automatically
    Fallback,   // pick the best, keep the rest ordered for fallback
}
Selection algorithm
Candidates are sorted by:

Prefer free (if prefer_free is set).

Lower combined cost.

Higher context window (tie-breaker).

The first candidate wins. The rest become alternatives (used for
fallback).

Example: for Coding, requirements filter out anything under 64k or
without tools. Among the survivors, gpt-4o-mini (cost 0.75) beats
gpt-4o (12.5) and claude-sonnet (18.0).

RouterEngine
Combines both routers.

rust
use mevdan_router::{ModelCandidate, RouterEngine, RouterPolicy, TaskKind};

let engine = RouterEngine::new();

let candidates = vec![
    ModelCandidate::new("gpt-4o", 128_000).with_tools().with_cost(2.5, 10.0),
    ModelCandidate::new("gpt-4o-mini", 128_000).with_tools().with_cost(0.15, 0.6),
];

let decision = engine.route_text(
    "write tests for the parser",
    &candidates,
    RouterPolicy::Automatic,
)?;

println!("{}", decision.summary());
// task=testing → agent=Tester, model=gpt-4o-mini
RoutingDecision contains:

task_kind — the classified task.

agent — the AgentDecision (role + reason).

model — the ModelDecision (chosen name + policy + alternatives).

What the Router is NOT
No ML. No embeddings, no learned weights. Everything is
explicable in a few lines.

No network calls. The router operates on local metadata.

No persisting. Decisions are ephemeral; the runtime records
them (later) via events.

No capability discovery. The candidate list is provided by
the caller (from mevdan-models in the runtime).

Design notes
Rules before statistics. A predictable rule is worth more than
a slightly-better opaque score.

Cost is a first-class signal. In an era of pay-per-token, the
cheapest model that meets requirements is often the right one.

Alternatives are kept. Fallback is a first-class concept.

TaskKind and role are decoupled. The same task can be routed
to different roles if the user wants to override.

Requirements are per task, not per model. Models don't declare
what they're good at; the task declares what it needs.
