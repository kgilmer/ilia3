//! Ilia - a bare bones desktop app launcher
#![doc(html_logo_url = "https://github.com/kgilmer/Ilia/blob/main/Ilia.svg")]
use common::{Ilia, IliaConfiguration, ItemDescriptor};
use std::process::exit;
use std::sync::LazyLock;

use anyhow::Context;
use freedesktop_desktop_entry::{default_paths, DesktopEntry, Iter as DesktopIter};
use iced::window;
use iced::{window::settings::PlatformSpecific, Theme};
use iced_core::{Font, Pixels, Size};

static PROGRAM_NAME: LazyLock<String> = std::sync::LazyLock::new(|| String::from("ilia-drun"));

#[derive(Debug, Clone)]
struct Item {
    desktop_entry: DesktopEntry<'static>
}

impl ItemDescriptor for Item {
    fn title(&self) -> &str {
        self.desktop_entry.desktop_entry("Name").unwrap_or("err")
    }

    fn exec(&self) -> &str {
        self.desktop_entry.exec().unwrap()
    }
}

impl <'a> From<DesktopEntry<'static>> for Item {
    fn from(value: DesktopEntry<'static>) -> Self {
        Item { desktop_entry: value}
    }
}

/// Program entrypoint.  Just configures the app, window, and kicks off the iced runtime.
fn main() -> iced::Result {
    // UI settings
    let iced_settings = iced::settings::Settings {
        id: Some(PROGRAM_NAME.to_string()),
        fonts: vec![],
        default_font: Font::DEFAULT,
        default_text_size: Pixels::from(18),
        antialiasing: true,
    };

    // Window settings
    let window_settings = window::Settings {
        size: Size {
            width: 320.0,
            height: 200.0,
        },
        position: window::Position::Centered,
        min_size: None,
        max_size: None,
        visible: true,
        resizable: false,
        decorations: false,
        transparent: false,
        level: Default::default(),
        icon: None,
        platform_specific: PlatformSpecific {
            application_id: PROGRAM_NAME.to_string(),
            override_redirect: false,
        },
        exit_on_close_request: true,
    };

    // A function that returns the app struct
    let app_factory = || {
        Ilia::new(IliaConfiguration {
            item_loader: load_apps,
            primary_action: launch_app,
        })
    };

    // Kick off iced GUI
    iced::application(PROGRAM_NAME.as_str(), Ilia::update, Ilia::view)
        .settings(iced_settings)
        .window(window_settings)
        .theme(|_| Theme::Nord)
        .subscription(Ilia::subscription)
        .run_with(app_factory)
}

/// Launch an app described by `entry`.  This implementation exits the process upon successful launch.
fn launch_app(entry: &Item) -> anyhow::Result<()> {
    let args = shell_words::split(entry.exec())?;
    let args = args
        .iter()
        // Filter out special freedesktop syntax
        .filter(|entry| !entry.starts_with('%'))
        .collect::<Vec<&String>>();

    std::process::Command::new(args[0])
        .args(&args[1..])
        .spawn()
        .context("Failed to spawn app")
        .map(|_| ())?;

    exit(0);
}

/// Load DesktopEntry's from `DesktopIter`
fn load_apps() -> Vec<Item> {
    DesktopIter::new(default_paths())
        .map(|path| DesktopEntry::from_path::<String>(path, None))
        .filter_map(|entry_result| 
            if let Ok(entry) = entry_result {
                Some(Item::from(entry))
            } else {
                None
            }
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use common::IliaMessage;
    use iced::keyboard::{key::Named, Key};

    use super::*;

    static EMPTY_LOADER: fn() -> Vec<Item> = || vec![];

    static TEST_DESKTOP_ENTRY_1: LazyLock<Item> =
        std::sync::LazyLock::new(|| Item { desktop_entry: DesktopEntry::from_appid("test_app_id_1") });
    static TEST_DESKTOP_ENTRY_2: LazyLock<Item> =
        std::sync::LazyLock::new(|| Item { desktop_entry: DesktopEntry::from_appid("test_app_id_2") });
    static TEST_DESKTOP_ENTRY_3: LazyLock<Item> =
        std::sync::LazyLock::new(|| Item { desktop_entry: DesktopEntry::from_appid("test_app_id_3") });

    static TEST_ENTRY_LOADER: fn() -> Vec<Item> = || {
        vec![
            TEST_DESKTOP_ENTRY_1.clone(),
            TEST_DESKTOP_ENTRY_2.clone(),
            TEST_DESKTOP_ENTRY_3.clone(),
        ]
    };

    #[test]
    fn test_default_app_launch() {
        let test_launcher: fn(&Item) -> anyhow::Result<()> = |e| {
            assert!(e.desktop_entry.appid == "test_app_id_1");
            Ok(())
        };

        let (mut unit, _) = Ilia::new(IliaConfiguration {
            item_loader: TEST_ENTRY_LOADER,
            primary_action: test_launcher,
        });

        let _ = unit.update(IliaMessage::ModelLoaded(TEST_ENTRY_LOADER()));
        let _ = unit.update(IliaMessage::ExecuteSelected());
    }

    #[test]
    fn test_no_apps_try_launch() {
        let test_launcher: fn(&Item) -> anyhow::Result<()> = |_e| {
            assert!(false); // should never get here
            Ok(())
        };

        let (mut unit, _) = Ilia::new(IliaConfiguration {
            item_loader: TEST_ENTRY_LOADER,
            primary_action: test_launcher,
        });

        let _ = unit.update(IliaMessage::ModelLoaded(EMPTY_LOADER()));
        let _result = unit.update(IliaMessage::ExecuteSelected());
    }

    #[test]
    fn test_app_navigation() {
        let test_launcher: fn(&Item) -> anyhow::Result<()> = |e| {
            assert!(e.desktop_entry.appid == "test_app_id_2");
            Ok(())
        };

        let (mut unit, _) = Ilia::new(IliaConfiguration {
            item_loader: TEST_ENTRY_LOADER,
            primary_action: test_launcher,
        });

        let _ = unit.update(IliaMessage::ModelLoaded(TEST_ENTRY_LOADER()));
        let _ = unit.update(IliaMessage::KeyEvent(Key::Named(Named::ArrowDown)));
        let _ = unit.update(IliaMessage::KeyEvent(Key::Named(Named::ArrowDown)));
        let _ = unit.update(IliaMessage::KeyEvent(Key::Named(Named::ArrowUp)));
        let _ = unit.update(IliaMessage::ExecuteSelected());
    }
}
