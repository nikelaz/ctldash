// SPDX-License-Identifier: MPL-2.0

use crate::app::AppModel;
use crate::fl;
use crate::message::Message;
use crate::systemd::SystemdService;
use crate::types::{SortColumn, SortDirection};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use cosmic::iced::mouse::Interaction;

fn compare_services(a: &SystemdService, b: &SystemdService, column: SortColumn) -> std::cmp::Ordering {
    match column {
        SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortColumn::Description => a.description.to_lowercase().cmp(&b.description.to_lowercase()),
        SortColumn::ActiveState => a.active_state.cmp(&b.active_state),
        SortColumn::SubState => a.sub_state.cmp(&b.sub_state),
    }
}

fn header_cell<'a>(
    text: String,
    portion: u16,
    column: SortColumn,
    sort_column: SortColumn,
    sort_direction: SortDirection,
) -> Element<'a, Message> {
    let mut label = widget::row::with_capacity(2)
        .push(widget::text(text))
        .align_y(Alignment::Center);

    if sort_column == column {
        let chevron = match sort_direction {
            SortDirection::Ascending => " \u{00A0}\u{00A0}\u{25B2}",
            SortDirection::Descending => " \u{00A0}\u{00A0}\u{25BC}",
        };
        label = label.push(widget::text(chevron).size(8.0));
    }

    widget::mouse_area(label.width(Length::FillPortion(portion)))
        .interaction(Interaction::Pointer)
        .on_press(Message::SortServices(column))
        .into()
}

pub fn view_services_list<'a>(
    app: &'a AppModel,
    services: &'a [SystemdService],
    title: String,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    
    let header = widget::row::with_capacity(1)
        .push(widget::text::title3(title))
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

    // Sort the filtered list by the active sort column and direction.
    let mut filtered_services = filtered_services;
    filtered_services.sort_by(|a, b| {
        let ordering = compare_services(a, b, app.sort_column);
        match app.sort_direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });

    // Localized table headers
    let service_text = fl!("service");
    let description_text = fl!("description");
    let active_state_text = fl!("active-state");
    let sub_state_text = fl!("sub-state");
    let loading_text = fl!("loading-services");
    let no_services_text = fl!("no-services-found");
    let no_match_text = fl!("no-services-match");

    let list_header = widget::row::with_capacity(4)
        .push(header_cell(service_text, 3, SortColumn::Name, app.sort_column, app.sort_direction))
        .push(header_cell(description_text, 3, SortColumn::Description, app.sort_column, app.sort_direction))
        .push(header_cell(active_state_text, 1, SortColumn::ActiveState, app.sort_column, app.sort_direction))
        .push(header_cell(sub_state_text, 1, SortColumn::SubState, app.sort_column, app.sort_direction))
        .padding(cosmic::iced::Padding::from([0, spacing.space_m]));

    let mut list = widget::list_column();

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
            let row_content = widget::row::with_capacity(4)
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

    let services_table = widget::column::with_capacity(2)
        .push(list_header)
        .push(scrollable)
        .spacing(spacing.space_xs);

    widget::column::with_capacity(2)
        .push(header)
        .push(services_table)
        .spacing(spacing.space_m)
        .into()
}
