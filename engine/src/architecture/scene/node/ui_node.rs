//! `UiNode` — an ImGui panel as a first-class scene-graph node.
//!
//! # Motivation
//!
//! Previously the game had to implement `Application::build_ui` and contain
//! all ImGui logic in one monolithic function.  With `UiNode` you instead
//! attach named, show/hide-able UI panels directly to the scene graph — the
//! engine collects them automatically every frame.
//!
//! # Usage (game side)
//!
//! ```rust,ignore
//! use formosaic_engine::architecture::scene::node::ui_node::UiNode;
//!
//! // Create a HUD node
//! let hud = UiNode::new("hud", move |ui, w, h, ctx| {
//!     ui.window("##hud")
//!         .position([w - 220.0, 10.0], imgui::Condition::Always)
//!         .size([210.0, 80.0], imgui::Condition::Always)
//!         .build(|| {
//!             ui.text("Hello from scenegraph!");
//!         });
//! });
//!
//! // Add it to the scene
//! scene.add_node(Rc::new(RefCell::new(hud)));
//!
//! // Hide/show at runtime
//! hud_ref.borrow_mut().set_hidden(true);
//! ```
//!
//! The engine's `GameEngine` calls `UiNode::collect_and_build` once per frame,
//! which walks the scene graph, finds every visible `UiNode`, and invokes
//! each one's callback inside the current imgui frame.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use rand::Rng;

use crate::architecture::scene::node::node::{NodeBehavior, NodeChildren};
use crate::architecture::scene::node::transform::Transform;
use crate::architecture::scene::node::scenegraph::Scenegraph;
use crate::architecture::scene::scene_context::SceneContext;

// ─── Callback type ────────────────────────────────────────────────────────────

/// Signature of a UI-building callback.
///
/// `ui`  — the live imgui `Ui` frame handle.
/// `w/h` — logical (unscaled) viewport dimensions.
/// `ctx` — mutable scene context (read game state, fire events, etc.).
pub type UiCallback = Box<dyn FnMut(&imgui::Ui, f32, f32, &mut SceneContext)>;

// ─── UiNode ──────────────────────────────────────────────────────────────────

/// A named, show/hide-able ImGui panel living in the scene graph.
pub struct UiNode {
    uuid:      u32,
    name:      String,
    hidden:    bool,
    transform: Transform,
    children:  Vec<Rc<RefCell<dyn NodeBehavior>>>,
    callback:  UiCallback,
}

impl UiNode {
    /// Create a `UiNode` with the given name and UI-building callback.
    pub fn new<F>(name: impl Into<String>, callback: F) -> Self
    where
        F: FnMut(&imgui::Ui, f32, f32, &mut SceneContext) + 'static,
    {
        Self {
            uuid:      rand::rng().random(),
            name:      name.into(),
            hidden:    false,
            transform: Transform::new(),
            children:  Vec::new(),
            callback:  Box::new(callback),
        }
    }

    /// Invoke the callback — called by the engine inside the imgui frame.
    pub fn build(&mut self, ui: &imgui::Ui, w: f32, h: f32, ctx: &mut SceneContext) {
        if !self.hidden {
            (self.callback)(ui, w, h, ctx);
        }
    }

    // ── Scene-graph collection ────────────────────────────────────────────

    /// Walk `scenegraph`, collect every visible `UiNode`, and call each one.
    ///
    /// Must be called inside an active imgui frame.
    pub fn collect_and_build(
        scenegraph: &Scenegraph,
        ui: &imgui::Ui,
        w: f32,
        h: f32,
        ctx: &mut SceneContext,
    ) {
        let nodes = scenegraph.collect_nodes_of_type::<UiNode>();
        for node_rc in nodes {
            // Cast to UiNode and call — safe because collect_of_type already
            // verified the type via as_any().is::<UiNode>().
            let mut borrow = node_rc.borrow_mut();
            if let Some(ui_node) = borrow.as_any_mut().downcast_mut::<UiNode>() {
                if !ui_node.is_hidden() {
                    (ui_node.callback)(ui, w, h, ctx);
                }
            }
        }
    }
}

// ─── NodeBehavior impl ────────────────────────────────────────────────────────

impl NodeBehavior for UiNode {
    fn get_uuid(&self)  -> u32   { self.uuid }
    fn is_hidden(&self) -> bool  { self.hidden }

    fn set_hidden(&mut self, hidden: bool) { self.hidden = hidden; }

    fn get_name(&self) -> String { self.name.clone() }

    fn transform(&self)     -> &Transform     { &self.transform }
    fn transform_mut(&mut self) -> &mut Transform { &mut self.transform }

    fn as_any(&self) -> &dyn Any { self }

    /// Needed for downcasting inside `collect_and_build`.
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl NodeChildren for UiNode {
    fn add_child_impl(
        &mut self,
        parent: Rc<RefCell<dyn NodeBehavior>>,
        child:  Rc<RefCell<dyn NodeBehavior>>,
    ) {
        child.borrow_mut().transform_mut().set_parent(Some(Rc::downgrade(&parent)));
        self.children.push(child);
    }

    fn get_children_impl(&self) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        self.children.clone()
    }
}
