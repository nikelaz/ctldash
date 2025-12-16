// SPDX-License-Identifier: MPL-2.0

use crate::systemd::{ServiceScope, SystemdService};
use crate::types::ContextPage;

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    LoadServices(Option<ServiceScope>),
    ServicesLoaded(ServiceScope, Vec<SystemdService>),
    SelectService(SystemdService),
    BackToList,
    StartService(String),
    StopService(String),
    RestartService(String),
    EnableService(String),
    DisableService(String),
    ServiceActionComplete,
    LogsLoaded(String),
    RefreshCurrentService,
    CurrentServiceRefreshed(Option<SystemdService>, String),
    Tick,
    SearchFilterChanged(String),
}
