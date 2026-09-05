// SPDX-License-Identifier: MPL-2.0

use crate::fl;
use crate::message::Message;
use crate::systemd::{ServiceScope, SystemdService};
use crate::types::{ContextPage, MenuAction, Page, SortColumn, SortDirection};
use crate::views;
use cosmic::app::context_drawer;
use cosmic::iced::{Length, Subscription};
use cosmic::widget::menu::{ItemHeight, ItemWidth};
use cosmic::widget::{RcElementWrapper, button};
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use cosmic::{prelude::*, theme};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long the "Copied" state on the logs copy button is shown.
pub(crate) const LOGS_COPIED_INDICATOR_DURATION: Duration = Duration::from_secs(3);

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

pub struct AppModel {
    pub(crate) core: cosmic::Core,
    pub(crate) context_page: ContextPage,
    about: About,
    pub nav: nav_bar::Model,
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    pub(crate) system_services: Vec<SystemdService>,
    pub(crate) user_services: Vec<SystemdService>,
    pub(crate) selected_service: Option<SystemdService>,
    pub(crate) current_scope: ServiceScope,
    pub current_page: Page,
    pub service_logs: String,
    /// Editor state for the logs view; provides multi-line text selection.
    pub logs_editor: cosmic::widget::text_editor::Content<cosmic::Renderer>,
    pub is_loading: bool,
    pub search_filter: String,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    /// When the logs were last copied, used to show the "Copied" state.
    pub logs_copied_at: Option<Instant>,
    pub(crate) search_expanded: bool,
    pub(crate) search_id: cosmic::widget::Id,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "io.github.nikelaz.CtlDash";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("system-services"))
            .data::<Page>(Page::SystemServices)
            .icon(icon::from_name("applications-system-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("user-services"))
            .data::<Page>(Page::UserServices)
            .icon(icon::from_name("system-users-symbolic"));

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .author("Nikola Lazarov")
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([
                (fl!("support"), REPOSITORY),
                (fl!("repository"), REPOSITORY),
            ])
            .license(env!("CARGO_PKG_LICENSE"))
            .developers([("Nikola Lazarov", "nikola.n.lazarov@outlook.com")]);

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            system_services: Vec::new(),
            user_services: Vec::new(),
            selected_service: None,
            current_scope: ServiceScope::System,
            current_page: Page::SystemServices,
            service_logs: "".to_string(),
            logs_editor: cosmic::widget::text_editor::Content::new(),
            is_loading: false,
            search_filter: String::new(),
            sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            logs_copied_at: None,
            search_expanded: false,
            search_id: cosmic::widget::Id::unique(),
        };

        // Create a startup command that sets the window title and loads services.
        let title_command = app.update_title();
        let load_command = Task::perform(async {}, |_| {
            cosmic::Action::from(Message::LoadServices(Some(ServiceScope::System)))
        });

        (app, Task::batch(vec![title_command, load_command]))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            RcElementWrapper::new(
                button::icon(icon::from_name("open-menu-symbolic"))
                    .padding([4, 12])
                    .class(theme::Button::MenuRoot)
                    .into(),
            ),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )])
        .item_height(ItemHeight::Dynamic(40))
        .item_width(ItemWidth::Uniform(240))
        .spacing(4.0);

        vec![menu_bar.into()]
    }

    /// Elements to pack at the end of the header bar, next to the window controls.
    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        match &self.current_page {
            Page::SystemServices | Page::UserServices => {
                if self.search_expanded {
                    let search_input =
                        widget::search_input(fl!("search-placeholder"), &self.search_filter)
                            .id(self.search_id.clone())
                            .on_clear(Message::ToggleSearch(false))
                            .on_input(Message::SearchFilterChanged)
                            .select_on_focus(true)
                            .width(Length::Fixed(240.0));

                    vec![search_input.into()]
                } else {
                    let search_button = widget::button::icon(widget::icon::from_name(
                        "system-search-symbolic",
                    ))
                    .on_press(Message::ToggleSearch(true));

                    vec![search_button.into()]
                }
            },
            Page::Details => Vec::new(),
        }
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

        match &self.current_page {
            Page::SystemServices => {
                content = views::view_services_list(self, &self.system_services, fl!("system-services"));
            },
            Page::UserServices => {
                content = views::view_services_list(self, &self.user_services, fl!("user-services"));
            },
            Page::Details => {
                content = views::view_service_detail(self, self.selected_service.as_ref());
            },
        }

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(cosmic::iced::Padding::from([0, spacing.space_m, spacing.space_m, spacing.space_m]))
            .into()
    }

    /// Register subscriptions for this application.
    fn subscription(&self) -> Subscription<Self::Message> {
        cosmic::iced::time::every(std::time::Duration::from_secs(1))
            .map(|_| Message::Tick)
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        self.update_message(message)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        self.selected_service = None;
        self.search_filter.clear();
        self.search_expanded = false;

        let mut scope = ServiceScope::System;

        let active_nav_page = self.nav.active_data::<Page>().unwrap();

        self.current_page = *active_nav_page;

        if *active_nav_page == Page::UserServices {
            scope = ServiceScope::User;
        }

        let title_command = self.update_title();
        let load_command = Task::perform(async {}, move |_| {
            cosmic::Action::from(Message::LoadServices(Some(scope)))
        });

        Task::batch(vec![title_command, load_command])
    }
}
