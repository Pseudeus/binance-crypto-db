## Context

The system has transitioned to a "Combined Stream Actor" architecture to minimize WebSocket connections and improve manageability via a central `Supervisor`. However, a partial refactor left naming inconsistencies (e.g., `FuturesStreamActor` struct with `Actor` trait implemented for `FuturesStreamService`) and stale imports in the `executor` crate, causing compilation failures.

## Goals / Non-Goals

**Goals:**
- Resolve all compilation errors related to `market_data` actors and their usage in the `executor`.
- Unify the naming convention: Structs and their `Actor` trait implementations should consistently use the `Actor` suffix.
- Clean up the `executor/src/main.rs` by removing dead code and correcting actor registration.

**Non-Goals:**
- Adding new data streams or modifying the data processing logic (broadcast channels, parsing).
- Changing the `Supervisor` or `Actor` trait definitions themselves.

## Decisions

### 1. Unified Actor Naming
**Decision:** Rename all ingestion components to use the `Actor` suffix consistently in both struct definitions and `impl Actor` blocks.
- `PublicStreamActor` (Struct) -> `impl Actor for PublicStreamActor`
- `FuturesStreamActor` (Struct) -> `impl Actor for FuturesStreamActor`
- `UserStreamActor` (Struct) -> `impl Actor for UserStreamActor`

**Rationale:** The system uses an actor-based supervision model. The `Actor` suffix clearly distinguishes these components as supervised units of execution rather than passive services.

### 2. Consolidate Imports in `executor/src/main.rs`
**Decision:** Remove imports for `ForceOrderService`, `MarkPriceService`, and `OpenInterestService`. Correct the imports for the consolidated actors from the `market_data` crate.
**Rationale:** These individual services no longer exist as separate files/structs; their functionality is now handled by the combined stream actors.

### 3. Standardize Actor Registration
**Decision:** Use `supervisor.register_actor` with the correct `ActorType` and factory closures that instantiate the newly named actors.
**Rationale:** Ensures the `Supervisor` can correctly restart these actors using their registered factories upon failure.

## Risks / Trade-offs

- **[Risk] Name Collision** → **Mitigation**: Perform a global search (grep) for `*Service` to ensure no active components are accidentally renamed or left broken.
- **[Risk] Import Cycles** → **Mitigation**: Maintain the current crate structure; `executor` depends on `market_data`, which depends on `common`.
- **[Trade-off] Breaking Changes** → Consolidating to `Actor` suffix is a breaking change for any external callers, but since this is a self-contained monorepo/appliance, the impact is limited to internal consistency.
