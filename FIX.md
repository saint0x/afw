# Aria Runtime - Postmortem and Final Fixes

This document outlines the remaining compiler errors in the `aria_runtime` crate and the definitive plan to resolve them. The previous refactoring addressed the core architectural issues related to the `DeepSizeOf` trait, but a cascade of unresolved imports and type mismatches remains.

This is the final push to a clean compile.

## Current Error Landscape

A `cargo check` reveals the following categories of errors across the crate:

1.  **Unresolved Imports (`use` statements missing):** This is the most common error. Types like `TaskComplexity`, `RuntimeStatus`, `ExecutionStatus`, `ActionType`, `StepType`, and various engine traits are not in scope in the files where they are used.

2.  **Incorrect Trait Implementations:** The `ToolRegistry` struct does not correctly implement the `ToolRegistryInterface`, leading to method-not-found errors.

3.  **Type Mismatches with `DeepValue`:** The `execution.rs` file still has logic that incorrectly handles `serde_json::Value` instead of the required `DeepValue` newtype, particularly in pattern matching.

4.  **Incorrect `ErrorCode` enums:** A few incorrect `ErrorCode` variants remain in `errors.rs` and `tool_registry.rs`.

## Definitive Fix Plan

The following is a file-by-file plan to eliminate all remaining errors. Each step will be a complete and authoritative fix for the specified file.

### 1. `crates/aria_runtime/src/errors.rs`

*   **Problem:** The `ErrorCode` enum still contains incorrect variants for timeouts (`ToolTimeout`, `AgentTimeout`, `ContainerTimeout`) and other miscellaneous errors.
*   **Fix:** I will rewrite the file to consolidate all timeout errors into a single `Timeout` variant and fix any other remaining incorrect error codes.

### 2. `crates/aria_runtime/src/engines/mod.rs`

*   **Problem:** This file is missing definitions for several engine interface traits, causing unresolved import errors throughout the crate.
*   **Fix:** I will create this file and define the `Engine` base trait, along with `ExecutionEngineInterface`, `PlanningEngineInterface`, `ReflectionEngineInterface`, `ConversationEngineInterface`, and `ContextManagerInterface`. This will centralize all engine trait definitions.

### 3. `crates/aria_runtime/src/engines/tool_registry.rs`

*   **Problem:** This file has been persistently problematic. It's missing the implementation of `ToolRegistryInterface` and still has incorrect `ErrorCode` variants and type mismatches.
*   **Fix:** I will delete the file to ensure a clean state and recreate it from scratch. The new file will:
    *   Correctly implement the `ToolRegistryInterface`.
    *   Use the correct `ErrorCode` variants.
    *   Properly handle the `DeepValue` newtype for all parameters and results.

### 4. `crates/aria_runtime/src/engines/execution.rs`

*   **Problem:** This file is littered with unresolved imports and incorrect pattern matching on the `DeepValue` newtype.
*   **Fix:** I will rewrite the file, adding every necessary `use` statement for all types, traits, and enums. I will also correct the `match` statements to correctly handle the `DeepValue` newtype wrapper.

### 5. `crates/aria_runtime/src/engines/planning.rs`

*   **Problem:** Missing `use` statements for `StepType` and `TaskComplexity`.
*   **Fix:** I will add the necessary `use` statements at the top of the file.

### 6. `crates/aria_runtime/src/engines/conversation.rs`

*   **Problem:** Missing `use` statements for `ConversationRole`, `ActionType`, `ConversationState`, and `StepType`.
*   **Fix:** I will add the necessary `use` statements at the top of the file.

### 7. `crates/aria_runtime/src/engines/reflection.rs`

*   **Problem:** Missing `use` statements for `SuggestedAction`, `PerformanceLevel`, `QualityLevel`, and `EfficiencyLevel`.
*   **Fix:** I will add the necessary `use` statements at the top of the file.

### 8. `crates/aria_runtime/src/runtime.rs`

*   **Problem:** This file has numerous unresolved imports for types like `TaskComplexity`, `RuntimeStatus`, `ExecutionStatus`, etc.
*   **Fix:** I will add all necessary `use` statements to bring all required types into scope.

### 9. `crates/aria_runtime/src/context.rs` & `crates/aria_runtime/src/tools.rs`

*   **Problem:** These files have minor unresolved import issues.
*   **Fix:** I will add the necessary `use` statements.

Executing this plan will result in a fully compiling `aria_runtime` crate. 