// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::systemd::{ServiceScope, SystemdManager, SystemdService};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use cosmic::prelude::*;
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// System services
    system_services: Vec<SystemdService>,
    /// User services
    user_services: Vec<SystemdService>,
    /// Selected service for detail view
    selected_service: Option<SystemdService>,
    /// Currently viewing scope
    current_scope: ServiceScope,
    /// Current Service Logs
    service_logs: String,
    /// Loading state
    is_loading: bool,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LoadServices(ServiceScope),
    ServicesLoaded(ServiceScope, Vec<SystemdService>),
    SelectService(SystemdService),
    BackToList,
    StartService(String),
    StopService(String),
    RestartService(String),
    ServiceActionComplete,
    RefreshServices,
    LogsLoaded(String),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.nikelaz.CtlDash";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with two page items for system and user services.
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text("System Services")
            .data::<Page>(Page::SystemServices)
            .icon(icon::from_name("applications-system-symbolic"))
            .activate();

        nav.insert()
            .text("User Services")
            .data::<Page>(Page::UserServices)
            .icon(icon::from_name("system-users-symbolic"));

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            system_services: Vec::new(),
            user_services: Vec::new(),
            selected_service: None,
            current_scope: ServiceScope::System,
            service_logs: "".to_string(),
            is_loading: false,
        };

        // Create a startup command that sets the window title and loads services.
        let title_command = app.update_title();
        let load_command = Task::perform(async {}, |_| {
            cosmic::Action::from(Message::LoadServices(ServiceScope::System))
        });

        (app, Task::batch(vec![title_command, load_command]))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let spacing = cosmic::theme::spacing();

        let content: Element<_>;

        if let Some(service) = &self.selected_service {
            content = self.view_service_detail(service);
        }
        else {
            content = match self.nav.active_data::<Page>().unwrap() {
                Page::SystemServices => self.view_services_list(&self.system_services, "System Services"),
                Page::UserServices => self.view_services_list(&self.user_services, "User Services"),
            };
        }

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(cosmic::iced::Padding::from([0, spacing.space_m, spacing.space_m, spacing.space_m]))
            .into()
    }

    /// Register subscriptions for this application.
    fn subscription(&self) -> Subscription<Self::Message> {
        self.core()
            .watch_config::<Config>(Self::APP_ID)
            .map(|update| Message::UpdateConfig(update.config))
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::LoadServices(scope) => {
                self.is_loading = true;
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
                        self.system_services = services;

                        if let Some(name) = selected_service_name {
                            self.selected_service = self.system_services
                                .iter()
                                .find(|s| s.name == name)
                                .cloned();
                        }
                    },
                    ServiceScope::User => {
                        self.user_services = services;

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
            }

            Message::BackToList => {
                self.selected_service = None;
            }

            Message::StartService(name) => {
                let scope = self.current_scope.clone();
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
                let scope = self.current_scope.clone();
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
                let scope = self.current_scope.clone();
                return Task::perform(
                    async move {
                        if let Ok(manager) = SystemdManager::new(scope).await {
                            let _ = manager.restart_service(&name).await;
                        }
                    },
                    |_| cosmic::Action::from(Message::ServiceActionComplete),
                );
            }

            Message::ServiceActionComplete | Message::RefreshServices => {
                let scope = self.current_scope;
                return Task::perform(async {}, move |_| {
                    cosmic::Action::from(Message::LoadServices(scope))
                });
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
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

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        self.selected_service = None;

        let scope = match self.nav.active_data::<Page>().unwrap() {
            Page::SystemServices => ServiceScope::System,
            Page::UserServices => ServiceScope::User,
        };

        let title_command = self.update_title();
        let load_command = Task::perform(async {}, move |_| {
            cosmic::Action::from(Message::LoadServices(scope))
        });

        Task::batch(vec![title_command, load_command])
    }
}

impl AppModel {
    /// Updates the header and window titles.
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

    fn view_services_list<'a>(&'a self, services: &'a [SystemdService], title: &'a str) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();

        let header = widget::row()
            .push(widget::text::title3(title))
            .push(
                widget::button::standard("Refresh")
                    .on_press(Message::RefreshServices)
            )
            .spacing(spacing.space_m)
            .align_y(Alignment::Center);

        let list_header = widget::row()
            .push(widget::text("Service").width(Length::FillPortion(3)))
            .push(widget::text("Description").width(Length::FillPortion(3)))
            .push(widget::text("Active State").width(Length::FillPortion(1)))
            .push(widget::text("Sub State").width(Length::FillPortion(1)))
            .padding(cosmic::iced::Padding::from([0, spacing.space_m]));

        let mut list = widget::list_column().spacing(spacing.space_xs);

        if self.is_loading {
            list = list.add(widget::text("Loading services..."));
        } else if services.is_empty() {
            list = list.add(widget::text("No services found"));
        } else {
            for service in services {
                let row_content = widget::row()
                    .push(
                        widget::text(&service.name)
                            .width(Length::FillPortion(3))
                    )
                    .push(
                        widget::text(&service.description)
                            .width(Length::FillPortion(3))
                    )
                    .push(
                        widget::text(&service.active_state)
                            .width(Length::FillPortion(1))
                    )
                    .push(
                        widget::text(&service.sub_state)
                            .width(Length::FillPortion(1))
                    );

                let service_clone = service.clone();

                list = list.add(
                    widget::mouse_area(row_content).on_press(Message::SelectService(service_clone))
                )
            }
        }

        let scrollable = widget::scrollable(list)
            .height(Length::Fill);

        let services_table = widget::column()
            .push(list_header)
            .push(scrollable)
            .spacing(spacing.space_xs);

        widget::column()
            .push(header)
            .push(services_table)
            .spacing(spacing.space_m)
            .into()
    }

    fn view_service_detail<'a>(&'a self, service: &'a SystemdService) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();

        let previous_button_label = match self.nav.active_data::<Page>().unwrap() {
            Page::SystemServices => "All System Services",
            Page::UserServices => "All User Services",
        };

        let previous_button = widget::button::icon(icon::from_name("go-previous-symbolic"))
            .extra_small()
            .padding(0)
            .label(previous_button_label)
            .spacing(4)
            .class(widget::button::ButtonClass::Link)
            .on_press(Message::BackToList);

        let sub_page_header = widget::row::with_capacity(2).push(widget::text::title3(&service.name));

        let header = widget::column::with_capacity(2)
            .push(previous_button)
            .push(sub_page_header)
            .spacing(6)
            .width(Length::Shrink);

        let description = widget::row()
            .push(widget::text("Description:").width(Length::Fixed(120.0)))
            .push(widget::text(&service.description))
            .spacing(spacing.space_s);

        let load_state = widget::row()
            .push(widget::text("Load State:").width(Length::Fixed(120.0)))
            .push(widget::text(&service.load_state))
            .spacing(spacing.space_s);

        let enabled = widget::row()
            .push(widget::text("Enabled:").width(Length::Fixed(120.0)))
            .push(widget::toggler(service.active_state == "active"))
            .align_y(Alignment::Center)
            .spacing(spacing.space_s);

        let status = widget::row()
            .push(widget::text("Status:").width(Length::Fixed(120.0)))
            .push(widget::text(&service.sub_state))
            .spacing(spacing.space_s);

        let unit_path = widget::row()
            .push(widget::text("Unit Path:").width(Length::Fixed(120.0)))
            .push(widget::text(&service.unit_path))
            .spacing(spacing.space_s);

        let info_section = widget::column()
            .push(description)
            .push(enabled)
            .push(status)
            .push(load_state)
            .push(unit_path)
            .spacing(spacing.space_s);

        let service_name = service.name.clone();
        let service_name2 = service.name.clone();
        let service_name3 = service.name.clone();


        let mut controls;

        if service.sub_state == "running" {
            controls = widget::row()
                .push(widget::button::standard("Stop").on_press(Message::StopService(service_name2)))
                .push(widget::button::standard("Restart").on_press(Message::RestartService(service_name3)))
                .spacing(spacing.space_s);
        }
        else {
            controls = widget::row()
                .push(widget::button::standard("Start").on_press(Message::StartService(service_name)))
                .push(widget::button::standard("Restart").on_press(Message::RestartService(service_name3)))
                .spacing(spacing.space_s);
        }

        let logs = widget::container(
            widget::text(&self.service_logs)
                .size(12)
        );

        let scrollable_logs = widget::scrollable(logs)
            .width(Length::Fill)
            .height(Length::Fill);

        widget::column()
            .push(header)
            .push(info_section)
            .push(controls)
            .push(widget::text::title4("Logs"))
            .push(scrollable_logs)
            .spacing(spacing.space_m)
            .into()
    }
}

/// The page to display in the application.
pub enum Page {
    SystemServices,
    UserServices,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
