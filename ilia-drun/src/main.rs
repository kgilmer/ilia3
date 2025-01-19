//! Elbey - a bare bones desktop app launcher
#![doc(html_logo_url = "https://github.com/kgilmer/elbey/blob/main/elbey.svg")]
use common::{Elbey, ElbeyFlags, ItemDescriptor};
use std::process::exit;
use std::sync::LazyLock;

use anyhow::Context;
use freedesktop_desktop_entry::{default_paths, DesktopEntry, Iter as DesktopIter};
use iced::window;
use iced::{window::settings::PlatformSpecific, Theme};
use iced_core::{Font, Pixels, Size};

static PROGRAM_NAME: LazyLock<String> = std::sync::LazyLock::new(|| String::from("Elbey"));

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
        Elbey::new(ElbeyFlags {
            apps_loader: load_apps,
            app_launcher: launch_app,
        })
    };

    // Kick off iced GUI
    iced::application(PROGRAM_NAME.as_str(), Elbey::update, Elbey::view)
        .settings(iced_settings)
        .window(window_settings)
        .theme(|_| Theme::Nord)
        .subscription(Elbey::subscription)
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
