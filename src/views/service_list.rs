// SPDX-License-Identifier: MPL-2.0

use crate::app::AppModel;
use crate::fl;
use crate::message::Message;
use crate::systemd::SystemdService;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use cosmic::iced::mouse::Interaction;

pub fn view_services_list<'a>(
    app: &'a AppModel,
    services: &'a [SystemdService],
    title: String,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    
    // Localized strings
    let search_placeholder = fl!("search-placeholder");

    let search_input = widget::text_input(search_placeholder, &app.search_filter)
        .on_input(Message::SearchFilterChanged)
        .width(Length::Fill);

    let header = widget::row()
        .push(widget::text::title3(title))
        .push(search_input)
        .spacing(spacing.space_l)
        .align_y(Alignment::Center);


    let filtered_services: Vec<&SystemdService> = if app.search_filter.is_empty() {
        services.iter().collect()
    } else {
        let filter_lower = app.search_filter.to_lowercase();
        services
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&filter_lower)
                    || s.description.to_lowercase().contains(&filter_lower)
            })
            .collect()
    };

    // Localized table headers
    let service_text = fl!("service");
    let description_text = fl!("description");
    let active_state_text = fl!("active-state");
    let sub_state_text = fl!("sub-state");
    let loading_text = fl!("loading-services");
    let no_services_text = fl!("no-services-found");
    let no_match_text = fl!("no-services-match");

    let list_header = widget::row()
        .push(widget::text(service_text).width(Length::FillPortion(3)))
        .push(widget::text(description_text).width(Length::FillPortion(3)))
        .push(widget::text(active_state_text).width(Length::FillPortion(1)))
        .push(widget::text(sub_state_text).width(Length::FillPortion(1)))
        .padding(cosmic::iced::Padding::from([0, spacing.space_m]));

    let mut list = widget::list_column().spacing(spacing.space_xs);

    if app.is_loading {
        list = list.add(widget::text(loading_text));
    } else if filtered_services.is_empty() {
        if app.search_filter.is_empty() {
            list = list.add(widget::text(no_services_text));
        } else {
            list = list.add(widget::text(no_match_text));
        }
    } else {
        for service in filtered_services {
            let row_content = widget::row()
                .push(
                    widget::text(&service.name)
                        .width(Length::FillPortion(3))
                        .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph)
                )
                .push(
                    widget::text(&service.description)
                        .width(Length::FillPortion(3))
                        .wrapping(cosmic::iced::widget::text::Wrapping::Word)
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
                widget::mouse_area(row_content)
                    .interaction(Interaction::Pointer)
                    .on_press(Message::SelectService(service_clone))
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
