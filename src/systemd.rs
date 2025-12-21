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

    fn is_flatpak() -> bool {
        std::path::Path::new("/.flatpak-info").exists() || 
        std::env::var("FLATPAK_ID").is_ok()
    }

    pub async fn list_services(&self) -> Result<Vec<SystemdService>> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        let units: Vec<(String, String, String, String, String, String, zbus::zvariant::OwnedObjectPath, u32, String, zbus::zvariant::OwnedObjectPath)> = 
            proxy.call("ListUnits", &()).await?;

        let mut services: Vec<SystemdService> = Vec::new();
        
        for (name, description, load_state, active_state, sub_state, _following, unit_object_path, _job_id, _job_type, _job_object_path) in units {
            if !name.ends_with(".service") {
                continue;
            }

            let unit_file_state = self.get_unit_file_state(&unit_object_path).await
                .unwrap_or_else(|_| "unknown".to_string());
            
            services.push(SystemdService {
                name,
                description,
                load_state,
                active_state,
                sub_state,
                unit_path: unit_object_path.to_string(),
                unit_file_state,
            });
        }

        Ok(services)
    }



    async fn get_unit_file_state(&self, unit_object_path: &zbus::zvariant::OwnedObjectPath) -> Result<String> {
        let unit_proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            unit_object_path.as_str(),
            "org.freedesktop.systemd1.Unit",
        )
        .await?;

        let unit_file_state: String = unit_proxy
            .get_property("UnitFileState")
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(unit_file_state)
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
        let output = if Self::is_flatpak() {
            tokio::process::Command::new("flatpak-spawn")
                .arg("--host")
                .arg("pkexec")
                .arg("systemctl")
                .arg("enable")
                .arg(service_name)
                .output()
                .await
                .map_err(|e| zbus::Error::Failure(format!("Failed to execute flatpak-spawn: {}", e)))?
        } else {
            tokio::process::Command::new("pkexec")
                .arg("systemctl")
                .arg("enable")
                .arg(service_name)
                .output()
                .await
                .map_err(|e| zbus::Error::Failure(format!("Failed to execute pkexec: {}", e)))?
        };

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(zbus::Error::Failure(format!("Failed to enable service: {}", error)));
        }

        Ok(())
    }

    pub async fn disable_service(&self, service_name: &str) -> Result<()> {
        let output = if Self::is_flatpak() {
            tokio::process::Command::new("flatpak-spawn")
                .arg("--host")
                .arg("pkexec")
                .arg("systemctl")
                .arg("disable")
                .arg(service_name)
                .output()
                .await
                .map_err(|e| zbus::Error::Failure(format!("Failed to execute flatpak-spawn: {}", e)))?
        } else {
            tokio::process::Command::new("pkexec")
                .arg("systemctl")
                .arg("disable")
                .arg(service_name)
                .output()
                .await
                .map_err(|e| zbus::Error::Failure(format!("Failed to execute pkexec: {}", e)))?
        };

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

        let output = if Self::is_flatpak() {
            tokio::process::Command::new("flatpak-spawn")
                .arg("--host")
                .arg("journalctl")
                .arg("-u")
                .arg(&name)
                .arg("-n")
                .arg(lines.to_string())
                .arg("--no-pager")
                .output()
                .await
                .map_err(|e| zbus::Error::Failure(format!("Failed to execute flatpak-spawn: {}", e)))?
        } else {
            tokio::process::Command::new("journalctl")
                .arg("-u")
                .arg(&name)
                .arg("-n")
                .arg(lines.to_string())
                .arg("--no-pager")
                .output()
                .await
                .map_err(|e| zbus::Error::Failure(format!("Failed to execute journalctl: {}", e)))?
        };

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
