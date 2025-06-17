// Database Integration Demo for Aria Runtime
// Tests the database system integration with the engines

use aria_runtime::{
    AriaResult, AriaError,
    engines::AriaEngines,
    database::{DatabaseManager, DatabaseConfig},
};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> AriaResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 Aria Runtime Database Integration Demo");
    
    // Test 1: Database Manager Creation
    println!("📊 Test 1: Creating database manager...");
    let db_config = DatabaseConfig::default();
    println!("   Database path: {:?}", db_config.system_db_path);
    
    let db_manager = DatabaseManager::new(db_config);
    println!("   ✅ Database manager created");
    
    // Test 2: Database Initialization
    println!("📊 Test 2: Initializing database system...");
    db_manager.initialize().await?;
    println!("   ✅ Database system initialized");
    
    // Test 3: System Database Access
    println!("📊 Test 3: Testing system database access...");
    let system_pool = db_manager.get_system_database().await?;
    println!("   ✅ System database pool obtained");
    
    // Test 4: User Database Creation
    println!("📊 Test 4: Testing user database creation...");
    let user_id = "test_user_001";
    let user_pool = db_manager.get_user_database(user_id).await?;
    println!("   ✅ User database created for: {}", user_id);
    
    // Test 5: Database Operations
    println!("📊 Test 5: Testing database operations...");
    
    // Create a test user
    use aria_runtime::database::users::UserOps;
    UserOps::create_user(&system_pool, user_id, "Test User", Some("test@example.com".to_string())).await?;
    println!("   ✅ User created in system database");
    
    // Get the user back
    let user_record = UserOps::get_user(&system_pool, user_id).await?;
    println!("   ✅ User retrieved: {} ({})", user_record.username, user_record.user_id);
    
    // Create a session
    use aria_runtime::database::sessions::SessionOps;
    let session_id = SessionOps::create_session(&user_pool, user_id, "demo", None).await?;
    println!("   ✅ Session created: {}", session_id);
    
    // Create an async task
    use aria_runtime::database::async_tasks::AsyncTaskOps;
    let task_id = AsyncTaskOps::create_task(
        &user_pool,
        user_id,
        &session_id,
        "demo_task",
        vec!["echo".to_string(), "Hello World".to_string()],
        HashMap::new(),
        None,
    ).await?;
    println!("   ✅ Async task created: {}", task_id);
    
    // Update task status
    use aria_runtime::database::async_tasks::AsyncTaskStatus;
    AsyncTaskOps::update_task_status(
        &user_pool,
        &task_id,
        AsyncTaskStatus::Completed,
        Some(0),
        Some("Hello World".to_string()),
        None,
    ).await?;
    println!("   ✅ Task status updated to completed");
    
    // Get task back
    let task_record = AsyncTaskOps::get_task(&user_pool, &task_id).await?;
    println!("   ✅ Task retrieved: {} - Status: {:?}", task_record.task_id, task_record.status);
    
    // Test 6: Audit Logging
    println!("📊 Test 6: Testing audit logging...");
    use aria_runtime::database::audit::AuditOps;
    AuditOps::log_event(
        &user_pool,
        Some(user_id.to_string()),
        Some(session_id.clone()),
        "demo_event",
        Some(serde_json::json!({
            "action": "database_test",
            "timestamp": chrono::Utc::now().timestamp()
        })),
        "info",
    ).await?;
    println!("   ✅ Audit log entry created");
    
    // Test 7: AriaEngines Integration
    println!("📊 Test 7: Testing AriaEngines with database integration...");
    let engines = AriaEngines::new().await;
    println!("   ✅ AriaEngines created with database integration");
    
    // Verify database is available in engines
    let db_stats = engines.database.get_stats().await?;
    println!("   📈 Database stats: {} system connections, {} user databases", 
             db_stats.system_connections, db_stats.user_databases);
    
    // Test 8: Database Statistics
    println!("📊 Test 8: Testing database statistics...");
    let stats = db_manager.get_stats().await?;
    println!("   📈 Total connections: {}", stats.total_connections);
    println!("   📈 User databases: {}", stats.user_databases);
    println!("   📈 System connections: {}", stats.system_connections);
    
    // Test 9: Cleanup
    println!("📊 Test 9: Testing graceful shutdown...");
    db_manager.shutdown().await?;
    println!("   ✅ Database system shutdown completed");
    
    println!();
    println!("🎉 All database integration tests passed!");
    println!("✅ Database system is ready for production use");
    
    Ok(())
} 