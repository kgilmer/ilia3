use common::{iced_settings, window_settings, Ilia, IliaConfiguration, ItemDescriptor};
use std::process::exit;
use std::sync::LazyLock;
use swayipc::{Connection, Node, NodeLayout, NodeType};

use anyhow::Context;
use iced::Theme;

static PROGRAM_NAME: LazyLock<String> = std::sync::LazyLock::new(|| String::from("ilia-windows"));

// MAYDO: refactor for i3 compat
const IS_WAYLAND: bool = true;

#[derive(Debug, Clone)]
struct Item {
    id: i64,
    title: String,
}

impl ItemDescriptor for Item {
    fn title(&self) -> &str {
        &self.title
    }

    fn exec(&self) -> anyhow::Result<()> {
        let window_arg = format!("[con_id={}] focus", self.id);
        let args = ["/usr/bin/swaymsg", window_arg.as_str()];

        std::process::Command::new(args[0])
            .args(&args[1..])
            .spawn()
            .context("Failed to spawn app")
            .map(|_| ())?;

        exit(0);
    }
}

impl<'a> From<Node> for Item {
    fn from(node: Node) -> Self {
        let mut title = node.name.expect("Node has no name");

        if title.len() > 12 {
            title = format!("{}…", &title[..12]);
        }

        Item { id: node.id, title }
    }
}

fn main() -> iced::Result {
    let app_factory = || {
        Ilia::new(IliaConfiguration {
            item_loader: load_windows,
            entry_hint: String::from("window"),
        })
    };

    iced::application(PROGRAM_NAME.as_str(), Ilia::update, Ilia::view)
        .settings(iced_settings(PROGRAM_NAME.as_str()))
        .window(window_settings(PROGRAM_NAME.as_str()))
        .theme(|_| Theme::Nord)
        .subscription(Ilia::subscription)
        .run_with(app_factory)
}

fn load_windows() -> Vec<Item> {
    let root_node = Connection::new()
        .expect("Can't connect to WM socket")
        .get_tree()
        .expect("Can't get tree");

    let mut nodes: Vec<Node> = vec![];

    collect_nodes(&root_node, &mut nodes);

    nodes.into_iter().map(|n| Item::from(n)).collect()
}

fn collect_nodes(parent: &Node, container: &mut Vec<Node>) {
    if window_node_filter(parent) {
        container.push(parent.to_owned());
    }

    for node in parent.nodes.iter() {
        collect_nodes(node, container);
    }
}

fn window_node_filter(node: &Node) -> bool {
    if let Some(window_props) = &node.window_properties {
        let Some(window_type) = &window_props.window_type else {
            return false;
        };
        let Some(window_title) = &window_props.title else {
            return false;
        };
        (node.node_type == NodeType::Con || node.node_type == NodeType::FloatingCon)
            && (window_type == "normal"
                || window_type == "unknown"
                || IS_WAYLAND && node.layout == NodeLayout::None)
            && window_title != "i3bar"
    } else {
        (node.node_type == NodeType::Con || node.node_type == NodeType::FloatingCon)
            && (IS_WAYLAND && node.layout == NodeLayout::None)
    }
}
