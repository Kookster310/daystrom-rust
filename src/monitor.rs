use crate::config::{Config, Host, Protocol, Service};

use chrono::{DateTime, Utc};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub enum ServiceStatus {
    Up,
    Down,
    Unknown,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Up => write!(f, "🟢 UP"),
            ServiceStatus::Down => write!(f, "🔴 DOWN"),
            ServiceStatus::Unknown => write!(f, "🟡 UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceCheck {
    pub host_name: String,
    pub service_name: String,
    pub address: String,
    pub port: u16,
    pub protocol: Protocol,
    pub status: ServiceStatus,
    pub last_check: DateTime<Utc>,
    pub response_time: Duration,
    pub error_message: Option<String>,
}

impl ServiceCheck {
    pub fn new(host: &Host, service: &Service) -> Self {
        Self {
            host_name: host.name.clone(),
            service_name: service.name.clone(),
            address: host.address.clone(),
            port: service.port,
            protocol: service.protocol.clone(),
            status: ServiceStatus::Unknown,
            last_check: Utc::now(),
            response_time: Duration::from_secs(0),
            error_message: None,
        }
    }
}

#[derive(Debug)]
pub struct MonitorEngine {
    config: Config,
    statuses: Arc<RwLock<HashMap<String, ServiceCheck>>>,
    http_client: Client,
}

impl MonitorEngine {
    pub fn new(config: Config) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            statuses: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        }
    }

    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let interval = Duration::from_secs(self.config.settings.refresh_interval);
        let engine = self.clone();
        
        tokio::spawn(async move {
            info!("Starting monitoring engine with {} second interval", interval.as_secs());
            
            // Initial check
            engine.check_all_services().await;
            
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                engine.check_all_services().await;
            }
        })
    }

    async fn check_all_services(&self) {
        debug!("Starting service health checks");
        
        let mut tasks = Vec::new();
        
        for host in &self.config.hosts {
            for service in &host.services {
                let engine = self.clone();
                let host = host.clone();
                let service = service.clone();
                
                let task = tokio::spawn(async move {
                    engine.check_service(&host, &service).await;
                });
                
                tasks.push(task);
            }
        }
        
        // Wait for all checks to complete
        for task in tasks {
            if let Err(e) = task.await {
                error!("Service check task failed: {}", e);
            }
        }
        
        debug!("Completed service health checks");
    }

    async fn check_service(&self, host: &Host, service: &Service) {
        let key = format!("{}:{}:{}", host.name, service.name, service.port);
        let mut check = ServiceCheck::new(host, service);
        
        let start_time = Instant::now();
        
        match service.protocol {
            Protocol::Tcp => {
                let result = self.check_tcp(&host.address, service.port, service.timeout).await;
                check.status = result.0;
                check.error_message = result.1;
            }
            Protocol::Udp => {
                let result = self.check_udp(&host.address, service.port, service.timeout).await;
                check.status = result.0;
                check.error_message = result.1;
            }
            Protocol::Http => {
                let result = self.check_http(&host.address, service.port, &service.path, service.timeout).await;
                check.status = result.0;
                check.error_message = result.1;
            }
            Protocol::Https => {
                let result = self.check_https(&host.address, service.port, &service.path, service.timeout).await;
                check.status = result.0;
                check.error_message = result.1;
            }
        }
        
        check.response_time = start_time.elapsed();
        check.last_check = Utc::now();
        
        // Update status in shared map
        let mut statuses = self.statuses.write().await;
        statuses.insert(key, check);
    }

    async fn check_tcp(&self, address: &str, port: u16, timeout: u64) -> (ServiceStatus, Option<String>) {
        let addr = format!("{}:{}", address, port);
        let timeout_duration = Duration::from_secs(timeout);
        
        match tokio::time::timeout(timeout_duration, tokio::net::TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => (ServiceStatus::Up, None),
            Ok(Err(e)) => (ServiceStatus::Down, Some(e.to_string())),
            Err(_) => (ServiceStatus::Down, Some("Connection timeout".to_string())),
        }
    }

    async fn check_udp(&self, _address: &str, _port: u16, timeout: u64) -> (ServiceStatus, Option<String>) {
        // UDP checks are more complex - for now we'll do a basic socket test
        let timeout_duration = Duration::from_secs(timeout);
        
        match tokio::time::timeout(timeout_duration, tokio::net::UdpSocket::bind("0.0.0.0:0")).await {
            Ok(Ok(_)) => (ServiceStatus::Up, None),
            Ok(Err(e)) => (ServiceStatus::Down, Some(e.to_string())),
            Err(_) => (ServiceStatus::Down, Some("UDP socket creation timeout".to_string())),
        }
    }

    async fn check_http(&self, address: &str, port: u16, path: &Option<String>, timeout: u64) -> (ServiceStatus, Option<String>) {
        let url = if port == 80 {
            format!("http://{}", address)
        } else {
            format!("http://{}:{}", address, port)
        };
        
        let url = if let Some(path) = path {
            format!("{}{}", url, path)
        } else {
            url
        };
        
        let timeout_duration = Duration::from_secs(timeout);
        
        match tokio::time::timeout(timeout_duration, self.http_client.get(&url).send()).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    (ServiceStatus::Up, None)
                } else {
                    (ServiceStatus::Down, Some(format!("HTTP {}", response.status())))
                }
            }
            Ok(Err(e)) => (ServiceStatus::Down, Some(e.to_string())),
            Err(_) => (ServiceStatus::Down, Some("HTTP request timeout".to_string())),
        }
    }

    async fn check_https(&self, address: &str, port: u16, path: &Option<String>, timeout: u64) -> (ServiceStatus, Option<String>) {
        let url = if port == 443 {
            format!("https://{}", address)
        } else {
            format!("https://{}:{}", address, port)
        };
        
        let url = if let Some(path) = path {
            format!("{}{}", url, path)
        } else {
            url
        };
        
        let timeout_duration = Duration::from_secs(timeout);
        
        match tokio::time::timeout(timeout_duration, self.http_client.get(&url).send()).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    (ServiceStatus::Up, None)
                } else {
                    (ServiceStatus::Down, Some(format!("HTTPS {}", response.status())))
                }
            }
            Ok(Err(e)) => (ServiceStatus::Down, Some(e.to_string())),
            Err(_) => (ServiceStatus::Down, Some("HTTPS request timeout".to_string())),
        }
    }

    pub async fn get_statuses(&self) -> HashMap<String, ServiceCheck> {
        self.statuses.read().await.clone()
    }
}

impl Clone for MonitorEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            statuses: self.statuses.clone(),
            http_client: self.http_client.clone(),
        }
    }
} 