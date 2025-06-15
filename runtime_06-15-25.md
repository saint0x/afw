# Jockey Image

Generated: 06-15-2025 at 11:27:44

## Repository Structure

```
runtime
│   ├── engines
│       └── PlanningEngine.ts
│   ├── SymphonyRuntime.ts
│   ├── context
│   │   ├── RuntimeContextManager.ts
│       └── ExecutionState.ts
│   ├── context.ts
│   ├── symphony.ts
│   ├── conversation
│       └── ConversationManager.ts
    └── types.ts
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/SymphonyRuntime.ts

```ts
 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/context/ExecutionState.ts

```ts
import { AgentConfig, ToolResult } from '../../types/sdk';
import { ExecutionPlan, Insight, RuntimeError, ConversationJSON } from '../types';

/**
 * Represents a clean, serializable snapshot of the entire execution state.
 * This can be used for logging, debugging, and for the Context API to learn from.
 */
export interface ExecutionState {
  readonly sessionId: string;
  readonly agentConfig: AgentConfig;
  readonly plan: ExecutionPlan | null;
  readonly history: ReadonlyArray<ExecutionStateStep>;
  readonly insights: ReadonlyArray<Insight>;
  readonly errors: ReadonlyArray<RuntimeError>;
  readonly conversation: ConversationJSON;
  readonly workingMemory: Record<string, any>;
  readonly status: 'running' | 'succeeded' | 'failed' | 'aborted';
}

/**
 * Represents a single, completed step within the execution history.
 * It is an immutable record of what happened.
 */
export interface ExecutionStateStep {
  readonly stepId: string;
  readonly description: string;
  readonly toolUsed: string | null;
  readonly input: Record<string, any>;
  readonly output: ToolResult;
  readonly success: boolean;
  readonly reflection?: string; // Reasoning from the reflection engine
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/context/RuntimeContextManager.ts

```ts
import { IContextAPI } from '../../api/IContextAPI';
import { Logger } from '../../utils/logger';
import { RuntimeContext } from '../context';
import { ExecutionStep, Insight, Reflection, InsightType, ExecutionPlan } from '../types';
import { ExecutionState } from './ExecutionState';
import { v4 as uuidv4 } from 'uuid';

/**
 * Manages the runtime context, including learning, analytics, and memory optimization.
 * This class acts as the "brain" for context, using the IContextAPI to perform
 * intelligent operations based on the execution state.
 */
export class RuntimeContextManager {
  private logger: Logger;

  constructor(
    private context: RuntimeContext,
    private contextApi: IContextAPI
  ) {
    this.logger = new Logger('RuntimeContextManager');
    this.logger.info('RuntimeContextManager', 'created', { sessionId: this.context.sessionId });
  }

  /**
   * Records a completed execution step and triggers learning.
   * @param step The execution step that was just completed.
   */
  public async recordStep(step: ExecutionStep): Promise<void> {
    this.context.addExecutionStep(step);
    this.logger.debug('RuntimeContextManager', `Step recorded: ${step.description}`, { stepId: step.stepId, success: step.success });
    
    // Asynchronously learn from the execution without blocking the main flow
    this.learnFromExecution().catch(error => {
      this.logger.warn('RuntimeContextManager', 'Failed to learn from execution', { error, stepId: step.stepId });
    });
  }

  /**
   * Records a reflection and incorporates its insights.
   * @param reflection The reflection generated after a step.
   */
  public recordReflection(reflection: Reflection): void {
    this.context.addReflection(reflection);
    
    const insight: Insight = {
      id: reflection.id,
      type: 'strategy_assessment' as InsightType,
      description: `Reflection on step "${reflection.stepId}": ${reflection.reasoning}`,
      confidence: reflection.confidence,
      source: 'ReflectionEngine',
      timestamp: Date.now(),
      actionable: reflection.suggestedAction !== 'continue',
    };

    this.context.addInsight(insight);
    this.logger.debug('RuntimeContextManager', `Reflection recorded for step ${reflection.stepId}`);
  }

  /**
   * Generates a comprehensive, immutable snapshot of the current execution state.
   * @returns The current ExecutionState.
   */
  public getExecutionState(): ExecutionState {
    return this.context.toExecutionState();
  }

  /**
   * Sets the execution plan on the underlying context.
   * @param plan The execution plan to set.
   */
  public setPlan(plan: ExecutionPlan): void {
    this.context.setExecutionPlan(plan);
    this.logger.info('RuntimeContextManager', `Execution plan set with ${plan.steps.length} steps.`, { planId: plan.id });
  }

  /**
   * Updates the overall status of the execution.
   * @param status The new status.
   */
  public updateStatus(status: 'running' | 'succeeded' | 'failed' | 'aborted'): void {
    this.context.status = status;
    this.logger.info('RuntimeContextManager', `Execution status updated to: ${status}`);
  }

  /**
   * Generates a final summary insight for the entire execution.
   */
  public async generateExecutionInsights(): Promise<void> {
    this.logger.info('RuntimeContextManager', 'Generating final execution insights...');
    try {
        const { success, result } = await this.contextApi.useMagic('get_insights', {
            sessionId: this.context.sessionId,
            includeFailures: true,
        });

        if (success && result) {
            const summaryDescription = `Execution Summary: ${result.totalExecutions} total operations, ${result.successRate}% success rate. Average execution time: ${result.avgExecutionTime}ms.`;
            const insight: Insight = {
                id: uuidv4(),
                type: 'performance_optimization',
                description: summaryDescription,
                confidence: 0.9,
                source: 'ContextAPI.get_insights',
                timestamp: Date.now(),
                actionable: false,
                metadata: result,
            };
            this.context.addInsight(insight);
            this.logger.info('RuntimeContextManager', 'Successfully generated execution insights.');
        } else {
            this.logger.warn('RuntimeContextManager', 'Failed to generate execution insights', { error: result?.error });
        }
    } catch (error) {
        this.logger.error('RuntimeContextManager', 'An exception occurred during insight generation', { error });
    }
  }

  /**
   * Calls the Context API to learn from the latest execution details.
   */
  private async learnFromExecution(): Promise<void> {
    const lastStep = this.context.executionHistory[this.context.executionHistory.length - 1];
    if (!lastStep || !lastStep.toolUsed) {
        return; // Nothing to learn from
    }

    try {
        const { success, result } = await this.contextApi.useMagic('learn_from_execution', {
            toolName: lastStep.toolUsed,
            success: lastStep.success,
            executionTime: lastStep.duration,
            context: {
                task: this.context.currentPlan?.taskDescription || 'N/A',
                stepDescription: lastStep.description,
                agent: this.context.agentConfig.name,
            },
            errorDetails: lastStep.error,
        });

        if (success) {
            this.logger.info('RuntimeContextManager', `Learned from execution of tool: ${lastStep.toolUsed}`);
            if (result?.insights) {
                result.insights.forEach((insight: Insight) => this.context.addInsight(insight));
            }
        } else {
            this.logger.warn('RuntimeContextManager', `Context API failed to learn from execution`, { tool: lastStep.toolUsed, error: result?.error });
        }
    } catch (error) {
        this.logger.error('RuntimeContextManager', 'An exception occurred during learnFromExecution', { error });
    }
  }

  /**
   * Performs intelligent context pruning and memory cleanup.
   */
  public async performMaintenance(): Promise<void> {
    this.logger.info('RuntimeContextManager', 'Performing context maintenance...');
    try {
        const { success, result } = await this.contextApi.useMagic('prune_context', {
            sessionId: this.context.sessionId,
        });

        if (success) {
            this.logger.info('RuntimeContextManager', 'Intelligent context pruning successful', result);
        } else {
            this.logger.warn('RuntimeContextManager', 'Intelligent context pruning failed', result);
        }
    } catch (error) {
        this.logger.error('RuntimeContextManager', 'An exception occurred during context pruning', { error });
    }
  }
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/context.ts

```ts
import { v4 as uuidv4 } from 'uuid';
import { AgentConfig } from '../types/sdk';
import {
  RuntimeContext as IRuntimeContext,
  ExecutionPlan,
  ExecutionStep,
  Insight,
  RuntimeError,
  PlannedStep,
  Reflection
} from './types';
import { ExecutionState } from './context/ExecutionState';
import { ConversationJSON, RuntimeSnapshot } from './types';

/**
 * Manages the state of a single execution flow. It is a state container
 * with methods to update and query that state. The intelligent logic
 * for learning and analytics has been moved to the RuntimeContextManager.
 */
export class RuntimeContext implements IRuntimeContext {
  public readonly sessionId: string;
  public readonly agentConfig: AgentConfig;
  public readonly createdAt: number;
  public conversation: ConversationJSON | null = null;
  public status: 'running' | 'succeeded' | 'failed' | 'aborted' = 'running';

  public currentPlan?: ExecutionPlan;
  public executionHistory: ExecutionStep[] = [];
  public workingMemory: Map<string, any> = new Map();
  public insights: Insight[] = [];
  public errorHistory: RuntimeError[] = [];
  public currentStep: number = 0;
  public totalSteps: number = 0;
  public remainingSteps?: PlannedStep[];
  
  private _reflections: Reflection[] = [];
  private _memorySize: number = 0;
  private readonly _maxMemorySize: number = 50 * 1024 * 1024; // 50MB limit

  constructor(agentConfig: AgentConfig, sessionId?: string) {
    this.sessionId = sessionId || uuidv4();
    this.agentConfig = agentConfig;
    this.createdAt = Date.now();
  }

  setExecutionPlan(plan: ExecutionPlan): void {
    this.currentPlan = plan;
    this.totalSteps = plan.steps.length;
    this.remainingSteps = [...plan.steps];
    this.currentStep = 0;
  }

  addExecutionStep(stepData: Omit<ExecutionStep, 'stepId' | 'startTime' | 'endTime' | 'duration'>): ExecutionStep {
    const startTime = Date.now();
    const endTime = Date.now();
    const duration = endTime - startTime;

    const step: ExecutionStep = {
      ...stepData,
      stepId: uuidv4(),
      startTime,
      endTime,
      duration,
    };
    
    this.executionHistory.push(step);
    this.currentStep = this.executionHistory.length;
    
    if (this.remainingSteps && this.remainingSteps.length > 0) {
      this.remainingSteps = this.remainingSteps.slice(1);
    }

    this.setMemory(`step_${step.stepId}`, {
      result: step.result,
      success: step.success,
      duration: step.duration,
      timestamp: step.endTime
    });

    if (step.reflection) {
      this._reflections.push(step.reflection);
    }
    
    return step;
  }

  addInsight(insight: Insight): void {
    const isDuplicate = this.insights.some(existing => 
      existing.type === insight.type && existing.description === insight.description
    );

    if (!isDuplicate) {
      this.insights.push(insight);
      if (this.insights.length > 100) {
        this.insights.shift();
      }
    }
  }

  addReflection(reflection: Reflection): void {
    this._reflections.push(reflection);
  }

  getReflections(): Reflection[] {
    return [...this._reflections];
  }
  
  public toSnapshot(): RuntimeSnapshot {
    return {
      sessionId: this.sessionId,
      currentStep: this.currentStep,
      totalSteps: this.totalSteps,
      executionHistory: this.executionHistory,
      insights: this.insights,
      timestamp: Date.now()
    }
  }

  public updateExecutionPlan(plan: ExecutionPlan): void {
    if (!this.currentPlan) {
      this.setExecutionPlan(plan);
      return;
    }

    const previousStep = this.currentStep;
    this.currentPlan = plan;
    this.totalSteps = plan.steps.length;
    
    // Adjust remaining steps based on current progress
    this.remainingSteps = plan.steps.slice(previousStep);
  }

  toExecutionState(): ExecutionState {
    const memoryRecord: Record<string, any> = {};
    this.workingMemory.forEach((value, key) => {
        memoryRecord[key] = value;
    });

    return {
      sessionId: this.sessionId,
      agentConfig: this.agentConfig,
      plan: this.currentPlan || null,
      history: this.executionHistory.map(step => ({
        stepId: step.stepId,
        description: step.description,
        toolUsed: step.toolUsed || null,
        input: step.parameters || {},
        output: step.result,
        success: step.success,
        reflection: this._reflections.find(r => r.stepId === step.stepId)?.reasoning,
      })),
      insights: [...this.insights],
      errors: [...this.errorHistory],
      conversation: this.conversation || { id: '', originalTask: '', turns: [], finalResponse: '', reasoningChain: [], duration: 0, state: 'error' },
      workingMemory: memoryRecord,
      status: this.status,
    };
  }

  setMemory(key: string, value: any): void {
    const serialized = JSON.stringify(value);
    const size = new Blob([serialized]).size;
    
    if (this.workingMemory.has(key)) {
      const oldSize = new Blob([JSON.stringify(this.workingMemory.get(key))]).size;
      this._memorySize -= oldSize;
    }

    this.workingMemory.set(key, value);
    this._memorySize += size;
  }

  getMemory<T = any>(key: string): T | undefined {
    return this.workingMemory.get(key) as T;
  }

  addError(error: Omit<RuntimeError, 'id' | 'timestamp'>): void {
    const runtimeError: RuntimeError = { id: uuidv4(), timestamp: Date.now(), ...error };
    this.errorHistory.push(runtimeError);
    if (this.errorHistory.length > 50) {
      this.errorHistory.shift();
    }
  }

  getMemoryUsage(): { currentSize: number; maxSize: number; utilizationPercent: number; itemCount: number; } {
    return {
      currentSize: this._memorySize,
      maxSize: this._maxMemorySize,
      utilizationPercent: (this._memorySize / this._maxMemorySize) * 100,
      itemCount: this.workingMemory.size
    };
  }

  cleanupMemory(): void {
    const utilizationPercent = (this._memorySize / this._maxMemorySize) * 100;
    
    if (utilizationPercent > 80) {
      const stepKeys = Array.from(this.workingMemory.keys()).filter(k => k.startsWith('step_'));
      const sortedStepKeys = stepKeys.sort((a, b) => {
        const stepA = this.getMemory(a);
        const stepB = this.getMemory(b);
        return (stepA?.timestamp || 0) - (stepB?.timestamp || 0);
      });

      const toRemoveCount = Math.ceil(sortedStepKeys.length * 0.25);
      for (let i = 0; i < toRemoveCount; i++) {
        this.clearMemory(sortedStepKeys[i]);
      }
    }
  }
  
  private clearMemory(key: string): boolean {
    if (this.workingMemory.has(key)) {
      const value = this.workingMemory.get(key);
      const size = new Blob([JSON.stringify(value)]).size;
      this._memorySize -= size;
      this.workingMemory.delete(key);
      return true;
    }
    return false;
  }
}

export function createRuntimeContext(
  agentConfig: AgentConfig, 
  sessionId?: string
): RuntimeContext {
  return new RuntimeContext(agentConfig, sessionId);
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/types.ts

```ts
import { AgentConfig, ToolResult } from '../types/sdk';
import { IContextAPI } from '../api/IContextAPI';
import { LLMHandler } from '../llm/handler';
import { ToolRegistry } from '../tools/standard/registry';
import { Logger } from '../utils/logger';
import { ExecutionState } from './context/ExecutionState';

// ==========================================
// CORE RUNTIME TYPES
// ==========================================

export interface RuntimeResult {
  success: boolean;
  mode: RuntimeExecutionMode;
  conversation?: ConversationJSON;
  executionDetails: ExecutionDetails;
  plan?: ExecutionPlan;
  reflections?: Reflection[];
  error?: string;
  metrics: RuntimeMetrics;
}

export type RuntimeExecutionMode = 
  | 'legacy'
  | 'legacy_with_conversation' 
  | 'enhanced_planning'
  | 'adaptive_reflection';

export interface RuntimeMetrics {
  totalDuration: number;
  startTime: number;
  endTime: number;
  stepCount: number;
  toolCalls: number;
  tokenUsage?: {
    prompt: number;
    completion: number;
    total: number;
  };
  reflectionCount: number;
  adaptationCount: number;
}

// ==========================================
// EXECUTION CONTEXT & STATE
// ==========================================

export interface RuntimeContext {
  readonly sessionId: string;
  readonly agentConfig: AgentConfig;
  readonly createdAt: number;
  
  // Execution state
  currentPlan?: ExecutionPlan;
  executionHistory: ExecutionStep[];
  workingMemory: Map<string, any>;
  insights: Insight[];
  errorHistory: RuntimeError[];
  
  // Context management
  currentStep: number;
  totalSteps: number;
  remainingSteps?: PlannedStep[];
  
  // Methods
  setExecutionPlan(plan: ExecutionPlan): void;
  updateExecutionPlan(plan: ExecutionPlan): void;
  addExecutionStep(step: Omit<ExecutionStep, 'stepId' | 'startTime' | 'endTime' | 'duration'>): ExecutionStep;
  addInsight(insight: Insight): void;
  addReflection(reflection: Reflection): void;
  getReflections(): Reflection[];
  toSnapshot(): RuntimeSnapshot;
}

export interface RuntimeSnapshot {
  sessionId: string;
  currentStep: number;
  totalSteps: number;
  executionHistory: ExecutionStep[];
  insights: Insight[];
  timestamp: number;
}

// ==========================================
// PLANNING & EXECUTION
// ==========================================

export interface ExecutionPlan {
  readonly id: string;
  readonly taskDescription: string;
  readonly steps: PlannedStep[];
  readonly confidence: number;
}

export interface PlannedStep {
  readonly id: string;
  readonly description: string;
  readonly toolName: string;
  readonly parameters: Record<string, any>;
  readonly successCriteria: string;
}

export interface ExecutionStep {
  readonly stepId: string;
  readonly description: string;
  readonly startTime: number;
  readonly endTime: number;
  readonly duration: number;
  readonly success: boolean;
  readonly toolUsed?: string;
  readonly parameters?: Record<string, any>;
  readonly result?: any;
  readonly error?: string;
  readonly reflection?: Reflection;
  
  summary: string;
}

export interface TaskAnalysis {
  complexity: TaskComplexity;
  requiresPlanning: boolean;
  reasoning: string;
}

export type TaskComplexity = 'simple' | 'multi_step' | 'complex';

// ==========================================
// ACTION & INTENT SYSTEM
// ==========================================

export interface ActionIntent {
  readonly id: string;
  readonly description: string;
  readonly type: ActionType;
  readonly priority: number;
  readonly startTime: number;
  readonly context: Record<string, any>;
}

export type ActionType = 
  | 'search' 
  | 'analyze' 
  | 'create' 
  | 'transform' 
  | 'communicate'
  | 'validate'
  | 'other';

export interface ActionResult {
  readonly id: string;
  readonly actionIntent: ActionIntent;
  readonly success: boolean;
  readonly duration: number;
  readonly toolUsed?: string;
  readonly parameters?: Record<string, any>;
  readonly data?: any;
  readonly error?: string;
  readonly confidence: number;
  readonly startTime: number;
  readonly endTime: number;
}

// ==========================================
// CONVERSATION SYSTEM
// ==========================================

export interface Conversation {
  readonly id: string;
  readonly originalTask: string;
  readonly sessionId: string;
  readonly createdAt: number;
  
  turns: ConversationTurn[];
  currentState: ConversationState;
  finalResponse?: string;
  
  addTurn(role: 'user' | 'assistant', content: string, metadata?: ConversationMetadata): ConversationTurn;
  getRecentTurns(count: number): ConversationTurn[];
  getFinalResponse(): string | undefined;
  getReasoningChain(): string[];
  getFlowSummary(): string;
  getCurrentState(): ConversationState;
  toJSON(): ConversationJSON;
}

export interface ConversationTurn {
  readonly id: string;
  readonly role: 'user' | 'assistant';
  readonly content: string;
  readonly timestamp: number;
  readonly metadata?: ConversationMetadata;
}

export interface ConversationMetadata {
  stepId?: string;
  toolUsed?: string;
  actionType?: ActionType;
  confidence?: number;
  reflection?: boolean;
}

export type ConversationState = 
  | 'initiated'
  | 'working' 
  | 'reflecting'
  | 'adapting'
  | 'concluding'
  | 'completed'
  | 'error';

export interface ConversationJSON {
  id: string;
  originalTask: string;
  turns: ConversationTurn[];
  finalResponse: string;
  reasoningChain: string[];
  duration: number;
  state: ConversationState;
}

// ==========================================
// REFLECTION & ADAPTATION
// ==========================================

export interface Reflection {
  readonly id: string;
  readonly stepId: string;
  readonly assessment: ReflectionAssessment;
  readonly suggestedAction: 'continue' | 'retry' | 'abort' | 'modify_plan';
  readonly reasoning: string;
  readonly confidence: number;
  readonly timestamp: number;
}

export interface ReflectionAssessment {
  performance: 'excellent' | 'good' | 'acceptable' | 'poor';
  quality: 'optimal' | 'good' | 'suboptimal' | 'wrong';
  suggestedImprovements?: string[];
}

export interface ExecutionAdaptation {
  readonly type: AdaptationType;
  readonly specificChanges: string[];
  readonly planModifications: string[];
  readonly conversationAdjustments: string[];
  readonly confidenceBoostActions: string[];
  readonly estimatedImpact: 'positive' | 'neutral' | 'risky';
  readonly requiresReplanning: boolean;
}

export type AdaptationType = 
  | 'continue'
  | 'modify_approach' 
  | 'change_tools' 
  | 'replanning_needed';

export interface Insight {
  readonly id: string;
  readonly type: InsightType;
  readonly description: string;
  readonly confidence: number;
  readonly source: string;
  readonly timestamp: number;
  readonly actionable: boolean;
  readonly metadata?: Record<string, any>;
}

export type InsightType = 
  | 'pattern_recognition'
  | 'performance_optimization'
  | 'error_prevention'
  | 'strategy_improvement'
  | 'context_learning';

// ==========================================
// ERROR HANDLING
// ==========================================

export interface RuntimeError {
  readonly id: string;
  readonly type: RuntimeErrorType;
  readonly message: string;
  readonly stepId?: string;
  readonly toolName?: string;
  readonly timestamp: number;
  readonly context: Record<string, any>;
  readonly recoverable: boolean;
}

export type RuntimeErrorType = 
  | 'tool_execution'
  | 'planning_failure'
  | 'conversation_error'
  | 'reflection_error'
  | 'context_error'
  | 'system_error';

export interface FallbackStrategy {
  readonly id: string;
  readonly description: string;
  readonly triggerCondition: string;
  readonly actions: string[];
  readonly confidence: number;
}

// ==========================================
// ENGINE INTERFACES
// ==========================================

export interface RuntimeEngine {
  initialize(): Promise<void>;
  getDependencies(): string[];
  getState(): string;
  healthCheck(): Promise<boolean>;
}

export interface ExecutionEngineInterface extends RuntimeEngine {
  execute(task: string, agentConfig: AgentConfig, state: ExecutionState): Promise<ToolResult>;
}

export interface ConversationEngineInterface extends RuntimeEngine {
  initiate(task: string, context: { sessionId: string }): Promise<Conversation>;
  run(conversation: Conversation, context: { sessionId: string }): Promise<Conversation>;
  conclude(conversation: Conversation, context: { sessionId:string }): Promise<Conversation>;
}

export interface PlanningEngineInterface extends RuntimeEngine {
  analyzeTask(task: string, state: ExecutionState): Promise<TaskAnalysis>;
  createExecutionPlan(task: string, agentConfig: AgentConfig, state: ExecutionState): Promise<ExecutionPlan>;
}

export interface ReflectionEngineInterface extends RuntimeEngine {
  reflect(stepResult: ExecutionStep, state: ExecutionState, conversation: Conversation): Promise<Reflection>;
}

// ==========================================
// RUNTIME CONFIGURATION
// ==========================================

export interface RuntimeConfiguration {
  enhancedRuntime: boolean;
  planningThreshold: TaskComplexity;
  reflectionEnabled: boolean;
  maxStepsPerPlan: number;
  timeoutMs: number;
  retryAttempts: number;
  debugMode: boolean;
}

export interface RuntimeDependencies {
  toolRegistry: ToolRegistry;
  contextAPI: IContextAPI;
  llmHandler: LLMHandler;
  logger: Logger;
}

// ==========================================
// UTILITY TYPES
// ==========================================

export interface ExecutionDetails {
  mode: RuntimeExecutionMode;
  stepResults: ExecutionStep[];
  participatingAgents?: string[];
  totalSteps: number;
  completedSteps: number;
  failedSteps: number;
  adaptations: ExecutionAdaptation[];
  readonly insights?: readonly Insight[];
}

export type RuntimeStatus = 'initializing' | 'ready' | 'executing' | 'error' | 'shutdown';

// ==========================================
// FACTORY TYPES
// ==========================================

export interface RuntimeFactory {
  createRuntime(dependencies: RuntimeDependencies, config?: Partial<RuntimeConfiguration>): Promise<SymphonyRuntimeInterface>;
  createContext(agentConfig: AgentConfig, sessionId?: string): Promise<RuntimeContext>;
}

export interface SymphonyRuntimeInterface {
  initialize(): Promise<void>;
  execute(task: string, agentConfig: AgentConfig): Promise<RuntimeResult>;
  shutdown(): Promise<void>;
  getStatus(): RuntimeStatus;
  getMetrics(): RuntimeMetrics;
  healthCheck(): Promise<boolean>;
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/engines/PlanningEngine.ts

```ts
import { v4 as uuidv4 } from 'uuid';
import { PlanningEngineInterface, RuntimeDependencies, ExecutionPlan, TaskAnalysis, TaskComplexity, PlannedStep } from "../types";
import { ToolResult, AgentConfig } from "../../types/sdk";
import { ExecutionState } from '../context/ExecutionState';
import { ToolError, ErrorCode } from '../../errors/index';

/**
 * The PlanningEngine is responsible for analyzing tasks and creating execution plans.
 * It leverages existing tools to bootstrap its planning capabilities.
 */
export class PlanningEngine implements PlanningEngineInterface {
    private dependencies: RuntimeDependencies;

    constructor(dependencies: RuntimeDependencies) {
        this.dependencies = dependencies;
    }

    async initialize(): Promise<void> {
        this.dependencies.logger.info('PlanningEngine', 'PlanningEngine initialized');
    }

    getDependencies(): string[] {
        return ['toolRegistry', 'llmHandler', 'logger'];
    }

    getState(): string {
        return 'ready';
    }

    async healthCheck(): Promise<boolean> {
        // The health of the planning engine depends on the createPlanTool being available.
        const createPlanTool = this.dependencies.toolRegistry.getToolInfo('createPlanTool');
        return !!createPlanTool;
    }

    /**
     * Analyzes the complexity of a task to determine if it requires multi-step planning.
     * @param task The user's task description.
     * @param _state The current execution state.
     * @returns A TaskAnalysis object.
     */
    async analyzeTask(task: string, _state: ExecutionState): Promise<TaskAnalysis> {
        const keywords = ['then', 'and then', 'after that', 'first', 'second', 'finally', 'create a plan'];
        const taskLower = task.toLowerCase();

        const requiresPlanning = keywords.some(kw => taskLower.includes(kw)) || task.length > 200;
        const complexity: TaskComplexity = requiresPlanning ? 'multi_step' : 'simple';

        const reasoning = requiresPlanning 
            ? `Task contains keywords or is long, suggesting multiple steps are needed.`
            : `Task appears simple and suitable for single-shot execution.`;

        return {
            complexity,
            requiresPlanning,
            reasoning
        };
    }

    /**
     * Creates a detailed execution plan for a given task by wrapping the 'createPlanTool'.
     * @param task The user's task description.
     * @param agentConfig The configuration of the agent.
     * @param state The current execution state.
     * @returns An ExecutionPlan object.
     */
    async createExecutionPlan(task: string, agentConfig: AgentConfig, state: ExecutionState): Promise<ExecutionPlan> {
        this.dependencies.logger.info('PlanningEngine', `Creating execution plan for task: ${task}`);

        try {
            // Use the context-aware magic of the ContextAPI
            const planSuggestionResult = await this.dependencies.contextAPI.useMagic('suggest_tools', {
                task: task,
                context: {
                    agentName: agentConfig.name,
                    availableTools: agentConfig.tools,
                    sessionId: state.sessionId
                }
            });

            // For now, we still use the createPlanTool, but in the future,
            // the suggest_tools magic could directly return a structured plan.
            const planToolResult: ToolResult = await this.dependencies.toolRegistry.executeTool('createPlanTool', {
                objective: task,
                context: {
                    agentName: agentConfig.name,
                    availableTools: agentConfig.tools,
                    suggestions: planSuggestionResult.result?.suggestions
                }
            });

            if (!planToolResult.success || !planToolResult.result || !planToolResult.result.plan) {
                throw new ToolError(
                    'createPlanTool',
                    ErrorCode.TOOL_EXECUTION_FAILED,
                    `createPlanTool failed or returned an invalid plan: ${planToolResult.error}`,
                    { planToolResult, task },
                    { component: 'PlanningEngine', operation: 'createPlan' }
                );
            }

            // For now, we'll treat the raw LLM output as the plan steps.
            // In the future, we would parse this into a structured list of PlannedStep.
            const rawPlan = planToolResult.result.plan.generatedPlan;
            const steps: PlannedStep[] = this.parseRawPlanToSteps(rawPlan);

            const plan: ExecutionPlan = {
                id: uuidv4(),
                taskDescription: task,
                steps: steps,
                confidence: 0.85 // Confidence in the generated plan
            };

            return plan;

        } catch (error) {
            this.dependencies.logger.error('PlanningEngine', `Failed to create execution plan`, { error });
            throw error;
        }
    }

    /**
     * A simple parser to convert raw text plan into structured steps.
     * This will be improved later with more robust LLM-guided JSON generation.
     */
    private parseRawPlanToSteps(rawPlan: string): PlannedStep[] {
        if (!rawPlan) {
            this.dependencies.logger.warn('PlanningEngine', 'Received an empty or null raw plan string.');
            return [];
        }

        this.dependencies.logger.info('PlanningEngine', 'Attempting to parse raw plan...', { rawPlan });

        try {
            const parsedJson = JSON.parse(rawPlan);
            let stepsArray: any[] | null = null;

            // Log the keys of the parsed object to understand its structure
            this.dependencies.logger.info('PlanningEngine', 'Parsed raw plan JSON object.', { keys: Object.keys(parsedJson) });

            // More robustly find the array of steps
            if (Array.isArray(parsedJson)) {
                stepsArray = parsedJson;
            } else if (parsedJson.plan && Array.isArray(parsedJson.plan)) {
                stepsArray = parsedJson.plan;
            } else if (parsedJson.steps && Array.isArray(parsedJson.steps)) {
                stepsArray = parsedJson.steps;
            } else {
                // Look for any key that holds an array
                const arrayKey = Object.keys(parsedJson).find(key => Array.isArray(parsedJson[key]));
                if (arrayKey) {
                    this.dependencies.logger.info('PlanningEngine', `Found plan array under unexpected key: '${arrayKey}'`);
                    stepsArray = parsedJson[arrayKey];
                }
            }

            if (!stepsArray) {
                this.dependencies.logger.warn('PlanningEngine', 'Could not find a valid step array in the parsed JSON.', { parsedJson });
                return [];
            }

            this.dependencies.logger.info('PlanningEngine', `Successfully extracted ${stepsArray.length} steps from the plan.`);

            return stepsArray.map((step, index) => {
                if (!step || typeof step !== 'object') {
                    this.dependencies.logger.warn('PlanningEngine', `Step ${index} is not a valid object.`, { step });
                    return null;
                }
                return {
                    id: uuidv4(),
                    description: step.description || `Execute step for ${step.tool || 'TBD'}`,
                    toolName: step.useTool === false ? 'none' : step.tool || 'TBD',
                    parameters: step.parameters || {},
                    successCriteria: 'Step completes without error.'
                };
            }).filter((step): step is PlannedStep => step !== null);

        } catch (error) {
            this.dependencies.logger.error('PlanningEngine', 'Failed to parse raw plan JSON, falling back to line-by-line.', { rawPlan, error });
            // Fallback to line-by-line parsing if JSON fails
            return rawPlan.split('\n')
                .map(line => line.trim())
                .filter(line => line.length > 0 && /^\d+\./.test(line))
                .map(line => ({
                    id: uuidv4(),
                    description: line.replace(/^\d+\.\s*/, ''),
                    toolName: 'TBD',
                    parameters: {},
                    successCriteria: 'Step completes without error.'
                }));
        }
    }
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/engines/ConversationEngine.ts

```ts
import { ConversationEngineInterface, RuntimeContext, Conversation, RuntimeDependencies } from "../types";
import { ConversationManager } from '../conversation/ConversationManager';
import { LLMRequestConfig } from '../../llm/types';

/**
 * The ConversationEngine is responsible for managing the conversational flow of an agent's task.
 */
export class ConversationEngine implements ConversationEngineInterface {
    private dependencies: RuntimeDependencies;

    constructor(dependencies: RuntimeDependencies) {
        this.dependencies = dependencies;
    }

    async initialize(): Promise<void> {
        this.dependencies.logger.info('ConversationEngine', 'ConversationEngine initialized');
    }

    getDependencies(): string[] {
        return ['logger', 'llmHandler'];
    }

    getState(): string {
        return 'ready';
    }

    async healthCheck(): Promise<boolean> {
        return true;
    }

    /**
     * Initiates a new conversation for a given task.
     * @param task The user's initial task description.
     * @param context The runtime context.
     * @returns A new Conversation object.
     */
    async initiate(task: string, context: { sessionId: string }): Promise<Conversation> {
        const conversation = new ConversationManager(task, context.sessionId);
        
        // For now, we'll just add a simple, hardcoded opening response.
        // In the future, this would use an LLM to be more dynamic.
        const openingResponse = `Understood. Starting task: ${task}`;
        conversation.addTurn('assistant', openingResponse);
        conversation.currentState = 'working';
        
        this.dependencies.logger.info('ConversationEngine', `Initiated conversation for task: ${task}`, {
            conversationId: conversation.id,
            sessionId: context.sessionId
        });

        return conversation;
    }

    /**
     * Runs the main execution logic and updates the conversation.
     * (This will be implemented in a later step)
     */
    async run(conversation: Conversation, _context: RuntimeContext): Promise<Conversation> {
        this.dependencies.logger.warn('ConversationEngine', 'run method is not yet implemented.');
        return conversation;
    }

    /**
     * Concludes the conversation after execution is complete.
     * This involves generating a final summary response.
     */
    async conclude(conversation: Conversation, context: RuntimeContext): Promise<Conversation> {
        this.dependencies.logger.info('ConversationEngine', 'Concluding conversation.', {
            conversationId: conversation.id,
            sessionId: context.sessionId
        });

        // Generate a final summary using the LLM
        const history = conversation.turns.map((turn: any) => `${turn.role}: ${turn.content}`).join('\n');
        const summaryPrompt = `Based on the following conversation, provide a concise summary of the outcome:\n\n${history}`;

        try {
            // Extract model from agent config, handling both string and object types
            let model: string | undefined;
            if (typeof context.agentConfig.llm === 'string') {
                model = context.agentConfig.llm;
            } else if (context.agentConfig.llm && typeof context.agentConfig.llm === 'object') {
                model = context.agentConfig.llm.model;
            }

            const request: any = {
                messages: [{ role: 'user', content: summaryPrompt }],
                llmConfig: {
                    model: model,
                    temperature: 0.5
                } as LLMRequestConfig
            };
            const response = await this.dependencies.llmHandler.complete(request);
            const finalResponse = response.content || 'I was unable to summarize my work.';

            conversation.addTurn('assistant', finalResponse);
            conversation.finalResponse = finalResponse;
            conversation.currentState = 'completed';

            this.dependencies.logger.info('ConversationEngine', 'Conversation concluded successfully.', {
                conversationId: conversation.id
            });

        } catch (error) {
            this.dependencies.logger.error('ConversationEngine', 'Failed to generate final summary', { error });
            conversation.currentState = 'error';
            conversation.finalResponse = 'I was unable to summarize my work.';
        }

        return conversation;
    }
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/engines/ExecutionEngine.ts

```ts
import { ExecutionEngineInterface } from "../types";
import { RuntimeDependencies } from "../types";
import { ToolResult, AgentConfig } from "../../types/sdk";
import { LLMRequest, LLMMessage, LLMConfig as RichLLMAgentConfig } from "../../llm/types";
import { SystemPromptService } from "../../agents/sysprompt";
import { ExecutionState } from "../context/ExecutionState";
import { LLMHandler } from '../../llm/handler';
import { LLMError, ToolError, ValidationError, ErrorCode, ErrorUtils } from '../../errors/index';

/**
 * The ExecutionEngine is responsible for the core "magic" of tool execution.
 * It preserves the original, brilliant unconscious tool execution logic.
 */
export class ExecutionEngine implements ExecutionEngineInterface {
    private dependencies: RuntimeDependencies;
    private systemPromptService: SystemPromptService;

    constructor(dependencies: RuntimeDependencies) {
        this.dependencies = dependencies;
        this.systemPromptService = new SystemPromptService();
    }

    async initialize(): Promise<void> {
        this.dependencies.logger.info('ExecutionEngine', 'ExecutionEngine initialized');
    }
    
    getDependencies(): string[] {
        return ['toolRegistry', 'contextAPI', 'llmHandler', 'logger'];
    }

    getState(): string {
        return 'ready';
    }

    async healthCheck(): Promise<boolean> {
        return true;
    }

    /**
     * Executes a task using iterative tool orchestration for multi-step workflows.
     * Supports both single-tool and multi-tool chaining scenarios.
     * @param task The task to execute.
     * @param agentConfig The configuration of the agent performing the task.
     * @param state The current execution state.
     * @returns A promise that resolves with the result of the tool execution.
     */
    async execute(task: string, agentConfig: AgentConfig, state: ExecutionState): Promise<ToolResult> {
        // Find the current step from the plan based on the task description
        const currentStep = state.plan?.steps.find(s => s.description === task);

        // If the step is a non-tool step, perform a simple LLM completion
        if (currentStep && currentStep.toolName === 'none') {
            this.dependencies.logger.info('ExecutionEngine', `Executing non-tool reasoning step: ${task}`);
            const llm = LLMHandler.getInstance();
            const response = await llm.complete({
                messages: [
                    { role: 'system', content: agentConfig.systemPrompt || 'You are a helpful assistant.' },
                    { role: 'user', content: `Continue with the plan. The current step is to: ${task}` }
                ]
            });

            return {
                success: true,
                result: {
                    response: response.toString(),
                    reasoning: `Executed a non-tool reasoning step as per the plan.`
                }
            };
        }

        try {
            this.dependencies.logger.info('ExecutionEngine', `Executing task: ${task}`);

            const agentHasTools = agentConfig.tools && agentConfig.tools.length > 0;
            let systemPrompt = this.systemPromptService.generateSystemPrompt(agentConfig, agentHasTools);
            
            if (agentConfig.directives && !agentConfig.systemPrompt) {
                systemPrompt += `\n\nAdditional Directives:\n${agentConfig.directives}`;
            }
            
            this.dependencies.logger.info('ExecutionEngine', 'Generated system prompt', {
                promptLength: systemPrompt.length,
                agentName: agentConfig.name,
                toolCount: agentConfig.tools.length,
                hasCustomDirectives: !!agentConfig.directives
            });

            // Check if task requires multi-tool orchestration
            const requiresOrchestration = this._detectMultiToolRequirement(task);
            
            if (requiresOrchestration && agentHasTools) {
                this.dependencies.logger.info('ExecutionEngine', 'Detected multi-tool requirement, using orchestration mode');
                return await this._executeWithOrchestration(task, systemPrompt, agentConfig, state);
            } else {
                this.dependencies.logger.info('ExecutionEngine', 'Using single-tool execution mode');
                return await this._executeSingleStep(task, systemPrompt, agentConfig, state);
            }

        } catch (error: any) {
            this.dependencies.logger.error('ExecutionEngine', 'Task execution failed', { 
                error: error.message, 
                task, 
                agentName: agentConfig.name 
            });

            if (error instanceof ToolError || error instanceof LLMError || error instanceof ValidationError) {
                // Already a SymphonyError, re-throw
                throw error;
            }

            // Convert generic errors
            const symphonyError = ErrorUtils.convertError(
                error,
                'ExecutionEngine',
                'execute',
                { task, agentName: agentConfig.name }
            );
            throw symphonyError;
        }
    }

    /**
     * Executes a single, concrete tool call from a plan step.
     * @param toolName The name of the tool to execute.
     * @param parameters The parameters for the tool.
     * @returns A promise that resolves with the result of the tool execution.
     */
    async executeStep(toolName: string, parameters: any): Promise<ToolResult> {
        this.dependencies.logger.info('ExecutionEngine', `Directly executing planned step`, { toolName, parameters });
        if (toolName === 'none') {
            return {
                success: true,
                result: { response: 'No tool action required for this step.' }
            };
        }
        try {
            return await this.dependencies.toolRegistry.executeTool(toolName, parameters);
        } catch (error: any) {
            this.dependencies.logger.error('ExecutionEngine', 'Direct step execution failed', {
                error: error.message,
                toolName
            });
            return {
                success: false,
                error: `Tool '${toolName}' execution failed: ${error.message}`
            };
        }
    }

    /**
     * Executes a multi-tool workflow with proper orchestration and context flow
     */
    private async _executeWithOrchestration(task: string, systemPrompt: string, agentConfig: AgentConfig, _state: ExecutionState): Promise<ToolResult> {
        const conversationHistory: LLMMessage[] = [];
        const allToolResults: any[] = [];
        let overallSuccess = true;
        let primaryError: string | undefined;
        let finalResponse = '';
        let orchestrationStep = 0;
        const maxSteps = 5; // Prevent infinite loops

        // Enhanced system prompt for orchestration
        const orchestrationPrompt = systemPrompt + `

--- ORCHESTRATION MODE ---
You are executing a multi-step task that requires using multiple tools in sequence.

IMPORTANT ORCHESTRATION RULES:
1. Analyze the full task and identify ALL required steps
2. Execute ONE tool at a time, in logical order
3. After each tool execution, I will provide you the result
4. Continue with the next tool based on the previous results
5. When all required tools have been executed, respond with "tool_name": "none" and provide final synthesis

Your task: ${task}

Start with the FIRST tool needed for this task.`;

        conversationHistory.push({ role: 'system', content: orchestrationPrompt });
        conversationHistory.push({ role: 'user', content: `Execute the first step of this task: ${task}` });

        while (orchestrationStep < maxSteps) {
            orchestrationStep++;
            this.dependencies.logger.info('ExecutionEngine', `Orchestration step ${orchestrationStep}`, {
                conversationLength: conversationHistory.length,
                toolsExecutedSoFar: allToolResults.length
            });

            // Execute current step
            const stepResult = await this._executeSingleOrchestrationStep(conversationHistory, agentConfig);
            
            if (!stepResult.success) {
                overallSuccess = false;
                primaryError = stepResult.error;
                break;
            }

            // Check if this step executed a tool
            if (stepResult.toolExecuted) {
                allToolResults.push(stepResult.toolExecuted);
                
                // Add tool result to conversation history for context
                conversationHistory.push({
                    role: 'assistant',
                    content: JSON.stringify({
                        tool_name: stepResult.toolExecuted.name,
                        parameters: stepResult.toolExecuted.parameters || {}
                    })
                });
                
                conversationHistory.push({
                    role: 'user', 
                    content: `Tool "${stepResult.toolExecuted.name}" completed. Result: ${JSON.stringify(stepResult.toolExecuted.result)}. ${allToolResults.length < agentConfig.tools.length ? 'Continue with the next required tool.' : 'If all required tools have been used, provide your final synthesis.'}`
                });
            } else {
                // No tool executed, this should be the final response
                finalResponse = stepResult.response || 'Task completed';
                break;
            }
        }

        if (orchestrationStep >= maxSteps) {
            this.dependencies.logger.warn('ExecutionEngine', 'Orchestration reached maximum steps limit');
            overallSuccess = false;
            primaryError = 'Orchestration exceeded maximum step limit';
        }

        // If we don't have a final response, generate one from the last tool result
        if (!finalResponse && allToolResults.length > 0) {
            const lastResult = allToolResults[allToolResults.length - 1];
            finalResponse = lastResult.result?.response || JSON.stringify(lastResult.result) || 'Multi-tool orchestration completed';
        }

        return {
            success: overallSuccess,
            result: {
                response: finalResponse,
                reasoning: `Orchestrated ${allToolResults.length} tools in sequence: ${allToolResults.map(t => t.name).join(' → ')}`,
                agent: agentConfig.name,
                timestamp: new Date().toISOString(),
                model: allToolResults[0]?.model || 'unknown',
                tokenUsage: allToolResults.reduce((sum, t) => sum + (t.tokenUsage || 0), 0),
                toolsExecuted: allToolResults,
                orchestrationSteps: orchestrationStep
            },
            error: primaryError
        };
    }

    /**
     * Executes a single step in the orchestration flow
     */
    private async _executeSingleOrchestrationStep(conversationHistory: LLMMessage[], agentConfig: AgentConfig): Promise<{
        success: boolean;
        toolExecuted?: any;
        response?: string;
        error?: string;
    }> {
        try {
            const agentLLMConfig = typeof agentConfig.llm === 'object' ? agentConfig.llm as RichLLMAgentConfig : null;
            
            const baseLlmSettings = {
                model: agentLLMConfig?.model || (typeof agentConfig.llm === 'string' ? agentConfig.llm : 'default-model'),
                temperature: agentLLMConfig?.temperature ?? 0.7,
                maxTokens: agentLLMConfig?.maxTokens ?? 2048,
            };

            const llmRequest: LLMRequest = {
                messages: [...conversationHistory],
                llmConfig: baseLlmSettings,
                expectsJsonResponse: true
            };

            const llmResponse = await this.dependencies.llmHandler.complete(llmRequest);

            if (!llmResponse || !llmResponse.content) {
                return { success: false, error: 'LLM response was empty' };
            }

            // Parse JSON response
            let parsedJson;
            try {
                parsedJson = JSON.parse(llmResponse.content);
            } catch (e) {
                return { success: false, error: `Invalid JSON response: ${llmResponse.content}` };
            }

            const toolName = parsedJson.tool_name || parsedJson.toolName;
            const parameters = parsedJson.parameters;

            if (toolName && toolName !== 'none' && parameters) {
                // Execute the tool
                const toolResult = await this.dependencies.toolRegistry.executeTool(toolName, parameters);
                
                return {
                    success: true,
                    toolExecuted: {
                        name: toolName,
                        parameters,
                        success: toolResult.success,
                        result: toolResult.result,
                        error: toolResult.error,
                        model: llmResponse.model,
                        tokenUsage: llmResponse.usage
                    }
                };
            } else if (toolName === 'none') {
                // Final response
                return {
                    success: true,
                    response: parsedJson.response || 'Task completed'
                };
            } else {
                return { success: false, error: `Invalid tool specification: ${toolName}` };
            }

        } catch (error) {
            return { 
                success: false, 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    }

    /**
     * Executes a single-step task (original behavior)
     */
    private async _executeSingleStep(task: string, systemPrompt: string, agentConfig: AgentConfig, state: ExecutionState): Promise<ToolResult> {
            const analysisResult = await this._analyzeAndExecute(task, systemPrompt, agentConfig, state);
            
            let overallTaskSuccess = true;
            let primaryError: string | undefined;
            let finalResponse = analysisResult.response;

            if (analysisResult.toolsExecuted && analysisResult.toolsExecuted.length > 0) {
                const firstFailedTool = analysisResult.toolsExecuted.find(t => !t.success);
                if (firstFailedTool) {
                    overallTaskSuccess = false;
                    primaryError = `Tool '${firstFailedTool.name}' failed: ${firstFailedTool.error || 'Unknown tool error'}`;
                }
            } else {
                const agentHasTools = agentConfig.tools && agentConfig.tools.length > 0;
                if (agentHasTools) {
                    let llmIndicatedNoToolViaJson = false;
                    if (analysisResult.response) { 
                        try {
                            const parsedResponse = JSON.parse(analysisResult.response);
                            if (parsedResponse.tool_name === 'none' || parsedResponse.toolName === 'none') {
                                llmIndicatedNoToolViaJson = true;
                            }
                        } catch (e) {
                             this.dependencies.logger.debug('ExecutionEngine', 'execute: Could not parse analysisResult.response as JSON.', { response: analysisResult.response });
                        }
                    }

                    if (!llmIndicatedNoToolViaJson) {
                        overallTaskSuccess = false;
                        primaryError = `Agent ${agentConfig.name} has tools but did not select one.`;
                        this.dependencies.logger.warn('ExecutionEngine', primaryError, { agentName: agentConfig.name });
                    }
                }
            }

            return {
                success: overallTaskSuccess,
                result: {
                    response: finalResponse,
                    reasoning: analysisResult.reasoning,
                    agent: analysisResult.agent,
                    timestamp: analysisResult.timestamp,
                    model: analysisResult.model,
                    tokenUsage: analysisResult.tokenUsage,
                    toolsExecuted: analysisResult.toolsExecuted
                },
                error: primaryError
            };
    }

    /**
     * Detects if a task requires multi-tool orchestration
     */
    private _detectMultiToolRequirement(task: string): boolean {
        const multiToolIndicators = [
            'first.*then', 'step 1.*step 2', 'sequence', 'chain', 'orchestration',
            'ponder.*write', 'analyze.*generate', 'think.*code', 'both',
            'first.*second', 'after.*then', 'followed by', 'next.*tool',
            'FIRST:', 'SECOND:', '1.', '2.', 'execute TWO tools'
        ];
        
        const taskLower = task.toLowerCase();
        return multiToolIndicators.some(indicator => {
            const regex = new RegExp(indicator.replace('.*', '.*?'), 'i');
            return regex.test(taskLower);
        });
    }

    private async _analyzeAndExecute(task: string, systemPrompt: string, agentConfig: AgentConfig, _state: ExecutionState) {
        const agentLLMConfig = typeof agentConfig.llm === 'object' ? agentConfig.llm as RichLLMAgentConfig : null;
        const agentHasTools = agentConfig.tools && agentConfig.tools.length > 0;

        let finalSystemPrompt = systemPrompt;

        if (agentHasTools) {
            let jsonInstruction = "";
            const baseInstruction = "\n\n--- BEGIN SDK JSON REQUIREMENTS ---";
            const endInstruction = "\n--- END SDK JSON REQUIREMENTS ---";
            let toolGuidance = `
YOU HAVE TOOLS. TO USE A TOOL: your JSON object MUST contain a "tool_name" (string) key AND a "parameters" (object) key.
IF NO TOOL IS NEEDED: your JSON object MUST contain a "tool_name" (string) key set EXPLICITLY to "none", AND a "response" (string) key with your direct textual answer.`;
            
            jsonInstruction = `${baseInstruction}\nYOUR ENTIRE RESPONSE MUST BE A SINGLE VALID JSON OBJECT.${toolGuidance}\nFAILURE TO ADHERE TO THIS JSON STRUCTURE WILL RESULT IN AN ERROR.${endInstruction}`;
            
            finalSystemPrompt += jsonInstruction;
        }

        const initialMessages: LLMMessage[] = [
            { role: 'system', content: finalSystemPrompt }, 
            { role: 'user', content: task }
        ];

        const baseLlmSettings = {
            model: agentLLMConfig?.model || (typeof agentConfig.llm === 'string' ? agentConfig.llm : 'default-model'),
            temperature: agentLLMConfig?.temperature ?? 0.7,
            maxTokens: agentLLMConfig?.maxTokens ?? 2048,
        };

        const llmRequest: LLMRequest = {
            messages: initialMessages,
            llmConfig: baseLlmSettings
        };

        if (agentHasTools) {
            llmRequest.expectsJsonResponse = true;
        }

        const llmResponse = await this.dependencies.llmHandler.complete(llmRequest);

        if (!llmResponse) {
            this.dependencies.logger.error('ExecutionEngine', 'LLM completion returned null/undefined.');
            throw new LLMError(
                ErrorCode.LLM_API_ERROR,
                'LLM completion failed - no response received',
                { task, agentConfig: agentConfig.name },
                { component: 'ExecutionEngine', operation: '_analyzeAndExecute' }
            );
        }

        let toolResults: any[] = [];
        let actualResponseContent = llmResponse.content;

        if (agentHasTools && llmResponse.content) { 
            try {
                const parsedJson = JSON.parse(llmResponse.content);
                const toolName = parsedJson.tool_name || parsedJson.toolName;
                const parameters = parsedJson.parameters;

                if (toolName && toolName !== 'none' && parameters) {
                    const toolResultData = await this.dependencies.toolRegistry.executeTool(toolName, parameters);
                    toolResults.push({ name: toolName, success: toolResultData.success, result: toolResultData.result, error: toolResultData.error });
                    actualResponseContent = `Tool ${toolName} executed. Success: ${toolResultData.success}. Result: ${JSON.stringify(toolResultData.result || toolResultData.error)}`;
                } else if (toolName === 'none') {
                    actualResponseContent = parsedJson.response || "No further action taken.";
                } else {
                    actualResponseContent = llmResponse.content; // Fallback to raw content
                }
            } catch (e) {
                this.dependencies.logger.error('ExecutionEngine', 'Failed to parse LLM content as JSON.', { content: llmResponse.content, error: e });
                actualResponseContent = llmResponse.content || "Error: LLM response was not valid JSON."; 
            }
        }

        return {
            response: actualResponseContent ?? '',
            reasoning: toolResults.length > 0 
                ? `Processed ${toolResults.length} tool actions. First tool: ${toolResults[0].name}, Success: ${toolResults[0].success}` 
                : (agentHasTools ? 'No tool actions taken as per LLM decision.' : 'Direct LLM response as agent has no tools.'),
            agent: agentConfig.name,
            timestamp: new Date().toISOString(),
            model: llmResponse.model,
            tokenUsage: llmResponse.usage,
            toolsExecuted: toolResults.length > 0 ? toolResults : undefined
        };
    }
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/engines/ReflectionEngine.ts

```ts
import { v4 as uuidv4 } from 'uuid';
import { ReflectionEngineInterface, RuntimeDependencies, Reflection, ExecutionStep, Conversation, ReflectionAssessment } from "../types";
import { ToolResult } from '../../types/sdk';
import { ExecutionState } from '../context/ExecutionState';

/**
 * The ReflectionEngine leverages the 'ponderTool' to analyze execution results
 * and provide actionable insights for course correction and learning.
 */
export class ReflectionEngine implements ReflectionEngineInterface {
    private dependencies: RuntimeDependencies;

    constructor(dependencies: RuntimeDependencies) {
        this.dependencies = dependencies;
    }

    async initialize(): Promise<void> {
        this.dependencies.logger.info('ReflectionEngine', 'ReflectionEngine initialized');
    }

    getDependencies(): string[] {
        return ['logger'];
    }

    getState(): string {
        return 'ready';
    }

    async healthCheck(): Promise<boolean> {
        return true;
    }

    /**
     * Reflects on an execution step by forming a query and calling the ponderTool.
     * @returns A Reflection object.
     */
    async reflect(stepResult: ExecutionStep, state: ExecutionState, _conversation: Conversation): Promise<Reflection> {
        this.dependencies.logger.info('ReflectionEngine', `Reflecting on step: ${stepResult.description}`);
        
        const query = this.createPonderQuery(stepResult);

        try {
            const ponderResult: ToolResult = await this.dependencies.toolRegistry.executeTool('ponderTool', {
                query: query,
                context: {
                    agentConfig: state.agentConfig,
                    fullPlan: state.plan,
                    executionHistory: state.history
                }
            });

            if (!ponderResult.success || !ponderResult.result?.conclusion) {
                this.dependencies.logger.warn('ReflectionEngine', 'ponderTool execution failed or returned no conclusion.');
                return this.createFallbackReflection(stepResult, 'Ponder tool failed.');
            }

            return this.parsePonderResultToReflection(stepResult, ponderResult.result);

        } catch (error) {
            this.dependencies.logger.error('ReflectionEngine', 'Error during ponderTool execution', { error });
            return this.createFallbackReflection(stepResult, `Ponder tool execution threw an error.`);
        }
    }

    private createPonderQuery(stepResult: ExecutionStep): string {
        if (stepResult.success) {
            return `My last action to '${stepResult.description}' succeeded. Was this the most optimal and efficient approach? Analyze my method and suggest any potential optimizations or alternative strategies for similar future tasks.`;
        } else {
            return `My attempt to '${stepResult.description}' failed with the error: "${stepResult.error}". Analyze the root cause of this failure. Consider the tool used, the parameters, and the overall goal. Suggest a concrete, actionable correction strategy. Should I retry, use a different tool, modify the parameters, or abort the plan?`;
        }
    }

    private parsePonderResultToReflection(stepResult: ExecutionStep, ponderResult: any): Reflection {
        const { conclusion } = ponderResult;
        
        const assessment: ReflectionAssessment = {
            performance: stepResult.success ? 'good' : 'poor',
            quality: 'good', // This could be improved by parsing ponder's analysis
            suggestedImprovements: conclusion.nextSteps || []
        };
        
        let suggestedAction: 'continue' | 'retry' | 'abort' | 'modify_plan' = 'continue';
        if (!stepResult.success) {
            // A simple keyword search in the ponder conclusion to suggest next action
            const conclusionText = (conclusion.summary || '').toLowerCase();
            if (conclusionText.includes('retry')) {
                suggestedAction = 'retry';
            } else if (conclusionText.includes('modify') || conclusionText.includes('alternative')) {
                suggestedAction = 'modify_plan';
            } else if (conclusionText.includes('abort')) {
                suggestedAction = 'abort';
            }
        }
        
        const reflection: Reflection = {
            id: uuidv4(),
            stepId: stepResult.stepId,
            assessment,
            suggestedAction,
            reasoning: conclusion.summary || 'No summary provided by ponderTool.',
            confidence: conclusion.confidence || 0.7,
            timestamp: Date.now()
        };
        return reflection;
    }

    private createFallbackReflection(stepResult: ExecutionStep, reason: string): Reflection {
        return {
            id: uuidv4(),
            stepId: stepResult.stepId,
            assessment: { performance: 'poor', quality: 'wrong' },
            suggestedAction: 'abort',
            reasoning: `Reflection failed: ${reason}`,
            confidence: 0.9,
            timestamp: Date.now()
        };
    }
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/symphony.ts

```ts
import { v4 as uuidv4 } from 'uuid';

import { AgentConfig } from '../types/sdk';
import { RuntimeContext, createRuntimeContext } from './context';
import {
  SymphonyRuntimeInterface,
  RuntimeResult,
  RuntimeStatus,
  RuntimeMetrics,
  RuntimeConfiguration,
  RuntimeDependencies,
  RuntimeExecutionMode,
  Conversation,
  ExecutionStep
} from './types';
import { ExecutionEngine } from './engines/ExecutionEngine';
import { ConversationEngine } from './engines/ConversationEngine';
import { Logger } from '../utils/logger';
import { PlanningEngine } from './engines/PlanningEngine';
import { ReflectionEngine } from './engines/ReflectionEngine';
import { RuntimeContextManager } from './context/RuntimeContextManager';
import { SymphonyError, ErrorCode, ErrorCategory, ErrorSeverity } from '../errors/index';

/**
 * Default runtime configuration
 */
const DEFAULT_RUNTIME_CONFIG: RuntimeConfiguration = {
  enhancedRuntime: false,
  planningThreshold: 'multi_step',
  reflectionEnabled: false,
  maxStepsPerPlan: 10,
  timeoutMs: 300000, // 5 minutes
  retryAttempts: 3,
  debugMode: false
};

/**
 * Production-grade Symphony Runtime orchestrator.
 * Coordinates execution engines to provide a seamless conversational and tool-using experience.
 */
export class SymphonyRuntime implements SymphonyRuntimeInterface {
  private readonly dependencies: RuntimeDependencies;
  private readonly config: RuntimeConfiguration;
  private readonly logger: Logger;
  private readonly executionEngine: ExecutionEngine;
  private readonly conversationEngine: ConversationEngine;
  private readonly planningEngine: PlanningEngine;
  private readonly reflectionEngine: ReflectionEngine;
  
  private status: RuntimeStatus = 'initializing';
  private metrics: RuntimeMetrics;
  private activeManagers: Map<string, RuntimeContextManager> = new Map();
  private initializationPromise?: Promise<void>;

  constructor(dependencies: RuntimeDependencies, config?: Partial<RuntimeConfiguration>) {
    this.dependencies = dependencies;
    this.config = { ...DEFAULT_RUNTIME_CONFIG, ...config };
    this.logger = dependencies.logger || Logger.getInstance('SymphonyRuntime');
    
    this.executionEngine = new ExecutionEngine(this.dependencies);
    this.conversationEngine = new ConversationEngine(this.dependencies);
    this.planningEngine = new PlanningEngine(this.dependencies);
    this.reflectionEngine = new ReflectionEngine(this.dependencies);
    
    this.metrics = this.createInitialMetrics();

    this.logger.info('SymphonyRuntime', 'Runtime orchestrator created', {
      enhancedRuntime: this.config.enhancedRuntime,
      planningThreshold: this.config.planningThreshold
    });
  }

  async initialize(): Promise<void> {
    if (this.initializationPromise) {
      return this.initializationPromise;
    }
    this.initializationPromise = this._performInitialization();
    return this.initializationPromise;
  }

  async execute(
    task: string,
    agentConfig: AgentConfig,
    _sessionId?: string,
    _executionOptions?: {
        planFirst?: boolean;
        streaming?: boolean;
        maxSteps?: number;
    }
  ): Promise<RuntimeResult> {
    if (this.status !== 'ready') {
        throw new SymphonyError({
            code: ErrorCode.EXECUTION_FAILED,
            category: ErrorCategory.RUNTIME,
            severity: ErrorSeverity.HIGH,
            message: `Runtime not ready. Status: ${this.status}`,
            details: { status: this.status, task, agentConfig: agentConfig.name },
            context: { component: 'SymphonyRuntime', operation: 'execute' },
            userGuidance: 'Ensure the runtime is properly initialized before executing tasks.',
            recoveryActions: ['Initialize the runtime', 'Check runtime configuration'],
            timestamp: new Date(),
            component: 'SymphonyRuntime',
            operation: 'execute'
        });
    }

    await this.initialize();
    if (this.status !== 'ready') {
        throw new SymphonyError({
            code: ErrorCode.EXECUTION_FAILED,
            category: ErrorCategory.RUNTIME,
            severity: ErrorSeverity.HIGH,
            message: `Runtime not ready after initialization. Status: ${this.status}`,
            details: { status: this.status, task, agentConfig: agentConfig.name },
            context: { component: 'SymphonyRuntime', operation: 'execute' },
            userGuidance: 'Runtime failed to initialize properly. Check logs for initialization errors.',
            recoveryActions: ['Check runtime dependencies', 'Review initialization logs', 'Restart the runtime'],
            timestamp: new Date(),
            component: 'SymphonyRuntime',
            operation: 'execute'
        });
    }

    const startTime = Date.now();
    const executionId = uuidv4();
    const context = this.createExecutionContext(agentConfig, executionId);
    const contextManager = new RuntimeContextManager(context, this.dependencies.contextAPI);
    this.activeManagers.set(executionId, contextManager);

    try {
        this.status = 'executing';
        let conversation = await this.conversationEngine.initiate(task, context);
        
        const taskAnalysis = await this.planningEngine.analyzeTask(task, context.toExecutionState());
        
        if (this.config.enhancedRuntime && taskAnalysis.requiresPlanning) {
            await this.executePlannedTask(task, contextManager, context.agentConfig, conversation, context);
        } else {
            await this.executeSingleShotTask(task, contextManager, context.agentConfig, conversation);
        }
        
        conversation = await this.conversationEngine.conclude(conversation, context);
        context.conversation = conversation.toJSON();
        context.status = context.errorHistory.length > 0 ? 'failed' : 'succeeded';
        
        const finalResult = this.constructFinalResult(context, conversation, startTime);
        this.updateMetrics(finalResult.mode, startTime, finalResult.success);
        return finalResult;

    } catch (error) {
        this.logger.error('SymphonyRuntime', 'Execution failed catastrophically', {
            executionId, error: error instanceof Error ? error.message : String(error)
        });
        context.status = 'failed';
        const failedResult = this.constructFinalResult(context, undefined, startTime, error as Error);
        return failedResult;
    } finally {
        await contextManager.generateExecutionInsights();
        await contextManager.performMaintenance();
        this.activeManagers.delete(executionId);
        this.status = 'ready';
    }
  }

  async shutdown(): Promise<void> {
    this.logger.info('SymphonyRuntime', 'Shutting down runtime');
    this.status = 'shutdown';
    for (const [executionId, manager] of this.activeManagers) {
      this.logger.warn('SymphonyRuntime', `Cancelling active execution: ${executionId}`);
      manager.updateStatus('aborted');
    }
    this.activeManagers.clear();
    this.logger.info('SymphonyRuntime', 'Runtime shutdown complete');
  }

  getStatus(): RuntimeStatus {
    return this.status;
  }

  getMetrics(): RuntimeMetrics {
    return { ...this.metrics };
  }

  async healthCheck(): Promise<boolean> {
    const checks = await Promise.allSettled([
      this.dependencies.toolRegistry.getAvailableTools().length > 0,
      this.dependencies.contextAPI.healthCheck(),
      this.dependencies.llmHandler ? Promise.resolve(true) : Promise.resolve(false)
    ]);
    return checks.every(result => result.status === 'fulfilled' && result.value === true);
  }

  private async _performInitialization(): Promise<void> {
    try {
      this.logger.info('SymphonyRuntime', 'Initializing runtime engines');
      await this.validateDependencies();
      await this.initializeEngines();
      this.status = 'ready';
      this.logger.info('SymphonyRuntime', 'Runtime initialization complete');
    } catch (error) {
      this.status = 'error';
      this.logger.error('SymphonyRuntime', 'Runtime initialization failed', { error });
      throw error;
    }
  }

  private async validateDependencies(): Promise<void> {
    const { toolRegistry, contextAPI, llmHandler, logger } = this.dependencies;
    if (!toolRegistry || !contextAPI || !llmHandler || !logger) {
      throw new SymphonyError({
        code: ErrorCode.MISSING_DEPENDENCY,
        category: ErrorCategory.CONFIGURATION,
        severity: ErrorSeverity.CRITICAL,
        message: 'Missing required runtime dependencies',
        details: { 
          hasToolRegistry: !!this.dependencies.toolRegistry,
          hasContextAPI: !!this.dependencies.contextAPI,
          hasLLMHandler: !!this.dependencies.llmHandler,
          hasLogger: !!this.dependencies.logger
        },
        context: { component: 'SymphonyRuntime', operation: 'validateDependencies' },
        userGuidance: 'Ensure all required dependencies are provided when creating the runtime.',
        recoveryActions: [
          'Verify toolRegistry is provided',
          'Verify contextAPI is provided', 
          'Verify llmHandler is provided',
          'Verify logger is provided'
        ],
        timestamp: new Date(),
        component: 'SymphonyRuntime',
        operation: 'validateDependencies'
      });
    }
  }

  private async initializeEngines(): Promise<void> {
    await this.executionEngine.initialize();
    await this.conversationEngine.initialize();
    await this.planningEngine.initialize();
    if (this.config.reflectionEnabled) {
      await this.reflectionEngine.initialize();
    }
    
    this.logger.info('SymphonyRuntime', 'Engines initialized', {
      execution: !!this.executionEngine,
      conversation: !!this.conversationEngine,
      planning: !!this.planningEngine,
      reflection: !!this.reflectionEngine && this.config.reflectionEnabled,
    });
  }

  private createExecutionContext(agentConfig: AgentConfig, sessionId?: string): RuntimeContext {
    return createRuntimeContext(agentConfig, sessionId);
  }

  private updateMetrics(mode: RuntimeExecutionMode, startTime: number, success: boolean): void {
    const duration = Date.now() - startTime;
    this.metrics.totalDuration += duration;
    this.metrics.stepCount += 1;
    this.metrics.toolCalls += 1;
    if (mode === 'enhanced_planning') {
      this.metrics.adaptationCount += 1;
    }
    if (!success) {
      this.logger.warn('SymphonyRuntime', `${mode} execution failed`, { duration });
    }
  }

  private createInitialMetrics(): RuntimeMetrics {
    return {
      totalDuration: 0, startTime: Date.now(), endTime: 0, stepCount: 0, toolCalls: 0, reflectionCount: 0, adaptationCount: 0
    };
  }

  private createFinalMetrics(startTime: number, context: RuntimeContext): RuntimeMetrics {
    const duration = Date.now() - startTime;
    return {
      totalDuration: duration,
      startTime: startTime,
      endTime: Date.now(),
      stepCount: context.executionHistory.length,
      toolCalls: context.executionHistory.filter(s => s.toolUsed).length,
      reflectionCount: context.getReflections().length,
      adaptationCount: 0 // To be implemented
    };
  }

  private async executePlannedTask(task: string, contextManager: RuntimeContextManager, agentConfig: AgentConfig, conversation: Conversation, context: RuntimeContext): Promise<void> {
    this.logger.info('SymphonyRuntime', 'Task requires planning. Executing multi-step plan.');
    const plan = await this.planningEngine.createExecutionPlan(task, agentConfig, contextManager.getExecutionState());
    contextManager.setPlan(plan);

    for (const step of plan.steps) {
        conversation.addTurn('assistant', `Executing step: ${step.description}`);
        
        const resolvedParameters = this.resolvePlaceholders(step.parameters, context.executionHistory);

        const stepResult = await this.executionEngine.executeStep(step.toolName, resolvedParameters);
        
        const executionStep: ExecutionStep = {
            stepId: uuidv4(),
            startTime: Date.now(),
            endTime: Date.now(),
            duration: 0,
            description: step.description,
            success: stepResult.success,
            result: stepResult.result,
            error: stepResult.error,
            summary: `Step "${step.description}" ${stepResult.success ? 'succeeded' : 'failed'}.`,
            toolUsed: stepResult.result?.toolsExecuted?.[0]?.name,
            parameters: stepResult.result?.toolsExecuted?.[0]?.parameters
        };
        await contextManager.recordStep(executionStep);

        if (this.config.reflectionEnabled && !stepResult.success) {
            const reflection = await this.reflectionEngine.reflect(executionStep, contextManager.getExecutionState(), conversation);
            contextManager.recordReflection(reflection);
            conversation.addTurn('assistant', `Reflection: ${reflection.reasoning}`);
        }

        if (!stepResult.success) {
            conversation.addTurn('assistant', `Step failed: ${step.description}. Error: ${stepResult.error}`);
            conversation.currentState = 'error';
            contextManager.updateStatus('failed');
            break; 
        }
    }
  }

  private resolvePlaceholders(parameters: any, history: ReadonlyArray<ExecutionStep>): any {
    if (!parameters || typeof parameters !== 'object') {
        return parameters;
    }

    const resolved = JSON.parse(JSON.stringify(parameters));

    const fullPlaceholderRegex = /^\{\{step_(\d+)_output(?:\.(.*))?\}\}$/;
    const partialPlaceholderRegex = /\{\{step_(\d+)_output(?:\.(.*))?\}\}/g;

    const resolveValue = (stepIndexStr: string, propertyPath: string | undefined): any => {
        const stepIndex = parseInt(stepIndexStr, 10) - 1;
        
        if (history[stepIndex] && history[stepIndex].success) {
            let currentValue = history[stepIndex].result;
            
            if (propertyPath) {
                const props = propertyPath.split('.');
                for (const prop of props) {
                    if (currentValue && typeof currentValue === 'object' && prop in currentValue) {
                        currentValue = (currentValue as any)[prop];
                    } else {
                        this.logger.warn('SymphonyRuntime', `Could not resolve property path "${propertyPath}" in step ${stepIndex + 1} output.`, { propertyPath, stepOutput: history[stepIndex].result });
                        return undefined; // Property path not found
                    }
                }
            }
            return currentValue;
        }
        
        this.logger.warn('SymphonyRuntime', `Referenced step ${stepIndex + 1} not found or failed.`, { stepIndex: stepIndex + 1, historyCount: history.length });
        return undefined; // Step not found or failed
    };

    const traverseAndResolve = (obj: any): any => {
        if (!obj || typeof obj !== 'object') {
            return obj;
        }

        if (Array.isArray(obj)) {
            return obj.map(item => traverseAndResolve(item));
        }

        for (const key in obj) {
            if (Object.prototype.hasOwnProperty.call(obj, key)) {
                if (typeof obj[key] === 'string') {
                    const strValue = obj[key];

                    // Case 1: The entire string is a placeholder. Replace it with the resolved value, preserving type.
                    const fullMatch = strValue.match(fullPlaceholderRegex);
                    if (fullMatch) {
                        const [, stepIndexStr, propertyPath] = fullMatch;
                        const resolvedValue = resolveValue(stepIndexStr, propertyPath);
                        if (resolvedValue !== undefined) {
                            obj[key] = resolvedValue;
                        }
                        continue; // Move to next key
                    }

                    // Case 2: The string contains one or more placeholders (partial substitution).
                    // The result must be a string.
                    obj[key] = strValue.replace(partialPlaceholderRegex, (match, stepIndexStr, propertyPath) => {
                        const resolvedValue = resolveValue(stepIndexStr, propertyPath);
                        if (resolvedValue === undefined) {
                            return match; // Keep original placeholder if not found
                        }
                        return typeof resolvedValue === 'object' ? JSON.stringify(resolvedValue) : String(resolvedValue);
                    });

                } else if (typeof obj[key] === 'object') {
                    // Recurse for nested objects or arrays
                    traverseAndResolve(obj[key]);
                }
            }
        }
        return obj;
    }

    return traverseAndResolve(resolved);
  }

  private async executeSingleShotTask(task: string, contextManager: RuntimeContextManager, agentConfig: AgentConfig, conversation: Conversation): Promise<void> {
    this.logger.info('SymphonyRuntime', 'Executing single-shot task.');
    const executionResult = await this.executionEngine.execute(task, agentConfig, contextManager.getExecutionState());
    
    await contextManager.recordStep({
        stepId: uuidv4(),
        startTime: Date.now(),
        endTime: Date.now(),
        duration: 0,
        description: task,
        success: executionResult.success,
        result: executionResult.result,
        error: executionResult.error,
        summary: `Single-shot task execution.`,
        toolUsed: executionResult.result?.toolsExecuted?.[0]?.name,
        parameters: executionResult.result?.toolsExecuted?.[0]?.parameters
    } as ExecutionStep);

    if (executionResult.success) {
        const finalContent = executionResult.result?.response || JSON.stringify(executionResult.result);
        conversation.addTurn('assistant', `Completed task successfully. Result: ${finalContent}`);
        conversation.finalResponse = finalContent;
    } else {
        conversation.addTurn('assistant', `Failed to complete task. Error: ${executionResult.error}`);
    }
  }

  private constructFinalResult(context: RuntimeContext, conversation: Conversation | undefined, startTime: number, error?: Error): RuntimeResult {
    const finalState = context.toExecutionState();
    const finalSuccess = !error && finalState.errors.length === 0 && finalState.status !== 'failed' && finalState.status !== 'aborted';
    
    const finalResult: RuntimeResult = {
        success: finalSuccess,
        mode: context.currentPlan ? 'enhanced_planning' : 'legacy_with_conversation',
        conversation: conversation?.toJSON(),
        plan: context.currentPlan,
        executionDetails: {
            mode: context.currentPlan ? 'enhanced_planning' : 'legacy_with_conversation',
            stepResults: context.executionHistory,
            totalSteps: context.totalSteps || (finalSuccess ? 1 : 0),
            completedSteps: context.executionHistory.filter(s => s.success).length,
            failedSteps: context.executionHistory.filter(s => !s.success).length,
            adaptations: [],
            insights: finalState.insights
        },
        error: error ? error.message : (finalSuccess ? undefined : finalState.errors[finalState.errors.length - 1]?.message),
        metrics: this.createFinalMetrics(startTime, context)
    };

    return finalResult;
  }
}

export function createSymphonyRuntime(
  dependencies: RuntimeDependencies,
  config?: Partial<RuntimeConfiguration>
): SymphonyRuntime {
  return new SymphonyRuntime(dependencies, config);
}

export function createSymphonyRuntimeWithFlags(
  dependencies: RuntimeDependencies,
  environmentFlags?: Record<string, string>
): SymphonyRuntime {
  const config: Partial<RuntimeConfiguration> = {
    enhancedRuntime: environmentFlags?.ENHANCED_RUNTIME === 'true',
    planningThreshold: (environmentFlags?.PLANNING_THRESHOLD as any) || 'multi_step',
    reflectionEnabled: environmentFlags?.REFLECTION_ENABLED === 'true',
    debugMode: environmentFlags?.DEBUG_MODE === 'true'
  };
  return new SymphonyRuntime(dependencies, config);
} 
```

## File: /Users/deepsaint/Desktop/symphony-sdk/src/runtime/conversation/ConversationManager.ts

```ts
import { v4 as uuidv4 } from 'uuid';
import { Conversation, ConversationTurn, ConversationState, ConversationMetadata, ConversationJSON } from "../types";

/**
 * Manages the state and flow of a single conversation.
 */
export class ConversationManager implements Conversation {
    public readonly id: string;
    public readonly originalTask: string;
    public readonly sessionId: string;
    public readonly createdAt: number;
    
    public turns: ConversationTurn[] = [];
    public currentState: ConversationState = 'initiated';
    public finalResponse?: string;

    constructor(task: string, sessionId: string) {
        this.id = uuidv4();
        this.originalTask = task;
        this.sessionId = sessionId;
        this.createdAt = Date.now();

        // Add the initial user request as the first turn
        this.addTurn('user', task);
    }

    public addTurn(role: 'user' | 'assistant', content: string, metadata?: ConversationMetadata): ConversationTurn {
        const turn: ConversationTurn = {
            id: uuidv4(),
            role,
            content,
            timestamp: Date.now(),
            metadata
        };
        this.turns.push(turn);
        return turn;
    }

    public getRecentTurns(count: number): ConversationTurn[] {
        return this.turns.slice(-count);
    }

    public getFinalResponse(): string | undefined {
        if (this.currentState === 'completed') {
            const lastTurn = this.turns[this.turns.length - 1];
            if (lastTurn?.role === 'assistant') {
                return lastTurn.content;
            }
        }
        return undefined;
    }

    public getReasoningChain(): string[] {
        return this.turns
            .filter(turn => turn.role === 'assistant' && turn.metadata?.toolUsed)
            .map(turn => `Used tool ${turn.metadata!.toolUsed} to: ${turn.content}`);
    }

    public getFlowSummary(): string {
        const userRequest = this.turns[0]?.content || this.originalTask;
        const finalResponse = this.getFinalResponse() || "Task in progress.";
        return `Task: "${userRequest}" -> Result: "${finalResponse}"`;
    }

    public getCurrentState(): ConversationState {
        return this.currentState;
    }

    public toJSON(): ConversationJSON {
        return {
            id: this.id,
            originalTask: this.originalTask,
            turns: this.turns,
            finalResponse: this.finalResponse || this.getFinalResponse() || "",
            reasoningChain: this.getReasoningChain(),
            duration: Date.now() - this.createdAt,
            state: this.currentState
        };
    }
} 
```



---

> 📸 Generated with [Jockey CLI](https://github.com/saint0x/jockey-cli)
