// SPDX-License-Identifier: MPL-2.0

use crate::app::AppModel;
use crate::fl;
use crate::message::Message;
use crate::types::Page;
use crate::systemd::SystemdService;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, icon};
use cosmic::Element;

pub fn view_service_detail<'a>(
    app: &'a AppModel,
    service: &'a SystemdService,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    
    // Localized strings
    let all_system_services = fl!("all-system-services");
    let all_user_services = fl!("all-user-services");
    let description_label = fl!("description-label");
    let load_state_label = fl!("load-state-label");
    let enabled_label = fl!("enabled-label");
    let status_label = fl!("status-label");
    let unit_path_label = fl!("unit-path-label");
    let start_text = fl!("start");
    let stop_text = fl!("stop");
    let restart_text = fl!("restart");
    let logs_text = fl!("logs");

    let previous_button_label = match app.nav.active_data::<Page>().unwrap() {
        Page::SystemServices => all_system_services,
        Page::UserServices => all_user_services,
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
        .push(widget::text(description_label).width(Length::Fixed(120.0)))
        .push(widget::text(&service.description))
        .spacing(spacing.space_s);

    let load_state = widget::row()
        .push(widget::text(load_state_label).width(Length::Fixed(120.0)))
        .push(widget::text(&service.load_state))
        .spacing(spacing.space_s);

    let is_enabled = service.unit_file_state == "enabled";
    let can_toggle = service.unit_file_state == "enabled" || service.unit_file_state == "disabled";
    let service_name_for_toggle = service.name.clone();
    
    let enabled_toggler = if can_toggle {
        widget::toggler(is_enabled)
            .on_toggle(move |enabled| {
                if enabled {
                    Message::EnableService(service_name_for_toggle.clone())
                } else {
                    Message::DisableService(service_name_for_toggle.clone())
                }
            })
    } else {
        widget::toggler(is_enabled)
    };
    
    let enabled = widget::row()
        .push(widget::text(enabled_label).width(Length::Fixed(120.0)))
        .push(enabled_toggler)
        .push(widget::text(format!("({})", service.unit_file_state)).size(12))
        .align_y(Alignment::Center)
        .spacing(spacing.space_s);

    let status = widget::row()
        .push(widget::text(status_label).width(Length::Fixed(120.0)))
        .push(widget::text(&service.sub_state))
        .spacing(spacing.space_s);

    let unit_path = widget::row()
        .push(widget::text(unit_path_label).width(Length::Fixed(120.0)))
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

    let controls;

    if service.sub_state == "running" {
        controls = widget::row()
            .push(widget::button::standard(stop_text.clone()).on_press(Message::StopService(service_name2)))
            .push(widget::button::standard(restart_text.clone()).on_press(Message::RestartService(service_name3)))
            .spacing(spacing.space_s);
    }
    else {
        controls = widget::row()
            .push(widget::button::standard(start_text).on_press(Message::StartService(service_name)))
            .push(widget::button::standard(restart_text).on_press(Message::RestartService(service_name3)))
            .spacing(spacing.space_s);
    }

    let logs = widget::container(
        widget::text(&app.service_logs)
            .size(12)
    );

    let scrollable_logs = widget::scrollable(logs)
        .width(Length::Fill)
        .height(Length::Fill);

    widget::column()
        .push(header)
        .push(info_section)
        .push(controls)
        .push(widget::text::title4(logs_text))
        .push(scrollable_logs)
        .spacing(spacing.space_m)
        .into()
}
