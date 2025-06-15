# Aria Firmware: Syntax Migration Guide

This document outlines the migration from Symphony SDK's object-based configuration to Aria's decorator-based `.aria` file syntax. The new syntax maintains all the power and flexibility of the current Symphony SDK while providing better developer experience and enabling shareable `.aria` bundles.

## Table of Contents

- [Overview](#overview)
- [Migration Principles](#migration-principles)
- [Tool Migration](#tool-migration)
- [Agent Migration](#agent-migration)
- [Team Migration](#team-migration)
- [Pipeline Migration](#pipeline-migration)
- [Complete Example](#complete-example)
- [Compilation & Distribution](#compilation--distribution)

## Overview

### Current Symphony SDK Pattern
```typescript
// Current: Object-based configuration
const tool = await symphony.tool.create(toolConfig);
const agent = await symphony.agent.create(agentConfig);
const team = await symphony.team.create(teamConfig);
```

### New Aria Decorator Pattern
```typescript
// New: Decorator-based with class structure
@aria({
  name: "MySystem",
  description: "Complete agentic system",
  version: "1.0.0"
})
export class MySystem {
  @tool({ /* config */ })
  async myTool() { /* implementation */ }
  
  @agent({ /* config */ })
  async myAgent() { /* implementation */ }
  
  @team({ /* config */ })
  async myTeam() { /* implementation */ }
}
```

## Migration Principles

1. **Multi-line decorators** for rich configuration
2. **Class-based organization** for better structure
3. **Full compatibility** with existing Symphony SDK features
4. **IDE support** with autocomplete and type checking
5. **Shareable .aria bundles** for distribution

## Tool Migration

### Before: Symphony SDK ToolConfig
```typescript
import { ToolConfig, ToolResult, ToolRegistry } from 'symphonic';

const customEmailTool: ToolConfig = {
  name: 'sendTransactionalEmail',
  description: 'Sends a transactional email to a user.',
  type: 'communication',
  nlp: 'send an email to a user with subject and body',
  config: {
    inputSchema: {
      type: 'object',
      properties: {
        to: { type: 'string', description: "Recipient's email", format: 'email' },
        subject: { type: 'string', description: "Email subject" },
        body: { type: 'string', description: "Email body" },
        templateId: { type: 'string', description: "Template ID" },
        templateVariables: { type: 'object', description: "Template variables" }
      },
      required: ['to', 'subject', 'body']
    }
  },
  handler: async (params) => {
    // Implementation logic
    const messageId = `msg_${Date.now()}`;
    return {
      success: true,
      result: { messageId, deliveryStatus: 'sent' },
      metrics: { duration: Date.now() - startTime }
    };
  },
  timeout: 10000,
  retryCount: 2
};

const toolRegistry = ToolRegistry.getInstance();
toolRegistry.registerTool(customEmailTool.name, customEmailTool);
```

### After: Aria Decorator Syntax
```typescript
@aria({
  name: "EmailSystem",
  description: "Comprehensive email sending system",
  version: "1.0.0"
})
export class EmailSystem {

  @tool({
    name: "sendTransactionalEmail",
    description: "Sends a transactional email to a user.",
    type: "communication",
    nlp: "send an email to a user with subject and body",
    parameters: [
      { 
        name: "to", 
        type: "string", 
        description: "Recipient's email address", 
        required: true,
        format: "email"
      },
      { 
        name: "subject", 
        type: "string", 
        description: "Email subject line", 
        required: true 
      },
      { 
        name: "body", 
        type: "string", 
        description: "Email body content", 
        required: true 
      },
      { 
        name: "templateId", 
        type: "string", 
        description: "Optional template ID", 
        required: false 
      },
      { 
        name: "templateVariables", 
        type: "object", 
        description: "Template personalization variables", 
        required: false 
      }
    ],
    timeout: 10000,
    retryCount: 2
  })
  async sendTransactionalEmail(params: {
    to: string;
    subject: string;
    body: string;
    templateId?: string;
    templateVariables?: Record<string, any>;
  }) {
    const startTime = Date.now();
    console.log(`Sending email to: ${params.to}`);
    
    try {
      if (!params.to.includes('@')) {
        throw new Error('Invalid recipient email address.');
      }
      
      // Simulate email sending
      await new Promise(resolve => setTimeout(resolve, 500));
      
      const messageId = `msg_${Date.now()}`;
      return {
        success: true,
        result: { messageId, deliveryStatus: 'sent' },
        metrics: { duration: Date.now() - startTime }
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        metrics: { duration: Date.now() - startTime }
      };
    }
  }
}
```

## Agent Migration

### Before: Symphony SDK AgentConfig
```typescript
const emailAgentConfig: AgentConfig = {
  name: 'NotificationAgent',
  description: 'An agent responsible for sending user notifications via email.',
  task: 'Send transactional and notification emails based on system events.',
  tools: ['sendTransactionalEmail'],
  llm: {
    provider: 'openai',
    model: 'gpt-3.5-turbo',
    temperature: 0.2,
    maxTokens: 1500
  },
  systemPrompt: "You are the Notification Agent...",
  maxCalls: 3
};

const emailAgent = new AgentExecutor(emailAgentConfig);
```

### After: Aria Decorator Syntax
```typescript
@aria({
  name: "NotificationSystem",
  description: "Intelligent notification and communication system",
  version: "1.0.0"
})
export class NotificationSystem {

  @tool({
    name: "sendTransactionalEmail",
    // ... tool config as shown above
  })
  async sendTransactionalEmail(params: any) {
    // ... tool implementation
  }

  @agent({
    name: "NotificationAgent",
    description: "An agent responsible for sending user notifications via email.",
    task: "Send transactional and notification emails based on system events.",
    tools: ["sendTransactionalEmail"],
    llm: {
      provider: "openai",
      model: "gpt-3.5-turbo",
      temperature: 0.2,
      maxTokens: 1500
    },
    systemPrompt: "You are the Notification Agent. Your job is to send emails using the 'sendTransactionalEmail' tool. When asked to send an email, carefully prepare the parameters based on the request.",
    maxCalls: 3,
    timeout: 60000,
    capabilities: ["email_sending", "user_notifications"]
  })
  async notificationAgent(task: string) {
    // Agent implementation - the orchestrator will handle LLM interaction
    // and tool calling based on the configuration above
    return {
      success: true,
      message: `NotificationAgent processing: ${task}`
    };
  }
}
```

## Team Migration

### Before: Symphony SDK TeamConfig
```typescript
const customerSupportTeamConfig: TeamConfig = {
  name: 'CustomerSupportTeam',
  description: 'A team to handle customer inquiries and notifications.',
  agents: [emailAgentConfig, querySupportAgentConfig],
  strategy: {
    name: 'coordinated_delegation',
    description: 'Manager delegates tasks to appropriate members.',
    coordinationRules: {
      maxParallelTasks: 1,
      taskTimeout: 180000
    }
  }
};
```

### After: Aria Decorator Syntax
```typescript
@aria({
  name: "CustomerSupportSystem",
  description: "Complete customer support and communication system",
  version: "1.0.0"
})
export class CustomerSupportSystem {

  @tool({
    name: "sendTransactionalEmail",
    // ... tool config
  })
  async sendTransactionalEmail(params: any) { /* ... */ }

  @agent({
    name: "NotificationAgent",
    description: "Handles email notifications",
    tools: ["sendTransactionalEmail"],
    llm: {
      provider: "openai",
      model: "gpt-3.5-turbo",
      temperature: 0.2
    }
  })
  async notificationAgent(task: string) { /* ... */ }

  @agent({
    name: "QuerySupportAgent", 
    description: "Handles user queries and provides information",
    tools: [],
    llm: {
      provider: "openai",
      model: "gpt-3.5-turbo", 
      temperature: 0.3
    },
    systemPrompt: "You are a helpful Query Support Agent. Answer user questions clearly. If a user needs an email sent, indicate that the NotificationAgent should handle it.",
    maxCalls: 2
  })
  async querySupportAgent(task: string) { /* ... */ }

  @team({
    name: "CustomerSupportTeam",
    description: "A team to handle customer inquiries and notifications.",
    agents: ["NotificationAgent", "QuerySupportAgent"],
    strategy: {
      name: "coordinated_delegation",
      description: "Manager delegates tasks to appropriate members.",
      coordinationRules: {
        maxParallelTasks: 1,
        taskTimeout: 180000
      }
    },
    capabilities: ["customer_support", "email_notifications", "query_resolution"]
  })
  async customerSupportTeam(request: string) {
    // Team coordination logic
    return {
      success: true,
      message: `CustomerSupportTeam handling: ${request}`
    };
  }
}
```

## Pipeline Migration

### Before: Symphony SDK PipelineConfig
```typescript
const supportRequestPipelineConfig: PipelineConfig = {
  name: 'UserSupportRequestPipeline',
  description: 'Handles incoming user support requests and takes appropriate action.',
  variables: {
    userInputQuery: 'I need to reset my password',
    userEmail: 'user@example.com'
  },
  steps: [
    {
      id: 'step1_understand_request',
      name: 'Understand User Request',
      type: 'agent',
      agent: 'QuerySupportAgent',
      inputs: {
        task_description: `User query: '$userInputQuery'. Determine if email needed.`
      },
      outputs: { agent_analysis: '.result.response' }
    },
    {
      id: 'step2_conditional_email',
      name: 'Conditionally Send Email',
      type: 'tool',
      tool: 'conditionalAgentRunnerTool',
      dependencies: ['step1_understand_request'],
      inputs: {
        condition: '@step1_understand_request.agent_analysis',
        agentToRun: 'NotificationAgent'
      }
    }
  ]
};
```

### After: Aria Decorator Syntax
```typescript
@aria({
  name: "SupportPipelineSystem",
  description: "Automated support request processing pipeline",
  version: "1.0.0"
})
export class SupportPipelineSystem {

  @tool({ /* ... */ })
  async sendTransactionalEmail(params: any) { /* ... */ }

  @agent({ /* ... */ })
  async querySupportAgent(task: string) { /* ... */ }

  @agent({ /* ... */ })
  async notificationAgent(task: string) { /* ... */ }

  @pipeline({
    name: "UserSupportRequestPipeline",
    description: "Handles incoming user support requests and takes appropriate action.",
    variables: {
      userInputQuery: "I need to reset my password",
      userEmail: "user@example.com",
      defaultSubject: "Regarding your Support Request"
    },
    steps: [
      {
        id: "step1_understand_request",
        name: "Understand User Request", 
        type: "agent",
        agent: "QuerySupportAgent",
        inputs: {
          task_description: "User query: '$userInputQuery'. Determine if email needed."
        },
        outputs: {
          agent_analysis: ".result.response"
        }
      },
      {
        id: "step2_conditional_email",
        name: "Conditionally Send Email",
        type: "agent",
        agent: "NotificationAgent",
        dependencies: ["step1_understand_request"],
        inputs: {
          task_description: "Based on analysis '@step1_understand_request.agent_analysis', send email to '$userEmail' if needed."
        },
        outputs: {
          email_result: ".result"
        }
      }
    ],
    errorStrategy: {
      type: "retry",
      maxAttempts: 2
    }
  })
  async userSupportRequestPipeline(variables: {
    userInputQuery: string;
    userEmail: string;
    defaultSubject?: string;
  }) {
    // Pipeline orchestration logic
    return {
      success: true,
      message: `Processing support request for ${variables.userEmail}`
    };
  }
}
```

## Complete Example

Here's a comprehensive example showing all patterns together:

```typescript
@aria({
  name: "ComprehensiveBusinessSystem",
  description: "Complete business automation system with tools, agents, teams, and pipelines",
  version: "2.0.0",
  author: "Business Automation Corp",
  license: "MIT"
})
export class ComprehensiveBusinessSystem {

  // ==================== TOOLS ====================

  @tool({
    name: "webSearch",
    description: "Search the web for information using Serper.dev API",
    type: "research",
    parameters: [
      { name: "query", type: "string", required: true, description: "Search query" },
      { name: "resultCount", type: "number", required: false, description: "Number of results" }
    ],
    timeout: 30000,
    retryCount: 3
  })
  async webSearch(params: { query: string; resultCount?: number }) {
    // Implementation for web search
    return {
      success: true,
      result: { articles: [], totalResults: 0 }
    };
  }

  @tool({
    name: "sendEmail",
    description: "Send professional emails",
    type: "communication", 
    parameters: [
      { name: "to", type: "string", required: true, format: "email" },
      { name: "subject", type: "string", required: true },
      { name: "body", type: "string", required: true }
    ]
  })
  async sendEmail(params: { to: string; subject: string; body: string }) {
    // Email implementation
    return { success: true, result: { messageId: `msg_${Date.now()}` } };
  }

  @tool({
    name: "writeFile",
    description: "Write content to a file",
    type: "storage",
    parameters: [
      { name: "filename", type: "string", required: true },
      { name: "content", type: "string", required: true }
    ]
  })
  async writeFile(params: { filename: string; content: string }) {
    // File writing implementation
    return { success: true, result: { path: params.filename, size: params.content.length } };
  }

  // ==================== AGENTS ====================

  @agent({
    name: "ResearchAgent",
    description: "Specialized research agent for gathering and analyzing information",
    tools: ["webSearch", "writeFile"],
    llm: {
      provider: "openai",
      model: "gpt-4o-mini",
      temperature: 0.3,
      maxTokens: 3000
    },
    systemPrompt: "You are an expert research agent. Conduct thorough research and provide comprehensive analysis.",
    capabilities: ["web_research", "data_analysis", "report_writing"],
    maxCalls: 10,
    timeout: 300000
  })
  async researchAgent(task: string) {
    return { success: true, message: `Research agent processing: ${task}` };
  }

  @agent({
    name: "CommunicationAgent",
    description: "Handles all business communications and correspondence",
    tools: ["sendEmail"],
    llm: {
      provider: "openai",
      model: "gpt-3.5-turbo",
      temperature: 0.5,
      maxTokens: 2000
    },
    systemPrompt: "You are a professional communication specialist. Draft clear, professional correspondence.",
    capabilities: ["email_drafting", "customer_communication"],
    maxCalls: 5
  })
  async communicationAgent(task: string) {
    return { success: true, message: `Communication agent processing: ${task}` };
  }

  // ==================== TEAMS ====================

  @team({
    name: "BusinessIntelligenceTeam", 
    description: "Team focused on market research and business intelligence",
    agents: ["ResearchAgent", "CommunicationAgent"],
    strategy: {
      name: "pipeline_execution",
      description: "Sequential execution with research followed by communication",
      coordinationRules: {
        maxParallelTasks: 2,
        taskTimeout: 600000,
        handoffProtocol: "structured_summary"
      }
    },
    capabilities: ["market_analysis", "competitive_research", "stakeholder_communication"]
  })
  async businessIntelligenceTeam(objective: string) {
    return { success: true, message: `BI Team working on: ${objective}` };
  }

  // ==================== PIPELINES ====================

  @pipeline({
    name: "MarketAnalysisPipeline",
    description: "Complete market analysis from research to stakeholder communication",
    variables: {
      researchTopic: "AI market trends 2024",
      stakeholderEmail: "executives@company.com",
      reportFormat: "executive_summary",
      urgencyLevel: "normal"
    },
    steps: [
      {
        id: "research_phase",
        name: "Market Research",
        type: "agent",
        agent: "ResearchAgent",
        inputs: {
          task_description: "Conduct comprehensive research on '$researchTopic'. Focus on trends, key players, and market opportunities."
        },
        outputs: {
          research_findings: ".result.analysis",
          data_sources: ".result.sources"
        }
      },
      {
        id: "report_generation",
        name: "Generate Report",
        type: "tool", 
        tool: "writeFile",
        dependencies: ["research_phase"],
        inputs: {
          filename: "market_analysis_$researchTopic.md",
          content: "# Market Analysis Report\n\n@research_phase.research_findings\n\nSources: @research_phase.data_sources"
        },
        outputs: {
          report_file: ".result.path"
        }
      },
      {
        id: "stakeholder_communication",
        name: "Notify Stakeholders",
        type: "agent",
        agent: "CommunicationAgent", 
        dependencies: ["report_generation"],
        inputs: {
          task_description: "Draft and send an email to '$stakeholderEmail' about the completed market analysis report. Report location: @report_generation.report_file"
        },
        outputs: {
          communication_status: ".result.status"
        }
      }
    ],
    errorStrategy: {
      type: "retry",
      maxAttempts: 3,
      backoffMs: 5000
    },
    timeout: 1800000 // 30 minutes
  })
  async marketAnalysisPipeline(variables: {
    researchTopic: string;
    stakeholderEmail: string;
    reportFormat?: string;
    urgencyLevel?: string;
  }) {
    return { 
      success: true, 
      message: `Market analysis pipeline initiated for: ${variables.researchTopic}` 
    };
  }
}
```

## Compilation & Distribution

### Compilation Process
```bash
# Parse decorators and generate .aria bundle
arc build ComprehensiveBusinessSystem.ts

# Output: comprehensive_business_system.aria
```

### Generated .aria Bundle Structure
```json
{
  "manifest": {
    "name": "ComprehensiveBusinessSystem",
    "version": "2.0.0", 
    "description": "Complete business automation system...",
    "author": "Business Automation Corp",
    "license": "MIT",
    "entry_point": "ComprehensiveBusinessSystem",
    "tools": [
      {
        "name": "webSearch",
        "description": "Search the web for information...",
        "type": "research",
        "parameters": [...],
        "timeout": 30000
      }
    ],
    "agents": [
      {
        "name": "ResearchAgent",
        "description": "Specialized research agent...",
        "tools": ["webSearch", "writeFile"],
        "llm": { "provider": "openai", "model": "gpt-4o-mini" },
        "capabilities": ["web_research", "data_analysis"]
      }
    ],
    "teams": [...],
    "pipelines": [...],
    "resource_tokens": [
      { "type": "network", "access": "read", "apis": ["serper"] },
      { "type": "llm", "provider": "openai", "models": ["gpt-4o-mini", "gpt-3.5-turbo"] },
      { "type": "io", "access": "write", "scope": "files" }
    ]
  },
  "source": "...base64 encoded TypeScript...",
  "signature": "...ed25519 signature...",
  "checksum": "...blake3 hash..."
}
```

### Distribution & Usage
```bash
# Create new project
arc new my-business-system

# Build .aria bundle
arc build comprehensive_business_system.ts

# Upload to firmware
arc upload comprehensive_business_system.aria --target production

# Share with community
arc publish comprehensive_business_system.aria --registry public

# Install from registry
arc install ComprehensiveBusinessSystem --from public

# Run specific components
arc run ResearchAgent "Research quantum computing market"
arc run BusinessIntelligenceTeam "Analyze competitor landscape"
arc run MarketAnalysisPipeline --variables '{"researchTopic": "Quantum AI"}'
```

This syntax migration provides:
- ✅ **Rich decorator configurations** matching Symphony SDK capabilities
- ✅ **Multi-line, IDE-friendly syntax** with full autocomplete
- ✅ **Class-based organization** for better code structure  
- ✅ **Shareable .aria bundles** for distribution
- ✅ **Full backward compatibility** with existing Symphony patterns
- ✅ **Type safety** and validation built-in
- ✅ **Automatic resource token generation** for Quilt integration 