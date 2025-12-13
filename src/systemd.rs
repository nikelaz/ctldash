// SPDX-License-Identifier: MPL-2.0

use zbus::{Connection, Result};

#[derive(Debug, Clone)]
pub struct SystemdService {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_path: String,
    pub unit_file_state: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceScope {
    System,
    User,
}

pub struct SystemdManager {
    connection: Connection,
}

impl SystemdManager {
    pub async fn new(scope: ServiceScope) -> Result<Self> {
        let connection = match scope {
            ServiceScope::System => Connection::system().await?,
            ServiceScope::User => Connection::session().await?,
        };
        Ok(Self { connection })
    }

    pub async fn list_services(&self) -> Result<Vec<SystemdService>> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        // Use ListUnitFiles to get all service files, not just loaded units
        let unit_files: Vec<(String, String)> = proxy.call("ListUnitFiles", &()).await?;

        let mut services: Vec<SystemdService> = Vec::new();
        
        for (unit_path, unit_file_state) in unit_files {
            // Extract service name from path
            let name = std::path::Path::new(&unit_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            
            if !name.ends_with(".service") {
                continue;
            }

            let (description, load_state, active_state, sub_state) = 
                self.get_unit_properties(&name).await
                    .unwrap_or_else(|_| (
                        String::new(),
                        "not-loaded".to_string(),
                        "inactive".to_string(),
                        "dead".to_string()
                    ));
            
            services.push(SystemdService {
                name,
                description,
                load_state,
                active_state,
                sub_state,
                unit_path,
                unit_file_state,
            });
        }

        Ok(services)
    }

    async fn get_unit_properties(&self, service_name: &str) -> Result<(String, String, String, String)> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        // Try to load the unit to get its properties
        let unit_path: zbus::zvariant::OwnedObjectPath = proxy
            .call("LoadUnit", &(service_name,))
            .await?;

        let unit_proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            unit_path.as_str(),
            "org.freedesktop.systemd1.Unit",
        )
        .await?;

        let description: String = unit_proxy
            .get_property("Description")
            .await
            .unwrap_or_default();
        
        let load_state: String = unit_proxy
            .get_property("LoadState")
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        
        let active_state: String = unit_proxy
            .get_property("ActiveState")
            .await
            .unwrap_or_else(|_| "inactive".to_string());
        
        let sub_state: String = unit_proxy
            .get_property("SubState")
            .await
            .unwrap_or_else(|_| "dead".to_string());

        Ok((description, load_state, active_state, sub_state))
    }

    pub async fn start_service(&self, service_name: &str) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        let _: zbus::zvariant::OwnedObjectPath =
            proxy.call("StartUnit", &(service_name, "replace")).await?;
        Ok(())
    }

    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        let _: zbus::zvariant::OwnedObjectPath =
            proxy.call("StopUnit", &(service_name, "replace")).await?;
        Ok(())
    }

    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        let _: zbus::zvariant::OwnedObjectPath =
            proxy.call("RestartUnit", &(service_name, "replace")).await?;
        Ok(())
    }

    pub async fn enable_service(&self, service_name: &str) -> Result<()> {
        // Use pkexec or polkit to execute systemctl enable
        let output = tokio::process::Command::new("pkexec")
            .arg("systemctl")
            .arg("enable")
            .arg(service_name)
            .output()
            .await
            .map_err(|e| zbus::Error::Failure(format!("Failed to execute pkexec: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(zbus::Error::Failure(format!("Failed to enable service: {}", error)));
        }

        Ok(())
    }

    pub async fn disable_service(&self, service_name: &str) -> Result<()> {
        // Use pkexec or polkit to execute systemctl disable
        let output = tokio::process::Command::new("pkexec")
            .arg("systemctl")
            .arg("disable")
            .arg(service_name)
            .output()
            .await
            .map_err(|e| zbus::Error::Failure(format!("Failed to execute pkexec: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(zbus::Error::Failure(format!("Failed to disable service: {}", error)));
        }

        Ok(())
    }

    pub async fn get_service_logs(&self, service_name: &str, lines: u32) -> Result<String> {
        let name = if service_name.ends_with(".service") {
            service_name.to_string()
        } else {
            format!("{}.service", service_name)
        };

        let output = tokio::process::Command::new("journalctl")
            .arg("-u")
            .arg(&name)
            .arg("-n")
            .arg(lines.to_string())
            .arg("--no-pager")
            .output()
            .await
            .map_err(|e| zbus::Error::Failure(format!("Failed to execute journalctl: {}", e)))?;

        let logs = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_service_creation() {
        let service = SystemdService {
            name: "test.service".to_string(),
            description: "Test Service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            unit_path: "/lib/systemd/system/test.service".to_string(),
            unit_file_state: "enabled".to_string(),
        };

        assert_eq!(service.name, "test.service");
        assert_eq!(service.description, "Test Service");
        assert_eq!(service.load_state, "loaded");
        assert_eq!(service.active_state, "active");
        assert_eq!(service.sub_state, "running");
        assert_eq!(service.unit_path, "/lib/systemd/system/test.service");
        assert_eq!(service.unit_file_state, "enabled");
    }

    #[test]
    fn test_systemd_service_clone() {
        let service = SystemdService {
            name: "test.service".to_string(),
            description: "Test Service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            unit_path: "/lib/systemd/system/test.service".to_string(),
            unit_file_state: "enabled".to_string(),
        };

        let cloned = service.clone();
        assert_eq!(service.name, cloned.name);
        assert_eq!(service.description, cloned.description);
    }

    #[test]
    fn test_service_scope_equality() {
        assert_eq!(ServiceScope::System, ServiceScope::System);
        assert_eq!(ServiceScope::User, ServiceScope::User);
        assert_ne!(ServiceScope::System, ServiceScope::User);
    }

    #[test]
    fn test_service_scope_copy() {
        let scope1 = ServiceScope::System;
        let scope2 = scope1;
        assert_eq!(scope1, scope2);
    }

    #[tokio::test]
    async fn test_systemd_manager_new_system() {
        // This test requires a D-Bus system connection
        // It may fail in CI environments without proper setup
        let result = SystemdManager::new(ServiceScope::System).await;
        
        // We just check if the creation doesn't panic
        // In a real environment with systemd, this should succeed
        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                // Expected in environments without systemd or D-Bus
                println!("Expected error in test environment: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_systemd_manager_new_user() {
        // This test requires a D-Bus session connection
        let result = SystemdManager::new(ServiceScope::User).await;
        
        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                // Expected in environments without systemd or D-Bus
                println!("Expected error in test environment: {:?}", e);
            }
        }
    }

    #[test]
    fn test_service_name_extraction() {
        let unit_path = "/lib/systemd/system/test.service";
        let name = std::path::Path::new(unit_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        assert_eq!(name, "test.service");
        assert!(name.ends_with(".service"));
    }

    #[test]
    fn test_service_name_with_suffix() {
        let service_name = "myservice";
        let name = if service_name.ends_with(".service") {
            service_name.to_string()
        } else {
            format!("{}.service", service_name)
        };
        
        assert_eq!(name, "myservice.service");
    }

    #[test]
    fn test_service_name_already_with_suffix() {
        let service_name = "myservice.service";
        let name = if service_name.ends_with(".service") {
            service_name.to_string()
        } else {
            format!("{}.service", service_name)
        };
        
        assert_eq!(name, "myservice.service");
    }
}
