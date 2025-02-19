
//         <------------ Outer Frame Width ----------->
//         ____________________________________________
//      ^(0)  ________ External/Outer Border _______   |
//      | |  |_____________ Title Bar ______________|  |
//      | | (1)_____________ Menu Bar ______________|  | ^
//      | | (2)_____________ Tool Bar ______________|  | ^
//      | | (3)_____________ Tab Bar _______________|  | ^
//      | |  |  _________ Internal Border ________  |  | ^
//      | |  | |   ^                              | |  | |
//      | |  | |   |                              | |  | |
// Outer  |  | | Inner                            | |  | Native
// Frame  |  | | Frame                            | |  | Frame
// Height |  | | Height                           | |  | Height
//      | |  | |   |                              | |  | |
//      | |  | |<--+--- Inner Frame Width ------->| |  | |
//      | |  | |   |                              | |  | |
//      | |  | |___v______________________________| |  | |
//      | |  |___________ Internal Border __________|  | v
//      v |___________ External/Outer Border __________|
//            <-------- Native Frame Width -------->


use taffy::prelude::*;

use crate::window::Window;

#[derive(Debug)]
pub struct Frame {
    /* Name of this frame: a Lisp string.  It is used for looking up resources,
    as well as for the title in some cases.  */
    name: String,

    // title: String,
    // parent_frame: Option<Arc<Frame>>,
    layout: FrameLayout,

    /// collection of windows. window id is converted from taffy's `NodeId`.
    windows: std::collections::HashMap<u64, Window>,

    pub selected_window: u64,

    cursor_pos: usize,

    left: f32,
    top: f32,
}

#[derive(Debug)]
pub struct FrameLayout {
    pub tree: TaffyTree<()>,
    pub root: NodeId,
    pub external_border: NodeId,
    pub title_bar: NodeId,
    pub menu_bar: NodeId,
    pub tool_bar: NodeId,
    pub tab_bar: NodeId,
    pub internal_border: NodeId,
    pub main: NodeId,
}

impl FrameLayout {
    pub fn new(width: f32, height: f32) -> Self {
        let mut tree = TaffyTree::new();
        
        // Create root node
        let root = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(height) },
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }).unwrap();

        // Create frame components
        let external_border = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(2.0) },
            ..Default::default()
        }).unwrap();

        let title_bar = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(30.0) },
            ..Default::default()
        }).unwrap();

        let menu_bar = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(25.0) },
            ..Default::default()
        }).unwrap();

        let tool_bar = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(40.0) },
            ..Default::default()
        }).unwrap();

        let tab_bar = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(30.0) },
            ..Default::default()
        }).unwrap();

        let internal_border = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(2.0) },
            ..Default::default()
        }).unwrap();

        let main = tree.new_leaf(Style {
            size: Size { width: length(width), height: length(height - 127.0) }, // 127 = sum of other components
            flex_grow: 1.0,
            ..Default::default()
        }).unwrap();

        // Build the layout tree
        tree.add_child(root, external_border).unwrap();
        tree.add_child(root, title_bar).unwrap();
        tree.add_child(root, menu_bar).unwrap();
        tree.add_child(root, tool_bar).unwrap();
        tree.add_child(root, tab_bar).unwrap();
        tree.add_child(root, internal_border).unwrap();
        tree.add_child(root, main).unwrap();

        tree.compute_layout(root, Size::max_content()).unwrap();

        FrameLayout {
            tree,
            root,
            external_border,
            title_bar,
            menu_bar,
            tool_bar,
            tab_bar,
            internal_border,
            main,
        }
    }

    pub fn get(&self, id: NodeId) -> anyhow::Result<&Layout> {
        Ok(self.tree.layout(id)?)
    }

    pub fn resize(&mut self, width: f32, height: f32) -> anyhow::Result<()> {
        // let r = self.tree.set_style(self.root, );
        let mut style = self.tree.style(self.root)?.clone();
        style.size.width = length(width);
        style.size.height = length(height);
        self.tree.set_style(self.root, style)?;

        self.tree.compute_layout(self.root, max_content())?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) fn print_tree(&mut self) {
        self.tree.print_tree(self.root)
    }

    pub fn split(
        &mut self,
        parent: NodeId,
        direction: FlexDirection,
    ) -> anyhow::Result<(NodeId, NodeId)> {
        let others = self.tree.child_count(parent);
        if others > 0 {
            return Err(taffy::TaffyError::InvalidInputNode(parent).into());
        }

        let Some(_) = self.tree.parent(parent) else {
            return Err(taffy::TaffyError::InvalidInputNode(parent).into());
        };

        let mut style = self.tree.style(parent)?.clone();
        let origin = self.tree.new_leaf(style.clone())?;
        let new = self.tree.new_leaf(Style {
            // justify_items: Some(JustifyItems::Stretch),
            flex_grow: 1.0,
            ..Default::default()
        })?;

        style.flex_direction = direction;
        style.justify_content = Some(JustifyContent::Stretch);
        self.tree.set_style(parent, style)?;
        self.tree.add_child(parent, origin)?;
        self.tree.add_child(parent, new)?;

        self.tree.compute_layout(self.root, Size::max_content())?;
        Ok((origin, new))
    }

    pub fn vsplit(&mut self, parent: NodeId) -> anyhow::Result<(NodeId, NodeId)> {
        self.split(parent, FlexDirection::Row)
    }

    pub fn hsplit(&mut self, parent: NodeId) -> anyhow::Result<(NodeId, NodeId)> {
        self.split(parent, FlexDirection::Column)
    }
}
