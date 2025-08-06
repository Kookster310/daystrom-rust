use crate::config::Config;
use crate::monitor::{MonitorEngine, ServiceCheck};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum SelectedItem {
    HostHeader(String),
    Service(ServiceCheck),
}

#[derive(Debug)]
pub struct App {
    pub config: Config,
    pub monitor_engine: MonitorEngine,
    pub statuses: HashMap<String, ServiceCheck>,
    pub selected_index: usize,
    pub show_help: bool,
    pub show_host_detail: bool,
    pub selected_host_name: Option<String>,
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
            show_host_detail: false,
            selected_host_name: None,
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
        let total_items = self.get_total_items();
        if total_items > 0 {
            self.selected_index = (self.selected_index + 1) % total_items;
        }
    }

    pub fn previous_item(&mut self) {
        let total_items = self.get_total_items();
        if total_items > 0 {
            self.selected_index = if self.selected_index == 0 {
                total_items - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn enter_host_detail(&mut self) {
        if let Some(selected_item) = self.get_selected_item() {
            match selected_item {
                SelectedItem::HostHeader(host_name) => {
                    self.selected_host_name = Some(host_name);
                    self.show_host_detail = true;
                }
                SelectedItem::Service(service) => {
                    self.selected_host_name = Some(service.host_name.clone());
                    self.show_host_detail = true;
                }
            }
        }
    }

    pub fn exit_host_detail(&mut self) {
        self.show_host_detail = false;
        self.selected_host_name = None;
    }

    pub fn get_selected_host(&self) -> Option<&crate::config::Host> {
        if let Some(host_name) = &self.selected_host_name {
            self.config.hosts.iter().find(|h| &h.name == host_name)
        } else {
            None
        }
    }

    pub fn get_host_services_status(&self, host_name: &str) -> Vec<ServiceCheck> {
        self.statuses
            .values()
            .filter(|status| status.host_name == host_name)
            .cloned()
            .collect()
    }

    pub fn get_grouped_status_list(&self) -> Vec<(String, Vec<ServiceCheck>)> {
        let mut grouped: HashMap<String, Vec<ServiceCheck>> = HashMap::new();
        
        // Group services by host
        for status in self.statuses.values() {
            grouped
                .entry(status.host_name.clone())
                .or_insert_with(Vec::new)
                .push(status.clone());
        }
        
        // Sort hosts and services within each host
        let mut result: Vec<_> = grouped.into_iter().collect();
        result.sort_by(|(a_host, _), (b_host, _)| a_host.cmp(b_host));
        
        for (_, services) in &mut result {
            services.sort_by(|a, b| a.service_name.cmp(&b.service_name));
        }
        
        result
    }

    pub fn get_selected_item(&self) -> Option<SelectedItem> {
        let grouped = self.get_grouped_status_list();
        let mut current_index = 0;
        
        for (host_name, services) in &grouped {
            // Check if selection is on this host header
            if current_index == self.selected_index {
                return Some(SelectedItem::HostHeader(host_name.clone()));
            }
            current_index += 1;
            
            // Check if selection is on one of this host's services
            for service in services {
                if current_index == self.selected_index {
                    return Some(SelectedItem::Service(service.clone()));
                }
                current_index += 1;
            }
        }
        
        None
    }

    pub fn get_total_items(&self) -> usize {
        let grouped = self.get_grouped_status_list();
        let mut total = 0;
        
        for (_, services) in &grouped {
            total += 1; // Host header
            total += services.len(); // Services
        }
        
        total
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