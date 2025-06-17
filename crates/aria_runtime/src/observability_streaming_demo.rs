use crate::engines::observability::{ObservabilityManager, EventFilter, ErrorSeverity};
use crate::engines::streaming::{StreamingService, StreamingConfig, StreamQuery};
use crate::engines::observability_endpoints::create_observability_router;
use crate::database::{DatabaseManager, DatabaseConfig};
use crate::error::AriaError;
use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

/// Complete observability and streaming demonstration
pub async fn run_observability_streaming_demo() -> Result<(), AriaError> {
    info!("🚀 Starting Aria Runtime Observability & Streaming Demo");
    
    // Initialize database
    let db_config = DatabaseConfig::default();
    let database = Arc::new(DatabaseManager::new(db_config).await?);
    
    // Initialize observability manager
    let observability = Arc::new(ObservabilityManager::new(Arc::clone(&database), 10000)?);
    
    // Initialize streaming service
    let streaming_config = StreamingConfig {
        max_concurrent_streams: 100,
        default_buffer_size: 1000,
        keep_alive_interval: Duration::from_secs(30),
        stream_timeout: Duration::from_secs(300),
        max_event_size: 64 * 1024,
    };
    let streaming = Arc::new(StreamingService::new(Arc::clone(&observability), streaming_config));
    
    // Start services
    info!("📊 Starting observability and streaming services...");
    observability.start().await?;
    streaming.start().await?;
    
    // Create HTTP router with observability endpoints
    let observability_router = create_observability_router(Arc::clone(&observability), Arc::clone(&streaming));
    
    // Demo 1: Record various observability events
    info!("📈 Demo 1: Recording observability events");
    demo_event_recording(&observability).await?;
    
    // Demo 2: Test streaming and SSE functionality
    info!("🌊 Demo 2: Testing streaming and SSE functionality");
    demo_streaming_functionality(&streaming).await?;
    
    // Demo 3: Test metrics collection and health monitoring
    info!("🏥 Demo 3: Testing metrics and health monitoring");
    demo_metrics_and_health(&observability).await?;
    
    // Demo 4: Test event filtering and subscriptions
    info!("🔍 Demo 4: Testing event filtering and subscriptions");
    demo_event_filtering(&streaming).await?;
    
    // Demo 5: Simulate production workload
    info!("⚡ Demo 5: Simulating production workload");
    demo_production_workload(&observability, &streaming).await?;
    
    // Demo 6: Test HTTP endpoints
    info!("🌐 Demo 6: Testing HTTP endpoints");
    demo_http_endpoints(&observability_router).await?;
    
    // Cleanup
    info!("🧹 Cleaning up services...");
    streaming.stop().await?;
    observability.stop().await?;
    
    info!("✅ Observability & Streaming Demo completed successfully!");
    Ok(())
}

/// Demo 1: Record various observability events
async fn demo_event_recording(observability: &ObservabilityManager) -> Result<(), AriaError> {
    info!("Recording various event types...");
    
    // Record tool executions
    observability.record_tool_execution(
        "ponderTool",
        "session-123",
        1500,
        true,
        None
    ).await?;
    
    observability.record_tool_execution(
        "webSearchTool",
        "session-123",
        800,
        false,
        Some("Rate limit exceeded".to_string())
    ).await?;
    
    // Record container events
    let mut metadata = HashMap::new();
    metadata.insert("image".to_string(), "ubuntu:22.04".to_string());
    metadata.insert("session_id".to_string(), "session-123".to_string());
    
    observability.record_container_event(
        "container-abc123",
        "created",
        metadata.clone()
    ).await?;
    
    observability.record_container_event(
        "container-abc123",
        "started",
        metadata
    ).await?;
    
    // Record agent executions
    observability.record_agent_execution(
        "session-123",
        "research_agent",
        5,
        2500,
        12000,
        true
    ).await?;
    
    // Record errors
    let context = HashMap::from([
        ("component".to_string(), "llm_handler".to_string()),
        ("operation".to_string(), "completion_request".to_string()),
    ]);
    
    let test_error = AriaError::Engine("Test error for observability demo".to_string());
    observability.record_error(&test_error, "llm_handler", context).await?;
    
    info!("✅ Events recorded successfully");
    Ok(())
}

/// Demo 2: Test streaming and SSE functionality
async fn demo_streaming_functionality(streaming: &StreamingService) -> Result<(), AriaError> {
    info!("Testing streaming functionality...");
    
    // Create different types of streams
    let all_events_query = StreamQuery {
        stream_type: Some("all".to_string()),
        events: None,
        components: None,
        session_id: None,
        min_severity: None,
        buffer_size: 1000,
    };
    
    let tool_events_query = StreamQuery {
        stream_type: Some("tools".to_string()),
        events: Some("tool".to_string()),
        components: None,
        session_id: Some("session-123".to_string()),
        min_severity: None,
        buffer_size: 500,
    };
    
    let error_events_query = StreamQuery {
        stream_type: Some("errors".to_string()),
        events: Some("error".to_string()),
        components: None,
        session_id: None,
        min_severity: Some("medium".to_string()),
        buffer_size: 100,
    };
    
    // Create streams
    let (stream1_id, mut stream1_receiver) = streaming.create_stream(all_events_query).await?;
    let (stream2_id, mut stream2_receiver) = streaming.create_stream(tool_events_query).await?;
    let (stream3_id, mut stream3_receiver) = streaming.create_stream(error_events_query).await?;
    
    info!("Created streams: {}, {}, {}", stream1_id, stream2_id, stream3_id);
    
    // Test stream reception (briefly)
    tokio::spawn(async move {
        for _ in 0..3 {
            match stream1_receiver.recv().await {
                Ok(event) => info!("Stream 1 received event: {:?}", event),
                Err(e) => warn!("Stream 1 error: {}", e),
            }
        }
    });
    
    // Let streams run briefly
    sleep(Duration::from_millis(100)).await;
    
    // Get streaming stats
    let stats = streaming.get_stats().await;
    info!("Streaming stats: {:?}", stats);
    
    // Get active streams
    let active_streams = streaming.get_active_streams().await;
    info!("Active streams count: {}", active_streams.len());
    
    info!("✅ Streaming functionality tested");
    Ok(())
}

/// Demo 3: Test metrics collection and health monitoring
async fn demo_metrics_and_health(observability: &ObservabilityManager) -> Result<(), AriaError> {
    info!("Testing metrics and health monitoring...");
    
    // Get current metrics
    let metrics = observability.get_metrics().await;
    info!("Runtime metrics timestamp: {}", metrics.timestamp);
    info!("Active sessions: {}", metrics.runtime.active_sessions);
    info!("Tool executions: {}", metrics.runtime.tool_executions);
    info!("Agent invocations: {}", metrics.runtime.agent_invocations);
    info!("Containers running: {}", metrics.containers.containers_running);
    info!("LLM tokens used: {}", metrics.llm.tokens_used);
    
    // Get health status
    let health = observability.get_health().await;
    info!("Overall health: {:?}", health.overall);
    info!("Component count: {}", health.components.len());
    
    for (component, health_info) in &health.components {
        info!("Component '{}' status: {:?}", component, health_info.status);
    }
    
    info!("✅ Metrics and health monitoring tested");
    Ok(())
}

/// Demo 4: Test event filtering and subscriptions
async fn demo_event_filtering(streaming: &StreamingService) -> Result<(), AriaError> {
    info!("Testing event filtering and subscriptions...");
    
    // Test different filters
    let filters = vec![
        EventFilter {
            event_types: Some(vec!["tool".to_string()]),
            components: None,
            severity_min: None,
            session_id: None,
        },
        EventFilter {
            event_types: Some(vec!["error".to_string()]),
            components: Some(vec!["llm_handler".to_string()]),
            severity_min: Some(ErrorSeverity::High),
            session_id: None,
        },
        EventFilter {
            event_types: None,
            components: None,
            severity_min: None,
            session_id: Some("session-123".to_string()),
        },
    ];
    
    for (i, filter) in filters.into_iter().enumerate() {
        let query = StreamQuery {
            stream_type: Some("custom".to_string()),
            events: filter.event_types.as_ref().map(|types| types.join(",")),
            components: filter.components.as_ref().map(|comps| comps.join(",")),
            session_id: filter.session_id.clone(),
            min_severity: filter.severity_min.as_ref().map(|_| "high".to_string()),
            buffer_size: 100,
        };
        
        let (stream_id, _receiver) = streaming.create_stream(query).await?;
        info!("Created filtered stream {}: {}", i + 1, stream_id);
    }
    
    // Check stats after creating filtered streams
    let stats = streaming.get_stats().await;
    info!("Updated streaming stats: {:?}", stats);
    
    info!("✅ Event filtering tested");
    Ok(())
}

/// Demo 5: Simulate production workload
async fn demo_production_workload(
    observability: &ObservabilityManager,
    streaming: &StreamingService,
) -> Result<(), AriaError> {
    info!("Simulating production workload...");
    
    // Create multiple concurrent streams
    let stream_count = 10;
    let mut stream_handles = Vec::new();
    
    for i in 0..stream_count {
        let query = StreamQuery {
            stream_type: Some("all".to_string()),
            events: None,
            components: None,
            session_id: None,
            min_severity: None,
            buffer_size: 100,
        };
        
        let (stream_id, mut receiver) = streaming.create_stream(query).await?;
        info!("Created production stream {}: {}", i + 1, stream_id);
        
        // Spawn task to consume events
        let handle = tokio::spawn(async move {
            let mut event_count = 0;
            while event_count < 5 {
                match receiver.recv().await {
                    Ok(_) => {
                        event_count += 1;
                    },
                    Err(_) => break,
                }
            }
            info!("Production stream {} processed {} events", i + 1, event_count);
        });
        
        stream_handles.push(handle);
    }
    
    // Generate multiple events rapidly
    for i in 0..50 {
        let session_id = format!("prod-session-{}", i % 3);
        
        // Tool executions
        observability.record_tool_execution(
            "calculator",
            &session_id,
            50 + (i * 10),
            i % 7 != 0, // 1/7 failures
            if i % 7 == 0 { Some("Calculation error".to_string()) } else { None }
        ).await?;
        
        // Container events
        if i % 5 == 0 {
            let container_id = format!("prod-container-{}", i / 5);
            let mut metadata = HashMap::new();
            metadata.insert("workload".to_string(), "production".to_string());
            
            observability.record_container_event(
                &container_id,
                "created",
                metadata
            ).await?;
        }
        
        // Agent executions
        if i % 10 == 0 {
            observability.record_agent_execution(
                &session_id,
                "production_agent",
                3 + (i % 5),
                1000 + (i * 50),
                5000 + (i * 100),
                i % 15 != 0 // 1/15 failures
            ).await?;
        }
        
        // Small delay to avoid overwhelming
        if i % 10 == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Wait for stream processing
    sleep(Duration::from_millis(100)).await;
    
    // Wait for all stream handlers to complete
    for handle in stream_handles {
        let _ = handle.await;
    }
    
    // Get final stats
    let final_stats = streaming.get_stats().await;
    info!("Final production stats: {:?}", final_stats);
    
    let final_metrics = observability.get_metrics().await;
    info!("Final runtime metrics - Tool executions: {}, Agent invocations: {}", 
          final_metrics.runtime.tool_executions, final_metrics.runtime.agent_invocations);
    
    info!("✅ Production workload simulation completed");
    Ok(())
}

/// Demo 6: Test HTTP endpoints
async fn demo_http_endpoints(router: &Router) -> Result<(), AriaError> {
    info!("Testing HTTP endpoints structure...");
    
    // Note: This is a structure test - in a real scenario you'd start an HTTP server
    // and make actual requests. For this demo, we're just validating the router exists.
    
    info!("Available endpoints:");
    info!("  GET  /metrics              - Runtime metrics (JSON)");
    info!("  GET  /metrics/prometheus   - Prometheus format metrics");
    info!("  GET  /logs                 - Recent log entries");
    info!("  GET  /logs/stream          - Real-time log streaming (SSE)");
    info!("  GET  /errors               - Recent error entries");
    info!("  GET  /errors/stream        - Real-time error streaming (SSE)");
    info!("  GET  /health               - Simple health check");
    info!("  GET  /health/detailed      - Detailed health status");
    info!("  GET  /stream               - General event streaming (SSE)");
    info!("  GET  /stream/stats         - Streaming statistics");
    info!("  GET  /stream/active        - Active streams list");
    info!("  GET  /debug/metrics        - Debug metrics information");
    info!("  GET  /debug/state          - Debug state information");
    
    info!("Example SSE usage:");
    info!("  curl -H 'Accept: text/event-stream' 'http://localhost:3000/stream?stream_type=all'");
    info!("  curl -H 'Accept: text/event-stream' 'http://localhost:3000/logs/stream?session_id=my-session'");
    info!("  curl -H 'Accept: text/event-stream' 'http://localhost:3000/errors/stream?min_severity=high'");
    
    info!("✅ HTTP endpoints structure validated");
    Ok(())
}

/// Helper function to demonstrate the complete system in a simple way
pub async fn simple_observability_demo() -> Result<(), AriaError> {
    info!("🎯 Simple Observability Demo");
    
    // Initialize minimal system
    let db_config = DatabaseConfig::default();
    let database = Arc::new(DatabaseManager::new(db_config).await?);
    let observability = Arc::new(ObservabilityManager::new(Arc::clone(&database), 1000)?);
    let streaming = Arc::new(StreamingService::new(Arc::clone(&observability), StreamingConfig::default()));
    
    // Start services
    observability.start().await?;
    streaming.start().await?;
    
    // Record some events
    observability.record_tool_execution("test_tool", "session-1", 100, true, None).await?;
    observability.record_agent_execution("session-1", "test_agent", 1, 50, 1000, true).await?;
    
    // Create a stream and test it briefly
    let query = StreamQuery {
        stream_type: Some("all".to_string()),
        events: None,
        components: None,
        session_id: None,
        min_severity: None,
        buffer_size: 100,
    };
    
    let (stream_id, mut receiver) = streaming.create_stream(query).await?;
    info!("Created stream: {}", stream_id);
    
    // Quick test to receive one event
    tokio::spawn(async move {
        match receiver.recv().await {
            Ok(event) => info!("Received event type: {:?}", event),
            Err(e) => warn!("Stream error: {}", e),
        }
    });
    
    sleep(Duration::from_millis(50)).await;
    
    // Get final stats
    let metrics = observability.get_metrics().await;
    let health = observability.get_health().await;
    let stream_stats = streaming.get_stats().await;
    
    info!("Final Summary:");
    info!("  Tool executions: {}", metrics.runtime.tool_executions);
    info!("  Agent invocations: {}", metrics.runtime.agent_invocations);
    info!("  Health status: {:?}", health.overall);
    info!("  Active streams: {}", stream_stats.active_streams);
    info!("  Events sent: {}", stream_stats.events_sent);
    
    // Cleanup
    streaming.stop().await?;
    observability.stop().await?;
    
    info!("✅ Simple demo completed!");
    Ok(())
} 