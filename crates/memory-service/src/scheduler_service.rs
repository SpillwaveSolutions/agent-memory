//! gRPC scheduler status service implementation.
//!
//! Per SCHED-05: Job status observable via gRPC.
//!
//! This module provides gRPC handlers for scheduler status and control:
//! - GetSchedulerStatus: Returns scheduler running state and all job statuses
//! - PauseJob: Pauses a scheduled job
//! - ResumeJob: Resumes a paused job

use std::sync::Arc;

use tonic::{Request, Response, Status};

use memory_scheduler::{JobRegistry, JobResult, SchedulerService};

use crate::pb::{
    GetSchedulerStatusRequest, GetSchedulerStatusResponse, JobResultStatus, JobStatusProto,
    PauseJobRequest, PauseJobResponse, ResumeJobRequest, ResumeJobResponse,
};

/// Convert domain JobResult to proto enum and error message.
fn job_result_to_proto(result: &JobResult) -> (JobResultStatus, Option<String>) {
    match result {
        JobResult::Success => (JobResultStatus::Success, None),
        JobResult::Failed(msg) => (JobResultStatus::Failed, Some(msg.clone())),
        JobResult::Skipped(msg) => (JobResultStatus::Skipped, Some(msg.clone())),
    }
}

/// gRPC service for scheduler status and control.
///
/// This service wraps a `SchedulerService` and provides gRPC handlers
/// for querying scheduler status and controlling job execution.
///
/// # Example
///
/// ```ignore
/// use memory_scheduler::SchedulerService;
/// use memory_service::SchedulerGrpcService;
///
/// let scheduler = Arc::new(SchedulerService::new(config).await?);
/// let grpc_service = SchedulerGrpcService::new(scheduler);
///
/// // Use in MemoryServiceImpl
/// let response = grpc_service.get_scheduler_status(request).await?;
/// ```
pub struct SchedulerGrpcService {
    scheduler: Arc<SchedulerService>,
}

impl SchedulerGrpcService {
    /// Create a new SchedulerGrpcService with the given scheduler.
    pub fn new(scheduler: Arc<SchedulerService>) -> Self {
        Self { scheduler }
    }

    /// Get the job registry.
    pub fn registry(&self) -> Arc<JobRegistry> {
        self.scheduler.registry()
    }

    /// Get scheduler and job status.
    ///
    /// Returns the scheduler running state and status of all registered jobs.
    pub async fn get_scheduler_status(
        &self,
        _request: Request<GetSchedulerStatusRequest>,
    ) -> Result<Response<GetSchedulerStatusResponse>, Status> {
        let registry = self.scheduler.registry();
        let statuses = registry.get_all_status();

        let jobs: Vec<JobStatusProto> = statuses
            .into_iter()
            .map(|s| {
                let (result_status, error) = s
                    .last_result
                    .as_ref()
                    .map(job_result_to_proto)
                    .unwrap_or((JobResultStatus::Unspecified, None));

                JobStatusProto {
                    job_name: s.job_name,
                    cron_expr: s.cron_expr,
                    last_run_ms: s.last_run.map(|t| t.timestamp_millis()).unwrap_or(0),
                    last_duration_ms: s.last_duration_ms.unwrap_or(0) as i64,
                    last_result: result_status.into(),
                    last_error: error,
                    next_run_ms: s.next_run.map(|t| t.timestamp_millis()).unwrap_or(0),
                    run_count: s.run_count,
                    error_count: s.error_count,
                    is_running: s.is_running,
                    is_paused: s.is_paused,
                }
            })
            .collect();

        Ok(Response::new(GetSchedulerStatusResponse {
            scheduler_running: self.scheduler.is_running(),
            jobs,
        }))
    }

    /// Pause a scheduled job.
    ///
    /// The job will skip execution when its scheduled time arrives.
    pub async fn pause_job(
        &self,
        request: Request<PauseJobRequest>,
    ) -> Result<Response<PauseJobResponse>, Status> {
        let job_name = &request.get_ref().job_name;

        match self.scheduler.pause_job(job_name) {
            Ok(()) => Ok(Response::new(PauseJobResponse {
                success: true,
                error: None,
            })),
            Err(e) => Ok(Response::new(PauseJobResponse {
                success: false,
                error: Some(e.to_string()),
            })),
        }
    }

    /// Resume a paused job.
    ///
    /// The job will resume executing at its next scheduled time.
    pub async fn resume_job(
        &self,
        request: Request<ResumeJobRequest>,
    ) -> Result<Response<ResumeJobResponse>, Status> {
        let job_name = &request.get_ref().job_name;

        match self.scheduler.resume_job(job_name) {
            Ok(()) => Ok(Response::new(ResumeJobResponse {
                success: true,
                error: None,
            })),
            Err(e) => Ok(Response::new(ResumeJobResponse {
                success: false,
                error: Some(e.to_string()),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_scheduler::{JitterConfig, OverlapPolicy, SchedulerConfig, TimeoutConfig};

    async fn create_test_scheduler() -> Arc<SchedulerService> {
        let config = SchedulerConfig::default();
        Arc::new(SchedulerService::new(config).await.unwrap())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_scheduler_status_empty() {
        let scheduler = create_test_scheduler().await;
        let service = SchedulerGrpcService::new(scheduler);

        let request = Request::new(GetSchedulerStatusRequest {});
        let response = service.get_scheduler_status(request).await.unwrap();
        let resp = response.into_inner();

        assert!(!resp.scheduler_running);
        assert!(resp.jobs.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_scheduler_status_with_jobs() {
        let scheduler = create_test_scheduler().await;

        // Register a job
        scheduler
            .register_job(
                "test-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        let service = SchedulerGrpcService::new(scheduler);

        let request = Request::new(GetSchedulerStatusRequest {});
        let response = service.get_scheduler_status(request).await.unwrap();
        let resp = response.into_inner();

        assert_eq!(resp.jobs.len(), 1);
        assert_eq!(resp.jobs[0].job_name, "test-job");
        assert_eq!(resp.jobs[0].cron_expr, "0 0 * * * *");
        assert!(!resp.jobs[0].is_paused);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pause_job_success() {
        let scheduler = create_test_scheduler().await;

        scheduler
            .register_job(
                "pause-test",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        let service = SchedulerGrpcService::new(scheduler.clone());

        let request = Request::new(PauseJobRequest {
            job_name: "pause-test".to_string(),
        });
        let response = service.pause_job(request).await.unwrap();
        let resp = response.into_inner();

        assert!(resp.success);
        assert!(resp.error.is_none());
        assert!(scheduler.registry().is_paused("pause-test"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pause_job_not_found() {
        let scheduler = create_test_scheduler().await;
        let service = SchedulerGrpcService::new(scheduler);

        let request = Request::new(PauseJobRequest {
            job_name: "nonexistent".to_string(),
        });
        let response = service.pause_job(request).await.unwrap();
        let resp = response.into_inner();

        assert!(!resp.success);
        assert!(resp.error.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_resume_job_success() {
        let scheduler = create_test_scheduler().await;

        scheduler
            .register_job(
                "resume-test",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        // Pause first
        scheduler.pause_job("resume-test").unwrap();
        assert!(scheduler.registry().is_paused("resume-test"));

        let service = SchedulerGrpcService::new(scheduler.clone());

        let request = Request::new(ResumeJobRequest {
            job_name: "resume-test".to_string(),
        });
        let response = service.resume_job(request).await.unwrap();
        let resp = response.into_inner();

        assert!(resp.success);
        assert!(resp.error.is_none());
        assert!(!scheduler.registry().is_paused("resume-test"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_resume_job_not_found() {
        let scheduler = create_test_scheduler().await;
        let service = SchedulerGrpcService::new(scheduler);

        let request = Request::new(ResumeJobRequest {
            job_name: "nonexistent".to_string(),
        });
        let response = service.resume_job(request).await.unwrap();
        let resp = response.into_inner();

        assert!(!resp.success);
        assert!(resp.error.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_job_result_to_proto() {
        let (status, error) = job_result_to_proto(&JobResult::Success);
        assert_eq!(status, JobResultStatus::Success);
        assert!(error.is_none());

        let (status, error) = job_result_to_proto(&JobResult::Failed("timeout".to_string()));
        assert_eq!(status, JobResultStatus::Failed);
        assert_eq!(error, Some("timeout".to_string()));

        let (status, error) = job_result_to_proto(&JobResult::Skipped("overlap".to_string()));
        assert_eq!(status, JobResultStatus::Skipped);
        assert_eq!(error, Some("overlap".to_string()));
    }
}
