// SPDX-License-Identifier: MPL-2.0

use crate::app::{AppModel, LOGS_COPIED_INDICATOR_DURATION};
use crate::fl;
use crate::message::Message;
use crate::systemd::{ServiceScope, SystemdManager};
use crate::types::{Page, SortDirection};
use cosmic::prelude::*;
use std::time::Instant;

impl AppModel {
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

impl AppModel {
    /// Handles messages emitted by the application and its widgets.
    pub fn update_message(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::LoadServices(scope) => {
                let mut scope = scope;

                if scope.is_none() {
                    scope = Some(self.current_scope);
                }

                let scope = scope.unwrap();

                // Check if services are already loaded for this scope
                let already_loaded = match scope {
                    ServiceScope::System => !self.system_services.is_empty(),
                    ServiceScope::User => !self.user_services.is_empty(),
                };

                // Only show loader if services aren't already loaded
                if !already_loaded {
                    self.is_loading = true;
                }

                self.current_scope = scope;
                return Task::perform(
                    async move {
                        let manager = SystemdManager::new(scope).await.ok()?;
                        let services = manager.list_services().await.ok()?;
                        Some((scope, services))
                    },
                    |result| {
                        if let Some((scope, services)) = result {
                            cosmic::Action::from(Message::ServicesLoaded(scope, services))
                        } else {
                            cosmic::Action::from(Message::ServicesLoaded(ServiceScope::System, Vec::new()))
                        }
                    },
                );
            }

            Message::ServicesLoaded(scope, services) => {
                self.is_loading = false;

                let selected_service_name = self
                    .selected_service
                    .as_ref()
                    .map(|s| s.name.clone());

                match scope {
                    ServiceScope::System => {
                        // If services were already loaded, update only changed items
                        if !self.system_services.is_empty() {
                            for new_service in &services {
                                if let Some(index) = self.system_services.iter().position(|s| s.name == new_service.name) {
                                    // Only update if the service data has changed
                                    let existing_service = &self.system_services[index];
                                    if existing_service.active_state != new_service.active_state
                                        || existing_service.sub_state != new_service.sub_state
                                        || existing_service.load_state != new_service.load_state
                                        || existing_service.unit_file_state != new_service.unit_file_state
                                        || existing_service.description != new_service.description {
                                        self.system_services[index] = new_service.clone();
                                    }
                                } else {
                                    // New service appeared, add it
                                    self.system_services.push(new_service.clone());
                                }
                            }
                            // Remove services that no longer exist
                            self.system_services.retain(|s| services.iter().any(|new_s| new_s.name == s.name));
                        } else {
                            // First load, replace everything
                            self.system_services = services;
                        }

                        if let Some(name) = selected_service_name {
                            self.selected_service = self.system_services
                                .iter()
                                .find(|s| s.name == name)
                                .cloned();
                        }
                    },
                    ServiceScope::User => {
                        // If services were already loaded, update only changed items
                        if !self.user_services.is_empty() {
                            for new_service in &services {
                                if let Some(index) = self.user_services.iter().position(|s| s.name == new_service.name) {
                                    // Only update if the service data has changed
                                    let existing_service = &self.user_services[index];
                                    if existing_service.active_state != new_service.active_state
                                        || existing_service.sub_state != new_service.sub_state
                                        || existing_service.load_state != new_service.load_state
                                        || existing_service.unit_file_state != new_service.unit_file_state
                                        || existing_service.description != new_service.description {
                                        self.user_services[index] = new_service.clone();
                                    }
                                } else {
                                    // New service appeared, add it
                                    self.user_services.push(new_service.clone());
                                }
                            }
                            // Remove services that no longer exist
                            self.user_services.retain(|s| services.iter().any(|new_s| new_s.name == s.name));
                        } else {
                            // First load, replace everything
                            self.user_services = services;
                        }

                        if let Some(name) = selected_service_name {
                            self.selected_service = self.user_services
                                .iter()
                                .find(|s| s.name == name)
                                .cloned();
                        }
                    },
                }
            }

            Message::SelectService(service) => {
                self.selected_service = Some(service.clone());
                self.current_page = Page::Details;
                let scope = self.current_scope;
                return Task::perform(
                    async move {
                        let manager = SystemdManager::new(scope).await.ok()?;
                        let logs = manager.get_service_logs(&service.name, 100).await.unwrap_or_default();
                        Some(logs)
                    },
                    |result| {
                        if let Some(logs) = result {
                            cosmic::Action::from(Message::LogsLoaded(logs))
                        }
                        else {
                            cosmic::Action::from(Message::LogsLoaded("Could not load logs".to_string()))
                        }
                    },
                );
            }

            Message::LogsLoaded(logs) => {
                self.service_logs = logs;
                self.sync_logs_editor();
            }

            Message::LogsEditorAction(action) => {
                // The logs editor is read-only: ignore editing actions, but
                // still allow selection, scrolling, and copy key binds.
                if !action.is_edit() {
                    self.logs_editor.perform(action);
                }
            }

            Message::BackToList => {
                self.selected_service = None;
                match self.current_scope {
                    ServiceScope::System => self.current_page = Page::SystemServices,
                    ServiceScope::User => self.current_page = Page::UserServices,
                }
            }

            Message::StartService(name) => {
                let scope = self.current_scope;
                return Task::perform(
                    async move {
                        if let Ok(manager) = SystemdManager::new(scope).await {
                            let _ = manager.start_service(&name).await;
                        }
                    },
                    |_| cosmic::Action::from(Message::ServiceActionComplete),
                );
            }

            Message::StopService(name) => {
                let scope = self.current_scope;
                return Task::perform(
                    async move {
                        if let Ok(manager) = SystemdManager::new(scope).await {
                            let _ = manager.stop_service(&name).await;
                        }
                    },
                    |_| cosmic::Action::from(Message::ServiceActionComplete),
                );
            }

            Message::RestartService(name) => {
                let scope = self.current_scope;
                return Task::perform(
                    async move {
                        if let Ok(manager) = SystemdManager::new(scope).await {
                            let _ = manager.restart_service(&name).await;
                        }
                    },
                    |_| cosmic::Action::from(Message::ServiceActionComplete),
                );
            }

            Message::EnableService(name) => {
                let scope = self.current_scope;
                return Task::perform(
                    async move {
                        if let Ok(manager) = SystemdManager::new(scope).await {
                            match manager.enable_service(&name).await {
                                Ok(_) => eprintln!("Successfully enabled: {}", name),
                                Err(e) => eprintln!("Failed to enable {}: {:?}", name, e),
                            }
                        } else {
                            eprintln!("Failed to create SystemdManager");
                        }
                    },
                    |_| cosmic::Action::from(Message::ServiceActionComplete),
                );
            }

            Message::DisableService(name) => {
                eprintln!("DisableService called for: {}", name);
                let scope = self.current_scope;
                return Task::perform(
                    async move {
                        eprintln!("Attempting to disable service: {} with scope: {:?}", name, scope);
                        if let Ok(manager) = SystemdManager::new(scope).await {
                            match manager.disable_service(&name).await {
                                Ok(_) => eprintln!("Successfully disabled: {}", name),
                                Err(e) => eprintln!("Failed to disable {}: {:?}", name, e),
                            }
                        } else {
                            eprintln!("Failed to create SystemdManager");
                        }
                    },
                    |_| cosmic::Action::from(Message::ServiceActionComplete),
                );
            }

            Message::ServiceActionComplete => {
                let scope = self.current_scope;
                return Task::perform(async {}, move |_| {
                    cosmic::Action::from(Message::LoadServices(Some(scope)))
                });
            }

            Message::CopyLogs => {
                self.logs_copied_at = Some(Instant::now());
                return cosmic::iced::clipboard::write(self.service_logs.clone());
            }

            Message::Tick => {
                // Expire the "Copied" indicator on the logs copy button.
                if self
                    .logs_copied_at
                    .is_some_and(|at| at.elapsed() >= LOGS_COPIED_INDICATOR_DURATION)
                {
                    self.logs_copied_at = None;
                }

                if self.selected_service.is_some() {
                    return Task::perform(async {}, |_| {
                        cosmic::Action::from(Message::RefreshCurrentService)
                    });
                }

                return Task::perform(async {}, |_| {
                    cosmic::Action::from(Message::LoadServices(None))
                });
            }

            Message::RefreshCurrentService => {
                if let Some(service) = &self.selected_service {
                    let service_name = service.name.clone();
                    let scope = self.current_scope;
                    return Task::perform(
                        async move {
                            let manager = SystemdManager::new(scope).await.ok()?;
                            let services = manager.list_services().await.ok()?;
                            let updated_service = services.into_iter().find(|s| s.name == service_name);
                            let logs = if let Some(_) = &updated_service {
                                manager.get_service_logs(&service_name, 100).await.unwrap_or_default()
                            } else {
                                String::new()
                            };
                            Some((updated_service, logs))
                        },
                        |result| {
                            if let Some((service, logs)) = result {
                                cosmic::Action::from(Message::CurrentServiceRefreshed(service, logs))
                            } else {
                                cosmic::Action::from(Message::CurrentServiceRefreshed(None, String::new()))
                            }
                        },
                    );
                }
            }

            Message::CurrentServiceRefreshed(service, logs) => {
                if let Some(updated_service) = service {
                    self.selected_service = Some(updated_service.clone());
                    self.service_logs = logs;
                    // Only rebuild the editor content when the logs actually
                    // changed, otherwise auto-refresh would reset the user's
                    // text selection every tick.
                    self.sync_logs_editor();

                    match self.current_scope {
                        ServiceScope::System => {
                            if let Some(index) = self.system_services.iter().position(|s| s.name == updated_service.name) {
                                self.system_services[index] = updated_service;
                            }
                        },
                        ServiceScope::User => {
                            if let Some(index) = self.user_services.iter().position(|s| s.name == updated_service.name) {
                                self.user_services[index] = updated_service;
                            }
                        },
                    }
                }
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::SortServices(column) => {
                if self.sort_column == column {
                    self.sort_direction = self.sort_direction.toggled();
                } else {
                    self.sort_column = column;
                    self.sort_direction = SortDirection::Ascending;
                }
            }

            Message::SearchFilterChanged(filter) => {
                self.search_filter = filter;
            }

            Message::ToggleSearch(expand) => {
                self.search_expanded = expand;

                if expand {
                    // Focus the input as soon as it is expanded.
                    return cosmic::widget::text_input::focus(self.search_id.clone());
                }

                self.search_filter.clear();
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },
        }
        Task::none()
    }

    /// Rebuilds the logs editor content when the logs changed, and auto-scrolls
    /// it to the bottom (the default position for freshly loaded logs).
    fn sync_logs_editor(&mut self) {
        if self.logs_editor.text() == self.service_logs {
            return;
        }

        self.logs_editor =
            cosmic::widget::text_editor::Content::with_text(&self.service_logs);

        // Scroll to the bottom. At this point the buffer has not been laid
        // out yet, so its metrics are still the placeholder 1px line height
        // and the real document height is unknown. Requesting far more lines
        // than any log can occupy is safe: the editor clamps the scroll to
        // the end of the document on the next layout pass.
        self.logs_editor
            .perform(cosmic::widget::text_editor::Action::Scroll {
                lines: 100_000_000,
            });
    }
}
