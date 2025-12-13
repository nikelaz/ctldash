// SPDX-License-Identifier: MPL-2.0

use crate::message::Message;
use cosmic::widget::menu;

/// The page to display in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Menu actions for the application's menu bar.
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
