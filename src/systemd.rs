// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use zbus::{Connection, Result};

#[derive(Debug, Clone)]
pub struct SystemdService {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_path: String,
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

        let units: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            zbus::zvariant::OwnedObjectPath,
            u32,
            String,
            zbus::zvariant::OwnedObjectPath,
        )> = proxy.call("ListUnits", &()).await?;

        let services: Vec<SystemdService> = units
            .into_iter()
            .filter(|(name, _, _, _, _, _, _, _, _, _)| name.ends_with(".service"))
            .map(
                |(name, description, load_state, active_state, sub_state, _, unit_path, _, _, _)| {
                    SystemdService {
                        name,
                        description,
                        load_state,
                        active_state,
                        sub_state,
                        unit_path: unit_path.to_string(),
                    }
                },
            )
            .collect();

        Ok(services)
    }

    pub async fn get_service_properties(
        &self,
        unit_path: &str,
    ) -> Result<HashMap<String, zbus::zvariant::OwnedValue>> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            unit_path,
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        let properties: HashMap<String, zbus::zvariant::OwnedValue> =
            proxy.call("GetAll", &("org.freedesktop.systemd1.Unit",)).await?;

        Ok(properties)
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
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        let _: (bool, Vec<(String, String, String)>) =
            proxy.call("EnableUnitFiles", &(vec![service_name], false, true)).await?;
        Ok(())
    }

    pub async fn disable_service(&self, service_name: &str) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;

        let _: Vec<(String, String, String)> =
            proxy.call("DisableUnitFiles", &(vec![service_name], false)).await?;
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
