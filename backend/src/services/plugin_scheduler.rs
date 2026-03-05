use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

pub type ScheduleTaskId = String;
pub type ScheduleCallbackFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
pub type ScheduleCallback =
    Arc<dyn Fn(ScheduleTriggerContext) -> ScheduleCallbackFuture + Send + Sync + 'static>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub cron: String,
    #[serde(default)]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScheduleRegistration {
    pub plugin_id: String,
    pub task_name: String,
    pub spec: ScheduleSpec,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerReason {
    Manual,
    ScheduleTick,
}

#[derive(Debug, Clone)]
pub struct ScheduleTriggerContext {
    pub task_id: ScheduleTaskId,
    pub plugin_id: String,
    pub task_name: String,
    pub triggered_at: DateTime<Utc>,
    pub reason: TriggerReason,
}

#[derive(Debug, Clone)]
pub struct ScheduleTaskSummary {
    pub task_id: ScheduleTaskId,
    pub plugin_id: String,
    pub task_name: String,
    pub cron: String,
    pub timezone: Option<String>,
}

struct RegisteredTask {
    registration: ScheduleRegistration,
    callback: ScheduleCallback,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginSchedulerError {
    #[error("非法 cron 表达式: {0}")]
    InvalidCron(String),
    #[error("非法时区: {0}")]
    InvalidTimezone(String),
    #[error("调度任务不存在: {0}")]
    TaskNotFound(String),
}

pub struct PluginScheduler {
    next_task_id: AtomicU64,
    tasks: RwLock<HashMap<ScheduleTaskId, RegisteredTask>>,
}

impl Default for PluginScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginScheduler {
    pub fn new() -> Self {
        Self {
            next_task_id: AtomicU64::new(1),
            tasks: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(
        &self,
        registration: ScheduleRegistration,
        callback: ScheduleCallback,
    ) -> Result<ScheduleTaskId, PluginSchedulerError> {
        Self::validate_registration(&registration)?;
        let task_id = format!(
            "sched-{}",
            self.next_task_id.fetch_add(1, Ordering::Relaxed)
        );

        let mut guard = self.tasks.write().await;
        guard.insert(
            task_id.clone(),
            RegisteredTask {
                registration,
                callback,
            },
        );

        Ok(task_id)
    }

    pub async fn cancel(&self, task_id: &str) -> bool {
        let mut guard = self.tasks.write().await;
        guard.remove(task_id).is_some()
    }

    pub async fn trigger_now(&self, task_id: &str) -> Result<(), PluginSchedulerError> {
        self.trigger(task_id, TriggerReason::Manual).await
    }

    pub async fn trigger(
        &self,
        task_id: &str,
        reason: TriggerReason,
    ) -> Result<(), PluginSchedulerError> {
        let (registration, callback) = {
            let guard = self.tasks.read().await;
            let task = guard
                .get(task_id)
                .ok_or_else(|| PluginSchedulerError::TaskNotFound(task_id.to_string()))?;
            (task.registration.clone(), task.callback.clone())
        };

        let ctx = ScheduleTriggerContext {
            task_id: task_id.to_string(),
            plugin_id: registration.plugin_id,
            task_name: registration.task_name,
            triggered_at: Utc::now(),
            reason,
        };

        (callback)(ctx).await;
        Ok(())
    }

    pub async fn trigger_due_tasks(&self) -> Result<usize, PluginSchedulerError> {
        // TODO(P7-8): compute due tasks from cron + timezone instead of triggering all tasks.
        let task_ids = {
            let guard = self.tasks.read().await;
            guard.keys().cloned().collect::<Vec<_>>()
        };

        for task_id in &task_ids {
            self.trigger(task_id, TriggerReason::ScheduleTick).await?;
        }

        Ok(task_ids.len())
    }

    pub async fn unregister_plugin(&self, plugin_id: &str) -> usize {
        let mut removed = 0;
        let mut guard = self.tasks.write().await;

        let keys = guard
            .iter()
            .filter_map(|(task_id, task)| {
                if task.registration.plugin_id == plugin_id {
                    Some(task_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for key in keys {
            if guard.remove(&key).is_some() {
                removed += 1;
            }
        }

        removed
    }

    pub async fn list_tasks(&self) -> Vec<ScheduleTaskSummary> {
        let guard = self.tasks.read().await;
        let mut items = guard
            .iter()
            .map(|(task_id, task)| ScheduleTaskSummary {
                task_id: task_id.clone(),
                plugin_id: task.registration.plugin_id.clone(),
                task_name: task.registration.task_name.clone(),
                cron: task.registration.spec.cron.clone(),
                timezone: task.registration.spec.timezone.clone(),
            })
            .collect::<Vec<_>>();

        items.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        items
    }

    fn validate_registration(
        registration: &ScheduleRegistration,
    ) -> Result<(), PluginSchedulerError> {
        if Schedule::from_str(&registration.spec.cron).is_err() {
            return Err(PluginSchedulerError::InvalidCron(
                registration.spec.cron.clone(),
            ));
        }

        if let Some(timezone) = &registration.spec.timezone {
            if timezone.trim().is_empty() {
                return Err(PluginSchedulerError::InvalidTimezone(timezone.clone()));
            }
            // TODO(P7-8): validate against IANA timezone db once timezone runtime is integrated.
        }

        Ok(())
    }
}
