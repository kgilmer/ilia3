use std::process::exit;
use std::sync::LazyLock;

use iced::widget::button::{primary, text};
use iced::widget::scrollable::{snap_to, RelativeOffset};
use iced::widget::{button, column, scrollable, text_input, Column};
use iced::{event, window, Element, Event, Length, Task};
use iced_core::keyboard::key::Named;
use iced_core::keyboard::Key;
use iced_runtime::futures::MaybeSend;

/// A magic value to calculate relative pixel hight to move one item in the scrollable
const ITEM_HEIGHT_SCALE_FACTOR: f32 = 0.00750;

static ENTRY_WIDGET_ID: LazyLock<iced::widget::text_input::Id> =
    std::sync::LazyLock::new(|| iced::widget::text_input::Id::new("entry"));
static ITEMS_WIDGET_ID: LazyLock<iced::widget::scrollable::Id> =
    std::sync::LazyLock::new(|| iced::widget::scrollable::Id::new("items"));

pub trait ItemDescriptor {
    fn title(&self) -> &str;
    fn exec(&self) -> &str;
}

/// The application model type.  See [the iced book](https://book.iced.rs/) for details.
#[derive(Debug)]
pub struct State<T: MaybeSend + ItemDescriptor> {
    /// A text entry box where a user can enter list filter criteria
    entry: String,
    /// The complete list of ItemDescriptor, as retrieved by lib
    apps: Vec<T>,
    /// The index of the item visibly selected in the UI
    selected_index: usize,
    /// A flag to indicate app window has received focus. Work around to some windowing environments passing `unfocused` unexpectedly.
    received_focus: bool,
}

/// Root struct of application
#[derive(Debug)]
pub struct Ilia<T: MaybeSend + ItemDescriptor> {
    state: State<T>,
    flags: IliaConfiguration<T>,
}

/// Messages are how your logic mutates the app state and GUI
#[derive(Debug, Clone)]
pub enum IliaMessage<T: MaybeSend> {
    /// Signals that the `ItemDescriptor` have been fully loaded into the vec
    ModelLoaded(Vec<T>),
    /// Signals that the primary text edit box on the UI has been changed by the user, including the new text.
    EntryUpdate(String),
    /// Signals that the user has taken primary action on a selection.
    ExecuteSelected(),
    /// Signals that the user has pressed a key
    KeyEvent(Key),
    /// Signals that the window has gained focus
    GainedFocus,
    /// Signals that the window has lost focus
    LostFocus,
}

/// Provide some initial configuration to app to facilitate testing
#[derive(Debug, Clone)]
pub struct IliaConfiguration<T: MaybeSend> {
    /**
     * A function that returns the list of Items
     */
    pub item_loader: fn() -> Vec<T>,
    /**
     * A function that performs the primary action from a `ItemDescriptor`
     */
    pub primary_action: fn(&T) -> anyhow::Result<()>, //TODO ~ return a task that exits app
}

impl <T: MaybeSend + Clone + ItemDescriptor + 'static> Ilia<T> {
    pub fn new(flags: IliaConfiguration<T>) -> (Self, Task<IliaMessage<T>>) {
        (
            Self {
                state: State {
                    entry: String::new(),
                    apps: vec![],
                    selected_index: 0,
                    received_focus: false,
                },
                flags: flags.clone(),
            },
            Task::perform(async {}, move |_| {
                IliaMessage::ModelLoaded((flags.item_loader)())
            }),
        )
    }

    /// Entry-point from `iced` into app to construct UI
    pub fn view(&self) -> Element<'_, IliaMessage<T>> {
        // Create the list UI elements based on the `ItemDescriptor` model
        let app_elements: Vec<Element<IliaMessage<T>>> = self
            .state
            .apps
            .iter()
            .filter(|e| Self::text_entry_filter(e, &self.state))
            .enumerate()
            .map(|(index, entry)| {
                let name = entry.title();
                let selected = self.state.selected_index == index;
                button(name)
                    .style(move |theme, status| {
                        if selected {
                            primary(theme, status)
                        } else {
                            text(theme, status)
                        }
                    })
                    .width(Length::Fill)
                    .on_press(IliaMessage::ExecuteSelected())
                    .into()
            })
            .collect();

        // Bare bones!
        // TODO: Fancier layout?
        column![
            text_input("drun", &self.state.entry)
                .id(ENTRY_WIDGET_ID.clone())
                .on_input(IliaMessage::EntryUpdate)
                .width(320),
            scrollable(Column::with_children(app_elements))
                .width(320)
                .id(ITEMS_WIDGET_ID.clone()),
        ]
        .into()
    }

    /// Entry-point from `iced` to handle user and system events
    pub fn update(&mut self, message: IliaMessage<T>) -> Task<IliaMessage<T>> {
        match message {
            // The model has been loaded, initialize the UI
            IliaMessage::ModelLoaded(items) => {
                self.state.apps = items;
                text_input::focus::<IliaMessage<T>>(ENTRY_WIDGET_ID.clone())
            }
            // Rebuild the select list based on the updated text entry
            IliaMessage::EntryUpdate(entry_text) => {
                self.state.entry = entry_text;
                self.state.selected_index = 0;

                Task::none()
            }
            // Launch an application selected by the user
            IliaMessage::ExecuteSelected() => {
                if let Some(entry) = self.selected_entry() {
                    (self.flags.primary_action)(entry).expect("Failed to launch app");
                }
                Task::none()
            }
            // Handle keyboard entries
            IliaMessage::KeyEvent(key) => match key {
                Key::Named(Named::Escape) => exit(0),
                Key::Named(Named::ArrowUp) => self.navigate_items(-1),
                Key::Named(Named::ArrowDown) => self.navigate_items(1),
                Key::Named(Named::Enter) => {
                    if let Some(entry) = self.selected_entry() {
                        (self.flags.primary_action)(entry).expect("Failed to launch app");
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
            // Handle window events
            IliaMessage::GainedFocus => {
                self.state.received_focus = true;
                Task::none()
            }
            IliaMessage::LostFocus => {
                if self.state.received_focus {
                    exit(0);
                }
                Task::none()
            }
        }
    }

    /// The `iced` entry-point to setup event listeners
    pub fn subscription(&self) -> iced::Subscription<IliaMessage<T>> {
        // Framework code to integrate with underlying user interface devices; keyboard, mouse.
        event::listen_with(|event, _status, _| match event {
            Event::Window(window::Event::Focused) => Some(IliaMessage::GainedFocus),
            Event::Window(window::Event::Unfocused) => Some(IliaMessage::LostFocus),
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modifiers: _,
                text: _,
                key,
                location: _,
                modified_key: _,
                physical_key: _,
            }) => Some(IliaMessage::KeyEvent(key)),
            _ => None,
        })
    }

    // Return ref to the selected item from the app list after applying filter
    fn selected_entry(&self) -> Option<&T> {
        self.state
            .apps
            .iter()
            .filter(|e| Self::text_entry_filter(e, &self.state))
            .nth(self.state.selected_index)
    }

    // Change the selected item and update the UI with the returned `Task`
    fn navigate_items(&mut self, delta: i32) -> iced::Task<IliaMessage<T>> {
        let new_index = (self.state.selected_index as i32 + delta) as usize;
        let size = self
            .state
            .apps
            .iter()
            .filter(|e| Self::text_entry_filter(e, &self.state))
            .count();

        if (0..size).contains(&new_index) {
            self.state.selected_index = new_index;

            snap_to::<IliaMessage<T>>(
                ITEMS_WIDGET_ID.clone(),
                RelativeOffset {
                    x: 0.0,
                    y: new_index as f32 * ITEM_HEIGHT_SCALE_FACTOR,
                },
            )
        } else {
            Task::none() // If the new location is out of bounds, ignore
        }
    }

    // Compute the items in the list to display based on the model
    fn text_entry_filter(entry: &T, model: &State<T>) -> bool {
        entry.title().to_lowercase().contains(&model.entry.to_lowercase())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {        
        assert_eq!(true, true);
    }
}
