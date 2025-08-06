use crate::config::Config;
use crate::monitor::{MonitorEngine, ServiceCheck};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug)]
pub struct App {
    pub config: Config,
    pub monitor_engine: MonitorEngine,
    pub statuses: HashMap<String, ServiceCheck>,
    pub selected_index: usize,
    pub show_help: bool,
    pub last_update: chrono::DateTime<Utc>,
}

impl App {
    pub fn new(config: Config, monitor_engine: MonitorEngine) -> Self {
        Self {
            config,
            monitor_engine,
            statuses: HashMap::new(),
            selected_index: 0,
            show_help: false,
            last_update: Utc::now(),
        }
    }

    pub async fn update_statuses(&mut self) {
        self.statuses = self.monitor_engine.get_statuses().await;
        self.last_update = Utc::now();
    }

    pub fn get_status_list(&self) -> Vec<ServiceCheck> {
        let mut statuses: Vec<_> = self.statuses.values().cloned().collect();
        statuses.sort_by(|a, b| {
            a.host_name
                .cmp(&b.host_name)
                .then(a.service_name.cmp(&b.service_name))
        });
        statuses
    }

    pub fn get_selected_service(&self) -> Option<ServiceCheck> {
        let statuses = self.get_status_list();
        statuses.get(self.selected_index).cloned()
    }

    pub fn next_item(&mut self) {
        let statuses = self.get_status_list();
        if !statuses.is_empty() {
            self.selected_index = (self.selected_index + 1) % statuses.len();
        }
    }

    pub fn previous_item(&mut self) {
        let statuses = self.get_status_list();
        if !statuses.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                statuses.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn get_summary_stats(&self) -> (usize, usize, usize) {
        let mut up = 0;
        let mut down = 0;
        let mut unknown = 0;

        for status in self.statuses.values() {
            match status.status {
                crate::monitor::ServiceStatus::Up => up += 1,
                crate::monitor::ServiceStatus::Down => down += 1,
                crate::monitor::ServiceStatus::Unknown => unknown += 1,
            }
        }

        (up, down, unknown)
    }

    pub fn get_total_services(&self) -> usize {
        self.statuses.len()
    }

    pub fn get_host_count(&self) -> usize {
        self.config.hosts.len()
    }

    pub fn get_refresh_interval(&self) -> Duration {
        Duration::from_secs(self.config.settings.refresh_interval)
    }
} 