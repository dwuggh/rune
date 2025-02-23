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

use std::sync::{LazyLock, Mutex};

use crate::{
    core::{
        env::{intern, Env, INTERNED_SYMBOLS},
        gc::{Context, Rt, Slot},
        object::{LispFrame, Object, ObjectType, Symbol, NIL},
    },
    window::Window,
};
use anyhow::Result;
use rune_core::{hashmap::HashMap, macros::list};
use rune_macros::defun;
use taffy::prelude::*;

type FrameMap = HashMap<String, &'static LispFrame>;
pub(crate) static FRAMES: LazyLock<Mutex<FrameMap>> = LazyLock::new(Mutex::default);

#[derive(Debug)]
pub struct Frame {
    /* Name of this frame: a Lisp string.  It is used for looking up resources,
    as well as for the title in some cases.  */
    pub name: String,
    pub frame_id: u64,

    pub parent: Option<String>,

    // title: String,
    // parent_frame: Option<Arc<Frame>>,
    pub layout: FrameLayout,

    /// collection of windows. window id is converted from taffy's `NodeId`.
    pub windows: HashMap<u64, Window>,

    pub selected_window: u64,

    pub cursor_pos: usize,

    pub left: f32,
    pub top: f32,
}

#[derive(Debug)]
pub struct FrameLayout {
    pub tree: TaffyTree<()>,
    /// the outer frame
    pub root: NodeId,
    pub title_bar: NodeId,
    pub menu_bar: NodeId,
    pub tool_bar: NodeId,
    pub tab_bar: NodeId,
    /// inner frame
    pub main: NodeId,
}

impl FrameLayout {
    fn new(width: f32, height: f32) -> Self {
        let outer_frame_style = Style {
            size: Size { width: length(width), height: length(height) },
            border: Rect::length(2.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        };

        let title_bar_style = Style {
            size: Size { width: Dimension::Percent(1.0), height: length(30.0) },
            border: Rect::length(2.0),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };

        let menu_bar_style = Style {
            size: Size { width: Dimension::Percent(1.0), height: length(30.0) },
            border: Rect::length(2.0),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };

        let tool_bar_style = Style {
            size: Size { width: Dimension::Percent(1.0), height: length(30.0) },
            border: Rect::length(2.0),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };

        let tab_bar_style = Style {
            size: Size { width: Dimension::Percent(1.0), height: length(20.0) },
            border: Rect::length(2.0),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };

        let inner_frame_style = Style {
            size: Size { width: percent(1.), height: percent(1.) },
            border: Rect::length(2.0),
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..Default::default()
        };

        let mut tree = TaffyTree::new();
        let title_bar = tree.new_leaf(title_bar_style).unwrap();
        let menu_bar = tree.new_leaf(menu_bar_style).unwrap();
        let tool_bar = tree.new_leaf(tool_bar_style).unwrap();
        let tab_bar = tree.new_leaf(tab_bar_style).unwrap();
        let inner_frame = tree.new_leaf(inner_frame_style).unwrap();

        let outer_frame = tree
            .new_with_children(
                outer_frame_style,
                &[title_bar, menu_bar, tool_bar, tab_bar, inner_frame],
            )
            .unwrap();
        tree.compute_layout(outer_frame, max_content()).unwrap();

        FrameLayout {
            tree,
            root: outer_frame,
            title_bar,
            menu_bar,
            tool_bar,
            tab_bar,
            main: inner_frame,
        }
    }
}

impl FrameLayout {
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

impl Frame {
    pub fn new(width: f32, height: f32) -> Self {
        let layout = FrameLayout::new(width, height);
        Self {
            name: String::new(),
            frame_id: 0,
            parent: None,
            layout,
            windows: HashMap::default(),
            selected_window: 0,
            cursor_pos: 0,
            left: 0.,
            top: 0.,
        }
    }
}

#[defun]
fn framep(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Frame(_))
}

#[defun]
pub fn make_frame<'ob>(
    parameters: Option<Object>,
    cx: &'ob Context,
    env: &mut Rt<Env>,
) -> Result<Object<'ob>> {
    // TODO width and height should be obtained from parameters
    // and various default parameters
    let mut width = 800.0; // Default width
    let mut height = 600.0; // Default height

    // Create a new frame with the specified or default dimensions
    let frame = Frame::new(width, height);
    let lispframe: &'static LispFrame = {
        let global = INTERNED_SYMBOLS.lock().unwrap();
        let params = Slot::new(parameters.unwrap_or(NIL));
        let lispframe = LispFrame::create(frame, params, global.global_block());
        unsafe { &*(lispframe as *const LispFrame) }
    };

    FRAMES.lock().unwrap().insert(String::new(), lispframe).unwrap();
    let result = cx.add(lispframe);
    Ok(result)
}

/// NOTE this function is implemented in elisp because it is gui platform-specific.
///
/// Return geometric attributes of FRAME.
/// FRAME must be a live frame and defaults to the selected one.
#[defun]
fn frame_geometry<'ob>(
    frame: Option<Object>,
    cx: &'ob Context,
    env: &mut Rt<Env>,
) -> Result<Object<'ob>> {
    if let Some(f) = frame
        .and_then(|f| match f.untag() {
            ObjectType::Frame(f) => Some(f),
            _ => None,
        })
        .or(env.selected_frame)
    {
        let data = f.data();
        let outpos = data.layout.get(data.layout.root)?;
        let outer_position = list!(intern("outer-position", cx),
            outpos.location.x as i64,
            outpos.location.y as i64,
            ; cx);

        let outer_size = list!(intern("outer-size", cx),
            outpos.size.width as i64,
            outpos.size.height as i64,
            ; cx);

        let out_border = outpos.border;
        let external_border_size = list!(intern("external-border-size", cx),
            out_border.right as i64, out_border.bottom as i64, ; cx);

        let outer_border_width = list!(intern("outer-border-width", cx), 0, ; cx);

        let title_bar_size = list!(intern("title-bar-size", cx), 0, 0, ; cx);

        let menu_bar_external = list!(intern("menu-bar-external", cx), true, ; cx);

        let menu_bar_size = list!(intern("menu-bar-size", cx), 0, 0, ; cx);

        let tab_bar_size = list!(intern("tab-bar-size", cx), 0, 0, ; cx);

        let tool_bar_external = list!(intern("tool-bar-external", cx), true, ; cx);

        let tool_bar_position = list!(intern("tool-bar-position", cx), intern("top", cx), ; cx);

        let tool_bar_size = list!(intern("tool-bar-size", cx), 0, 0, ; cx);

        let internal_border_width = list!(intern("internal-border-width", cx), 0, ; cx);
        let a = list!(external_border_size, outer_border_width, title_bar_size, menu_bar_external, menu_bar_size, tab_bar_size, tool_bar_external, tool_bar_position, tool_bar_size, internal_border_width; cx);
        return Ok(a);
    }

    Ok(NIL)
}

#[defun]
fn modify_frame_parameters<'ob>(frame: Object<'ob>, parameters: Object<'ob>, cx: &'ob Context, env: &mut Rt<Env>) {
    todo!()
}


