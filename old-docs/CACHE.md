# Symphony SDK: Cache Intelligence & Agent Learning

This document delves into the Cache Intelligence system within the Symphony SDK, explaining its components like pattern matching and context trees, and how these features, particularly when combined with mechanisms like `recordToolExecution`, can contribute to agents' ability to optimize their behavior and "learn" from past interactions.

## Overview of Cache Intelligence

The Cache Intelligence system in Symphony aims to go beyond simple key-value caching. Its goal is to optimize SDK performance, reduce redundant LLM calls or tool executions, and potentially guide agents towards more efficient task completion strategies based on historical data.

This is managed by a `ContextIntelligenceAPI` and interacted with via `symphony.cache.*` methods.

Key features:
- **Basic Caching**: Standard get/set/delete operations for caching arbitrary data, LLM responses, or tool results.
- **Pattern Matching**: Identifying recurring patterns in task queries or LLM interactions to potentially offer shortcuts or pre-computed responses/tool calls.
- **Context Trees**: Building hierarchical representations of context to better understand the current state and retrieve relevant information or predict next steps.
- **Fast Path Execution**: Using high-confidence pattern matches or context tree evaluations to bypass full LLM reasoning for known scenarios.

## How Agents Can "Self-Learn" (Optimize Behavior)

While agents in the Symphony SDK don't "self-learn" in the sense of retraining their underlying LLM weights, the Cache Intelligence system provides mechanisms for them to adapt and optimize their behavior over time based on recorded interactions. This is primarily achieved through:

1.  **`symphony.cache.recordToolExecution(...)`**: This is a crucial feedback loop.
    - **Functionality**: After a tool is executed (either by an agent or directly), this method can be called to log the details of that execution: the `sessionId`, `toolName`, input `params`, the `result`, `success` status, `executionTimeMs`, and optionally, a `patternId` if the execution was part of a recognized pattern.
    - **Learning Implication**: By recording these executions, the Cache Intelligence system gathers data on:
        - Which tools are called for which types of tasks/parameters.
        - The success/failure rates of tools.
        - The performance (execution time) of tools.
        - The parameters that lead to successful outcomes.

2.  **Pattern Matching & `symphony.cache.getIntelligence(...)`**:
    - **Functionality**: When a new task or query comes in (e.g., at the start of an agent's reasoning process or before deciding on a tool), `symphony.cache.getIntelligence(query, options)` can be called.
    - **Learning Implication**: This method analyzes the input `query` against its stored patterns and context data (which includes information from `recordToolExecution`).
        - If a high-confidence **pattern match** is found (e.g., a similar query previously led to a specific successful tool call), `getIntelligence` might return a `recommendation: 'fast_path'` along with the `toolCall` (name and parameters) and `reasoning` from the matched pattern.
        - An agent receiving this could then decide to directly execute the recommended tool call, bypassing an LLM call for tool selection, thus appearing to have "learned" the most efficient path for that recognized query pattern.
        - The `fastPathThreshold` option in `SymphonyConfig.cache` likely controls the confidence level required for such a fast path.

3.  **Context Trees & `symphony.cache.getIntelligence(...)`**:
    - **Functionality**: Context trees are likely more complex structures representing conversational context, user history, or task state.
    - **Learning Implication**: As interactions are recorded (implicitly through `recordToolExecution` or other context-building mechanisms), these trees become richer.
        - `getIntelligence` might leverage these trees to provide an `enhanced_context` recommendation or suggest a particular course of action even if an exact pattern isn't matched.
        - This helps the agent make more informed decisions by having a deeper understanding of the ongoing interaction, built from past data points.

4.  **Analytics & Optimization (`symphony.cache.getPatternAnalytics()`, etc.)**:
    - **Functionality**: Methods like `getPatternAnalytics`, `getContextAnalytics`, and `getSessionIntelligence` provide insights into how the cache and its learning mechanisms are performing.
    - **Learning Implication (for System/Developer)**: While not direct agent self-learning, these analytics allow developers to understand:
        - Which patterns are most effective.
        - The hit/miss rates of the cache.
        - Average confidence scores.
        - This data can be used to refine prompts, tool designs, or even manually curate or prune cache patterns and context data to improve the system's overall learning and efficiency.

## Example Flow of Agent Optimization:

1.  **Initial Task**: Agent receives a novel task. `symphony.cache.getIntelligence()` finds no high-confidence match.
2.  **LLM Reasoning**: Agent consults LLM to decide on a tool (e.g., `toolA` with `paramsX`).
3.  **Tool Execution**: `toolA` is executed.
4.  **Feedback Loop**: `symphony.cache.recordToolExecution(...)` is called with details of the `toolA` call, its success, and potentially a new `patternId` is associated if the system deems this interaction common enough.
5.  **Subsequent Similar Task**: Agent receives a task very similar to the initial one.
6.  **Cache Hit**: `symphony.cache.getIntelligence()` now finds a high-confidence pattern match from the previous recording.
7.  **Fast Path**: It recommends directly calling `toolA` with `paramsX` (or slightly adapted params).
8.  **Optimized Execution**: The agent might bypass the LLM call for tool selection and directly execute `toolA`, appearing to have learned the optimal path from the previous experience.

## Limitations and Considerations:

- **Explicit Recording**: The "learning" is heavily dependent on consistent and accurate use of `symphony.cache.recordToolExecution` and other data-feeding mechanisms.
- **Pattern Quality**: The effectiveness of pattern matching depends on the quality and generality of the patterns being learned or defined.
- **Context Scope**: Context trees and session intelligence are most effective if `sessionId` is managed correctly to delineate different interactions.
- **Not True Model Retraining**: This system optimizes behavior by reusing successful past strategies or learned associations; it does not retrain the underlying LLM.

In summary, Symphony's Cache Intelligence, particularly through the `recordToolExecution` feedback loop and the `getIntelligence` recommendation engine, provides a powerful mechanism for agents to optimize their responses and tool usage strategies over time by learning from historical interactions. This leads to improved efficiency, reduced LLM costs, and potentially more consistent agent behavior for recognized tasks.

--- 