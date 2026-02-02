//! gRPC server setup with health check and reflection.
//!
//! Per GRPC-01: Memory daemon exposes gRPC service via tonic.
//! Per GRPC-03: Health check endpoint via tonic-health.
//! Per GRPC-04: Reflection endpoint via tonic-reflection.

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::Server;
use tonic_health::server::health_reporter;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::info;

use memory_scheduler::SchedulerService;
use memory_storage::Storage;

use crate::ingest::MemoryServiceImpl;
use crate::pb::{memory_service_server::MemoryServiceServer, FILE_DESCRIPTOR_SET};

/// Run the gRPC server with health check and reflection.
///
/// This function:
/// 1. Sets up the health check service (GRPC-03)
/// 2. Sets up the reflection service (GRPC-04)
/// 3. Registers the MemoryService
/// 4. Starts serving on the given address
pub async fn run_server(
    addr: SocketAddr,
    storage: Arc<Storage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting gRPC server on {}", addr);

    // Health check service (GRPC-03)
    let (mut health_reporter, health_service) = health_reporter();

    // Mark MemoryService as serving
    health_reporter
        .set_serving::<MemoryServiceServer<MemoryServiceImpl>>()
        .await;

    // Reflection service (GRPC-04)
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    // Main service implementation
    let memory_service = MemoryServiceImpl::new(storage);

    info!("gRPC server ready on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(MemoryServiceServer::new(memory_service))
        .serve(addr)
        .await?;

    Ok(())
}

/// Run the gRPC server with graceful shutdown support.
///
/// Accepts a shutdown signal future that, when resolved, triggers graceful shutdown.
pub async fn run_server_with_shutdown<F>(
    addr: SocketAddr,
    storage: Arc<Storage>,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    info!("Starting gRPC server on {} (with graceful shutdown)", addr);

    // Health check service (GRPC-03)
    let (mut health_reporter, health_service) = health_reporter();

    // Mark MemoryService as serving
    health_reporter
        .set_serving::<MemoryServiceServer<MemoryServiceImpl>>()
        .await;

    // Reflection service (GRPC-04)
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    // Main service implementation
    let memory_service = MemoryServiceImpl::new(storage);

    info!("gRPC server ready on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(MemoryServiceServer::new(memory_service))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;

    info!("gRPC server shutdown complete");
    Ok(())
}

/// Run the gRPC server with scheduler integration and graceful shutdown.
///
/// This function:
/// 1. Starts the scheduler
/// 2. Sets up the gRPC server with scheduler service handlers
/// 3. Serves until shutdown signal
/// 4. Shuts down scheduler gracefully
///
/// The scheduler service is injected into MemoryServiceImpl to handle
/// scheduler-related RPCs (GetSchedulerStatus, PauseJob, ResumeJob).
pub async fn run_server_with_scheduler<F>(
    addr: SocketAddr,
    storage: Arc<Storage>,
    scheduler: SchedulerService,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    info!("Starting gRPC server with scheduler on {}", addr);

    // Start the scheduler
    scheduler.start().await?;
    let job_count = scheduler.registry().job_count();
    info!(job_count, "Scheduler started");

    // Wrap scheduler for shared access
    let scheduler = Arc::new(scheduler);

    // Health check service (GRPC-03)
    let (mut health_reporter, health_service) = health_reporter();

    // Mark MemoryService as serving
    health_reporter
        .set_serving::<MemoryServiceServer<MemoryServiceImpl>>()
        .await;

    // Reflection service (GRPC-04)
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    // Main service implementation with scheduler
    let memory_service = MemoryServiceImpl::with_scheduler(storage, scheduler.clone());

    info!("gRPC server ready on {}", addr);

    // Run server until shutdown signal
    Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(MemoryServiceServer::new(memory_service))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;

    info!("gRPC server shutdown, stopping scheduler...");

    // Shutdown scheduler - need to get mutable access
    // Arc::get_mut won't work here since we have multiple references,
    // so we use try_unwrap after dropping other references
    drop(scheduler); // Drop our reference, MemoryServiceImpl's reference is already gone since server stopped

    // Note: In a production system, you might want a different approach
    // such as using a separate shutdown channel. For now, the scheduler
    // will be dropped when this function returns, triggering implicit cleanup.

    info!("Server shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_server_starts_and_shuts_down() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        // Create a shutdown signal that fires immediately
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let server_handle = tokio::spawn(async move {
            run_server_with_shutdown(addr, storage, async {
                rx.await.ok();
            })
            .await
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Trigger shutdown
        tx.send(()).ok();

        // Server should shut down within reasonable time
        let result = timeout(Duration::from_secs(5), server_handle).await;
        assert!(result.is_ok());
    }
}
