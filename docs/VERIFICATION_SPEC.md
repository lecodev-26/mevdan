# MEVDAN — Verification Specification

**"Verification over claims."**

When an agent says "I created the file", that's a **claim**. MEVDAN
must **verify** it. This document describes how.

## Concepts

### Claim

An assertion made by an agent. Claims are never trusted as-is.

```rust
pub struct Claim {
    pub id: ClaimId,
    pub kind: ClaimKind,        // FileCreated, CommandRan, TestPassed, ...
    pub description: String,
    pub status: ClaimStatus,    // Unverified, Verified, Failed, Partial, Unknown
    pub evidence_ids: Vec<EvidenceId>,
    pub artifact_ids: Vec<ArtifactId>,
    pub expected_hash: Option<ContentHash>,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}
```

Evidence

Concrete, checkable proof that something happened.

```rust
pub struct Evidence {
    pub id: EvidenceId,
    pub kind: EvidenceKind,     // FileExists, HashMatch, CommandSuccess, ...
    pub description: String,
    pub artifact_id: Option<ArtifactId>,
    pub hash: Option<ContentHash>,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
```

Artifact

A produced result with a content hash.

```rust
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,     // File, Directory, Text, Json, Binary, CommandOutput
    pub name: String,
    pub hash: ContentHash,      // SHA-256
    pub size_bytes: u64,
    pub path: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
```

Content hashing

All artifacts are hashed with SHA-256. The hash is stored as a
64-char hex string.

```rust
let hash = ContentHash::of_str("hello world");
// b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
```

Comparison uses constant-time equality to avoid timing attacks.

The Verifier trait

A verifier knows how to check a specific claim type.

```rust
pub trait Verifier: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn can_verify(&self, claim: &Claim) -> bool;
    fn verify(&self, claim: &Claim, ctx: &VerificationContext)
        -> VerificationOutcome;
}
```

Built-in verifiers

HashVerifier

For claims with expected_hash. Reads the file, computes the actual
hash, compares. Priority: runs first because it's the most specific.

FileExistsVerifier

For FileCreated / FileModified claims. Checks that the file
exists in the sandbox.

CommandExitVerifier

For CommandRan / TestPassed claims. Runs the command in the
sandbox (via mevdan-tools::ShellTool) and compares the exit code.

Verification engine

```rust
let engine = VerificationEngine::with_defaults();
let ctx = VerificationContext::new("/path/to/project");
let mut claim = Claim::file_created("src/main.rs");
let outcome = engine.verify_and_update(&mut claim, &ctx);
```

The engine iterates verifiers in order. The first one whose
can_verify returns true handles the claim. If none applies, the
outcome is Unknown.

Outcomes

Status Meaning
Verified Evidence confirms the claim.
Failed Evidence contradicts the claim.
Partial Partially verified.
Unknown No verifier can check this.
Unverified Not yet processed.

Terminal statuses (Verified, Failed, Partial, Unknown)
record a verified_at timestamp.

Context and sandbox

VerificationContext carries the sandbox root. All paths are resolved
relative to it. Paths starting with / are always interpreted as
relative to the sandbox, cross-platform.

Flow example

```
Agent: "I created src/main.rs with content 'print(1)'"

1. Agent creates Artifact:
   - kind: Text
   - hash: ContentHash::of_str("print(1)")
   - size_bytes: 8

2. Agent creates Claim:
   - kind: FileCreated
   - expected_hash: <same hash>
   - data: { "path": "src/main.rs" }

3. VerificationEngine::verify_and_update(claim):
   - HashVerifier::can_verify → true (expected_hash present)
   - Reads src/main.rs from sandbox
   - Computes hash of actual file
   - Compares with expected
   - If equal: outcome = Verified, evidence = HashMatch
   - Else: outcome = Failed

4. Claim.status = Verified, verified_at = now
```

Persistence

Claims and evidence are stored as part of WorkState (checkpoints)
and WorkGraph nodes (future). No dedicated tables yet.

What verification is NOT

· It's not "asking the model if it did X". The model's word is not
  evidence.
· It's not "did the operation not crash". Verification checks a
  specific claim against a specific, checkable fact.
· It's not real-time. Verification is a discrete step in the
  runtime.
