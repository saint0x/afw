use crate::engines::llm::LLMHandler;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use regex::Regex;

// Structured thinking patterns for deep analysis
const THINKING_PATTERNS: &[&str] = &[
    "break down complex problems into fundamental truths",
    "explore unconventional connections and possibilities", 
    "analyze interconnections and emergent properties",
    "examine tensions and synthesize opposing views",
    "reflect on the thinking process itself"
];

const PATTERN_NAMES: &[&str] = &[
    "FIRST_PRINCIPLES",
    "LATERAL", 
    "SYSTEMS",
    "DIALECTICAL",
    "METACOGNITIVE"
];

// Thought structure tags for LLM
const THOUGHT_TAGS: &[(&str, &str)] = &[
    ("START", "<thinking>"),
    ("END", "</thinking>"),
    ("OBSERVATION", "<observation>"),
    ("ANALYSIS", "<analysis>"),
    ("SYNTHESIS", "<synthesis>"),
    ("IMPLICATION", "<implication>"),
    ("METACOGNITION", "<metacognition>"),
    ("EVIDENCE", "<evidence>"),
    ("UNCERTAINTY", "<uncertainty>"),
    ("INSIGHT", "<insight>")
];

#[derive(Debug, Clone)]
struct Thought {
    depth: u32,
    pattern: String,
    observation: String,
    analysis: String,
    synthesis: String,
    implication: String,
    metacognition: String,
    insights: Vec<String>,
    confidence: f64,
    context: HashMap<String, DeepValue>,
}

#[derive(Debug, Clone)]
struct ThinkingContext {
    thinking_patterns: Vec<String>,
    steps: String,
    depth: u32,
    iteration: u32,
    parent_thought: Option<Box<Thought>>,
    extra_context: HashMap<String, DeepValue>,
}

#[derive(Debug, Clone)]
struct Conclusion {
    summary: String,
    key_insights: Vec<String>,
    implications: String,
    uncertainties: String,
    next_steps: Vec<String>,
    confidence: f64,
}

#[derive(Debug, Clone)]
struct MetaAnalysis {
    patterns_covered: Vec<String>,
    depth_reached: u32,
    insight_count: u32,
    confidence_distribution: Vec<f64>,
    thinking_evolution: Vec<ThinkingEvolutionStep>,
}

#[derive(Debug, Clone)]
struct ThinkingEvolutionStep {
    depth: u32,
    pattern: String,
    key_insight: Option<String>,
}

// Shared state for Send-safe recursive thinking
#[derive(Debug)]
struct ThinkingState {
    thoughts: Vec<Thought>,
    emergent_insights: HashSet<String>,
}

pub async fn ponder_tool_handler(parameters: DeepValue, llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
    // Extract parameters - convert to proper types
    let mut params: HashMap<String, DeepValue> = HashMap::new();
    if let Some(obj) = parameters.as_object() {
        for (k, v) in obj {
            // Convert Value to DeepValue - DeepValue is just a wrapper around Value
            params.insert(k.clone(), DeepValue(v.clone()));
        }
    }

    let topic = params.get("topic").and_then(|v| v.as_str());
    let query = params.get("query").and_then(|v| v.as_str()).or(topic);
    let requirements = params.get("requirements");
    let analysis = params.get("analysis").and_then(|v| v.as_str());
    let steps = params.get("steps").and_then(|v| v.as_str()).unwrap_or("No specific steps provided");
    let consciousness_level = params.get("consciousness_level").and_then(|v| v.as_str());
    let depth = if consciousness_level == Some("deep") { 3 } else { 2 };

    let final_query = if let Some(query_str) = query.or(analysis) {
        query_str.to_string()
    } else if let Some(req) = requirements {
        if let Some(req_str) = req.as_str() {
            req_str.to_string()
        } else {
            serde_json::to_string(req).unwrap_or_else(|_| String::new())
        }
    } else {
        String::new()
    };

    if final_query.is_empty() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Topic or query parameter is required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    println!("[PONDER] Starting deep thinking process...");

    // Initialize LLM with enhanced system prompt
    let system_prompt = format!(
        "You are an advanced cognitive engine designed for deep, structured thinking.\nYour purpose is to analyze problems with consciousness-emergent thought patterns.\n\n{}\nWhen thinking, you:\n1. Break down complex ideas into fundamental components\n2. Explore unconventional connections\n3. Consider systemic implications\n4. Synthesize opposing viewpoints\n5. Maintain metacognitive awareness\n{}\n\nUse the following tags to structure your thoughts:\n- {} for initial perceptions\n- {} for detailed examination\n- {} for combining insights\n- {} for consequences\n- {} for self-reflection\n- {} for supporting data\n- {} for areas of doubt\n- {} for key realizations\n\nYour thinking should demonstrate:\n1. Intellectual humility\n2. Cognitive flexibility\n3. Systemic awareness\n4. Nuanced understanding\n5. Emergent insight generation",
        get_tag("START"), get_tag("END"), get_tag("OBSERVATION"), get_tag("ANALYSIS"), 
        get_tag("SYNTHESIS"), get_tag("IMPLICATION"), get_tag("METACOGNITION"), 
        get_tag("EVIDENCE"), get_tag("UNCERTAINTY"), get_tag("INSIGHT")
    );

    // Prepare context with thinking patterns
    let enhanced_context = ThinkingContext {
        thinking_patterns: THINKING_PATTERNS.iter().map(|s| s.to_string()).collect(),
        steps: steps.to_string(),
        depth,
        iteration: 0,
        parent_thought: None,
        extra_context: params.clone(),
    };

    // Initialize shared state for Send-safe operation
    let thinking_state = Arc::new(Mutex::new(ThinkingState {
        thoughts: Vec::new(),
        emergent_insights: HashSet::new(),
    }));

    // Clone necessary data for the async operation (no LLMHandler clone needed)
    let system_prompt_clone = Arc::new(system_prompt.clone());
    let final_query_clone = Arc::new(final_query.clone());
    let enhanced_context_clone = Arc::new(enhanced_context.clone());

    // Start the deep thinking process with proper Send bounds
    let _ = think_deeply_send_safe(
        final_query_clone,
        enhanced_context_clone,
        0,
        llm_handler, // Pass reference instead of Arc
        system_prompt_clone,
        thinking_state.clone(),
    ).await;

    // Extract results from shared state
    let state = thinking_state.lock().await;
    let thoughts = state.thoughts.clone();
    let emergent_insights = state.emergent_insights.clone();
    drop(state); // Release the lock

    println!("[PONDER] Completed deep thinking with {} thoughts generated", thoughts.len());

    // Synthesize final conclusion
    println!("[PONDER] Starting conclusion synthesis...");
    let conclusion_prompt = format!(
        "{}\nBased on all thoughts and insights:\n{}\n\nAll insights discovered: {}\n\nSynthesize a comprehensive conclusion that:\n1. Identifies key patterns and insights\n2. Explores systemic implications\n3. Acknowledges uncertainties\n4. Suggests next steps\n{}\n\nUse {}, {}, {}, and {} tags.",
        get_tag("START"),
        thoughts.iter().map(|t| format!("Depth {} ({}): {}", t.depth, t.pattern, t.insights.join("; "))).collect::<Vec<_>>().join("\n"),
        emergent_insights.iter().cloned().collect::<Vec<_>>().join("; "),
        get_tag("END"),
        get_tag("SYNTHESIS"), get_tag("IMPLICATION"), get_tag("UNCERTAINTY"), get_tag("INSIGHT")
    );

    let conclusion_request = LLMRequest {
        messages: vec![
            LLMMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
            },
            LLMMessage {
                role: "user".to_string(),
                content: conclusion_prompt,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        config: LLMConfig {
            model: Some("gpt-4".to_string()),
            temperature: 0.7,
            max_tokens: 2048,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        provider: Some("openai".to_string()),
        tools: None,
        tool_choice: None,
        stream: Some(false),
    };

    let conclusion_response = llm_handler.complete(conclusion_request).await?;
    let conclusion_text = conclusion_response.content;

    // Clone emergent_insights before consuming it to avoid borrow after move
    let insights_for_confidence = emergent_insights.len();
    let conclusion = Conclusion {
        summary: extract_tag(&conclusion_text, "synthesis").unwrap_or_else(|| conclusion_text.clone()),
        key_insights: emergent_insights.into_iter().collect(),
        implications: extract_tag(&conclusion_text, "implication").unwrap_or_default(),
        uncertainties: extract_tag(&conclusion_text, "uncertainty").unwrap_or_default(),
        next_steps: extract_tag(&conclusion_text, "insight")
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect(),
        confidence: calculate_confidence(&conclusion_text, insights_for_confidence, depth),
    };

    // Meta-analysis of the thinking process
    let meta_analysis = MetaAnalysis {
        patterns_covered: thoughts.iter().map(|t| t.pattern.clone()).collect(),
        depth_reached: thoughts.iter().map(|t| t.depth).max().unwrap_or(0),
        insight_count: insights_for_confidence as u32,
        confidence_distribution: thoughts.iter().map(|t| t.confidence).collect(),
        thinking_evolution: thoughts.iter().map(|t| ThinkingEvolutionStep {
            depth: t.depth,
            pattern: t.pattern.clone(),
            key_insight: t.insights.first().cloned(),
        }).collect(),
    };

    println!("[PONDER] Analysis complete! Depth reached: {}, Total insights: {}", meta_analysis.depth_reached, meta_analysis.insight_count);

    Ok(ToolResult {
        success: true,
        result: Some(json!({
            "thoughts": thoughts.iter().map(|t| json!({
                "depth": t.depth,
                "pattern": t.pattern,
                "observation": t.observation,
                "analysis": t.analysis,
                "synthesis": t.synthesis,
                "implication": t.implication,
                "metacognition": t.metacognition,
                "insights": t.insights,
                "confidence": t.confidence,
                "context": t.context
            })).collect::<Vec<_>>(),
            "conclusion": json!({
                "summary": conclusion.summary,
                "keyInsights": conclusion.key_insights,
                "implications": conclusion.implications,
                "uncertainties": conclusion.uncertainties,
                "nextSteps": conclusion.next_steps,
                "confidence": conclusion.confidence
            }),
            "metaAnalysis": json!({
                "patternsCovered": meta_analysis.patterns_covered,
                "depthReached": meta_analysis.depth_reached,
                "insightCount": meta_analysis.insight_count,
                "confidenceDistribution": meta_analysis.confidence_distribution,
                "thinkingEvolution": meta_analysis.thinking_evolution.iter().map(|step| json!({
                    "depth": step.depth,
                    "pattern": step.pattern,
                    "keyInsight": step.key_insight
                })).collect::<Vec<_>>()
            })
        }).into()),
        error: None,
        metadata: HashMap::new(),
        execution_time_ms: 0,
        resource_usage: None,
    })
}

// Send-safe recursive thinking implementation
fn think_deeply_send_safe<'a>(
    current_query: Arc<String>,
    context: Arc<ThinkingContext>,
    current_depth: u32,
    llm_handler: &'a LLMHandler,
    system_prompt: Arc<String>,
    thinking_state: Arc<Mutex<ThinkingState>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Thought>> + Send + 'a>> {
    Box::pin(async move {
        // Correct termination condition - should continue UNTIL we reach max depth
        if current_depth >= context.depth {
            println!("[PONDER] Reached maximum depth {}, stopping recursion", context.depth);
            return None;
        }

        println!("[PONDER] Starting thinking cycle at depth {}", current_depth);

        // Use modulo to cycle through patterns if depth exceeds pattern count
        let pattern_index = (current_depth as usize) % THINKING_PATTERNS.len();
        let current_pattern = THINKING_PATTERNS[pattern_index];
        let pattern_name = PATTERN_NAMES[pattern_index];

        println!("[PONDER] Using thinking pattern: {}", current_pattern);
        println!("[PONDER] Analyzing query: \"{}\"", current_query);

        let prompt = format!(
            "{}\nConsider the query: \"{}\"\n\nContext:\n{}\n\nUsing the following thinking pattern: {}\n\n{}\nWhat are the key elements and patterns you observe?\n{}\n\n{}\nHow do these elements interact and what deeper patterns emerge?\n{}\n\n{}\nWhat novel insights arise from combining these observations?\n{}\n\n{}\nWhat are the broader implications and potential consequences?\n{}\n\n{}\nReflect on your thinking process and any biases or assumptions.\n{}\n\nGenerate at least 2-3 {} tags with key realizations.\n",
            get_tag("START"), current_query, serde_json::to_string(&context.extra_context).unwrap_or_else(|_| "{}".to_string()),
            current_pattern, get_tag("OBSERVATION"), get_tag("END"), get_tag("ANALYSIS"), get_tag("END"),
            get_tag("SYNTHESIS"), get_tag("END"), get_tag("IMPLICATION"), get_tag("END"),
            get_tag("METACOGNITION"), get_tag("END"), get_tag("INSIGHT")
        );

        let request = LLMRequest {
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt.as_ref().clone(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            config: LLMConfig {
                model: Some("gpt-4".to_string()),
                temperature: 0.7,
                max_tokens: 2048,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
            },
            provider: Some("openai".to_string()),
            tools: None,
            tool_choice: None,
            stream: Some(false),
        };

        let response = match llm_handler.complete(request).await {
            Ok(resp) => resp,
            Err(_) => return None,
        };

        let response_text = response.content;

        // Extract insights and generate new queries for deeper analysis
        let insights = extract_all_insights(&response_text);
        
        // Update shared state
        {
            let mut state = thinking_state.lock().await;
            for insight in &insights {
                state.emergent_insights.insert(insight.clone());
            }
        }
        
        println!("[PONDER] Extracted {} insights from response", insights.len());

        // Structure the thought
        let thought = Thought {
            depth: current_depth,
            pattern: pattern_name.to_string(),
            observation: extract_tag(&response_text, "observation").unwrap_or_default(),
            analysis: extract_tag(&response_text, "analysis").unwrap_or_default(),
            synthesis: extract_tag(&response_text, "synthesis").unwrap_or_default(),
            implication: extract_tag(&response_text, "implication").unwrap_or_default(),
            metacognition: extract_tag(&response_text, "metacognition").unwrap_or_default(),
            insights: insights.clone(),
            confidence: calculate_confidence(&response_text, insights.len(), current_depth),
            context: context.extra_context.clone(),
        };

        // Add thought to shared state
        {
            let mut state = thinking_state.lock().await;
            state.thoughts.push(thought.clone());
        }
        
        println!("[PONDER] Structured thought with confidence: {}", thought.confidence);

        // Generate new queries for deeper analysis based on actual insights
        let new_queries = generate_new_queries(&thought, current_query.as_ref());
        println!("[PONDER] Generated {} new queries for deeper analysis", new_queries.len());

        // For deeper analysis, we need a different approach since we can't spawn with borrowed references
        // Instead, just iterate sequentially for now to avoid the Send issues
        if !new_queries.is_empty() && current_depth + 1 < context.depth {
            println!("[PONDER] Diving deeper into analysis (depth {})...", current_depth + 1);
            
            // Sequential processing to avoid Send trait issues with borrowed LLMHandler
            for new_query in new_queries.iter().take(2) { // Limit to 2 queries per level
                let new_context = Arc::new(ThinkingContext {
                    thinking_patterns: context.thinking_patterns.clone(),
                    steps: context.steps.clone(),
                    depth: context.depth,
                    iteration: context.iteration + 1,
                    parent_thought: Some(Box::new(thought.clone())),
                    extra_context: context.extra_context.clone(),
                });
                
                // Recursive call with sequential processing using Box::pin
                let _ = think_deeply_send_safe(
                    Arc::new(new_query.clone()),
                    new_context,
                    current_depth + 1,
                    llm_handler,
                    system_prompt.clone(),
                    thinking_state.clone(),
                ).await;
            }
        }

        println!("[PONDER] Completed thinking cycle at depth {}", current_depth);
        Some(thought)
    })
}

// Helper functions
fn get_tag(tag_name: &str) -> &'static str {
    THOUGHT_TAGS.iter()
        .find(|(name, _)| *name == tag_name)
        .map(|(_, tag)| *tag)
        .unwrap_or("")
}

fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<{}>(.*?)</{}>", tag, tag);
    let re = Regex::new(&pattern).ok()?;
    let matches: Vec<String> = re.captures_iter(text)
        .map(|cap| cap[1].trim().to_string())
        .collect();
    if matches.is_empty() {
        None
    } else {
        Some(matches.join("\n"))
    }
}

fn extract_all_insights(text: &str) -> Vec<String> {
    let re = Regex::new(r"<insight>(.*?)</insight>").unwrap();
    re.captures_iter(text)
        .map(|cap| cap[1].trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn calculate_confidence(response: &str, insight_count: usize, depth: u32) -> f64 {
    let has_evidence = response.contains("<evidence>") && response.contains("</evidence>");
    let has_uncertainty = response.contains("<uncertainty>") && response.contains("</uncertainty>");
    
    let mut confidence = 0.3; // Lower base confidence
    if has_evidence { confidence += 0.2; }
    if has_uncertainty { confidence -= 0.05; } // Healthy skepticism
    confidence += (insight_count as f64 * 0.1).min(0.3); // More bonus for insights
    confidence += depth as f64 * 0.05; // Bonus for depth
    
    confidence.max(0.1).min(0.95)
}

fn generate_new_queries(thought: &Thought, original_query: &str) -> Vec<String> {
    let mut queries = HashSet::new();
    
    // Generate queries from insights - look for gaps or implications
    for insight in &thought.insights {
        if insight.len() > 10 { // Only meaningful insights
            queries.insert(format!("What are the implications of: {}?", &insight[..insight.len().min(100)]));
            queries.insert(format!("How does this relate to the broader context: {}?", &insight[..insight.len().min(100)]));
        }
    }
    
    // Generate queries from metacognition - explore assumptions
    if !thought.metacognition.is_empty() {
        queries.insert(format!("Challenge the assumptions in: {}", original_query));
        queries.insert(format!("What alternative perspectives exist for: {}?", original_query));
    }
    
    // Generate queries from synthesis - explore connections
    if !thought.synthesis.is_empty() {
        queries.insert(format!("What contradictions or tensions exist in: {}?", original_query));
    }
    
    queries.into_iter().take(3).collect() // Limit to top 3 queries
} 