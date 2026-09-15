# MEVDAN — Skills Specification

A **skill** is an installable capability. It packages instructions,
tool requirements, and permission requests.

## Directory layout

```

<project>/.mevdan/skills/
coding/
skill.toml
research/
skill.toml
data-analysis/
skill.toml

```

## `skill.toml`

```toml
[skill]
name = "coding"
version = "0.1.0"
description = "Helps with writing and reviewing code"
author = "MEVDAN"   # optional

[skill.instructions]
system_prompt = """
You are a coding assistant inside MEVDAN.
Write clean, tested, documented code.
"""

[skill.permissions]
required_tools = ["filesystem", "shell", "git"]
required_permissions = ["filesystem.write", "shell.execute"]

[skill.metadata]
tags = ["dev", "coding"]
priority = 5
```

Manifest structure

```rust
pub struct SkillManifest {
    pub skill: SkillSection,
}

pub struct SkillSection {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub instructions: Instructions,   // system_prompt: Option<String>
    pub permissions: Permissions,     // required_tools, required_permissions
    pub metadata: serde_json::Value,
}
```

Validation rules:

· name: 1-64 chars, a-z, 0-9, -, _.
· version: semver-like, at least X.Y.
· description: non-empty.

Skill ID

Skills are identified by name + version:

```rust
SkillId { name: "coding", version: "0.1.0" }
// Display: "coding@0.1.0"
```

The registry allows multiple versions of the same skill to coexist.

Loading

```rust
let skills = SkillLoader::load_from_dir(".mevdan/skills")?;
let mut registry = SkillRegistry::new();
registry.register_all(skills)?;
```

load_from_dir:

· Iterates subdirectories.
· Ignores directories without skill.toml.
· Fails if any skill.toml is present but invalid.

load_skill_dir(path) loads one specific skill.

Registry operations

```rust
registry.register(skill)?;              // fails on duplicate
registry.register_or_replace(skill);    // replaces
registry.get(&skill_id) -> Option<&Skill>;
registry.get_by_name("coding") -> Option<&Skill>;  // highest version
registry.remove(&skill_id)?;
registry.list() -> Vec<&Skill>;
registry.names() -> Vec<String>;        // distinct names
```

Isolation

Before loading a skill, validate it against a policy.

Isolation policy

```rust
let policy = IsolationPolicy::new()
    .allow_tool("filesystem")
    .allow_permission("filesystem.read")
    .allow_permission("filesystem.write");
```

Special values: "*" in allowed_tools or allowed_permissions
grants everything. Used for tests, never for production.

Isolation engine

```rust
let iso = SkillIsolation::new(policy);
let report = iso.analyze(&skill);
if !report.is_compatible() {
    println!("{}", report.summary());
    // "coding@0.1.0: missing_tools (missing tools: [shell], ...)"
}
```

Verdicts

Verdict Meaning
Compatible All requirements satisfied.
MissingTools Missing tools.
MissingPermissions Missing permissions.
MissingToolsAndPermissions Both.
Incompatible Generic problem.

Filtering

```rust
let compatible = iso.filter_compatible(skills);
// Only returns skills whose reports are Compatible.
```

Flow example

```
1. User installs a skill: copy folder into .mevdan/skills/coding/

2. Runtime loads skills:
   let skills = SkillLoader::load_from_dir(".mevdan/skills")?;

3. Runtime builds an isolation policy from project config.

4. Runtime filters compatible skills:
   let compatible = iso.filter_compatible(skills);

5. Runtime registers them:
   registry.register_all(compatible)?;

6. When running an agent, the runtime injects the skill's system_prompt
   into the agent context and validates tool availability.
```

Security notes

· Skills are local. No automatic download.
· A skill's system_prompt is injected as instructions, not as
  user input.
· A skill requesting tools it can't have is rejected during isolation.
· A skill cannot escalate permissions at runtime. The policy is fixed
  at load time.

What skills are NOT

· Skills are not agents. They add capabilities to agents.
· Skills are not code plugins. They don't execute arbitrary code.
· Skills are not a marketplace (yet). Installation is manual.
