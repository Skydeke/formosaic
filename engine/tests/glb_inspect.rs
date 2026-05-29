use std::cell::RefCell;
use std::rc::Rc;

/// Construct a minimal russimp-ng scene with mesh nodes that share names
/// with bones — the pattern that caused the skeleton-building bug.
fn build_scene() -> russimp_ng::scene::Scene {
    fn id() -> russimp_ng::Matrix4x4 {
        russimp_ng::Matrix4x4 {
            a1: 1.0, a2: 0.0, a3: 0.0, a4: 0.0,
            b1: 0.0, b2: 1.0, b3: 0.0, b4: 0.0,
            c1: 0.0, c2: 0.0, c3: 1.0, c4: 0.0,
            d1: 0.0, d2: 0.0, d3: 0.0, d4: 1.0,
        }
    }

    fn node(name: &str) -> Rc<russimp_ng::node::Node> {
        Rc::new(russimp_ng::node::Node {
            name: name.to_string(),
            children: RefCell::new(Vec::new()),
            meshes: Vec::new(),
            metadata: None,
            transformation: id(),
            parent: std::rc::Weak::new(),
        })
    }

    // Scene graph:
    //   RootNode
    //     ├── CharacterArmature
    //     │    └── Root
    //     │         └── Body  ← bone
    //     │              └── Neck
    //     │                   └── Head  ← bone
    //     ├── Body  ← mesh node (name collision!)
    //     └── Head  ← mesh node (name collision!)

    let root = node("RootNode");
    let armature = node("CharacterArmature");
    let bone_root = node("Root");
    let bone_body = node("Body");
    let bone_neck = node("Neck");
    let bone_head = node("Head");
    let mesh_body = node("Body");
    let mesh_head = node("Head");

    bone_neck.children.borrow_mut().push(bone_head.clone());
    bone_body.children.borrow_mut().push(bone_neck.clone());
    bone_root.children.borrow_mut().push(bone_body.clone());
    armature.children.borrow_mut().push(bone_root.clone());
    root.children.borrow_mut().push(armature.clone());
    root.children.borrow_mut().push(mesh_body.clone());
    root.children.borrow_mut().push(mesh_head.clone());

    // One mesh referencing the bones
    let ai_mesh = russimp_ng::mesh::Mesh {
        name: "TestMesh".into(),
        bones: vec![
            russimp_ng::bone::Bone {
                name: "Root".into(),
                offset_matrix: id(),
                weights: vec![russimp_ng::bone::VertexWeight { vertex_id: 0, weight: 1.0 }],
            },
            russimp_ng::bone::Bone {
                name: "Body".into(),
                offset_matrix: id(),
                weights: vec![russimp_ng::bone::VertexWeight { vertex_id: 1, weight: 1.0 }],
            },
            russimp_ng::bone::Bone {
                name: "Neck".into(),
                offset_matrix: id(),
                weights: vec![russimp_ng::bone::VertexWeight { vertex_id: 2, weight: 1.0 }],
            },
            russimp_ng::bone::Bone {
                name: "Head".into(),
                offset_matrix: id(),
                weights: vec![russimp_ng::bone::VertexWeight { vertex_id: 3, weight: 1.0 }],
            },
        ],
        ..Default::default()
    };

    russimp_ng::scene::Scene {
        root: Some(root),
        meshes: vec![ai_mesh],
        materials: Vec::new(),
        animations: Vec::new(),
        cameras: Vec::new(),
        lights: Vec::new(),
        metadata: None,
        flags: 0,
    }
}

fn print_scene_graph(node: &Rc<russimp_ng::node::Node>, indent: usize) {
    let prefix = "  ".repeat(indent);
    println!(
        "{}+- Node: {} ({} meshes, {} children)",
        prefix,
        node.name,
        node.meshes.len(),
        node.children.borrow().len()
    );
    for child in node.children.borrow().iter() {
        print_scene_graph(child, indent + 1);
    }
}

/// Find the armature root — the node whose child is `bone_name`.
fn find_armature_root(
    node: &Rc<russimp_ng::node::Node>,
    bone_name: &str,
) -> Option<Rc<russimp_ng::node::Node>> {
    for child in node.children.borrow().iter() {
        if child.name == bone_name {
            return Some(node.clone());
        }
    }
    for child in node.children.borrow().iter() {
        if let Some(found) = find_armature_root(child, bone_name) {
            return Some(found);
        }
    }
    None
}

/// Find a node's parent index within the name-to-index map,
/// searching only within a given subtree.
fn find_parent(
    node: &russimp_ng::node::Node,
    bone_name: &str,
    name_to_idx: &std::collections::HashMap<&str, usize>,
) -> Option<usize> {
    for child in node.children.borrow().iter() {
        if child.name == bone_name {
            return name_to_idx.get(node.name.as_str()).copied();
        }
    }
    for child in node.children.borrow().iter() {
        if let Some(idx) = find_parent(child, bone_name, name_to_idx) {
            return Some(idx);
        }
    }
    None
}

#[test]
fn inspect_skeleton_with_name_collisions() {
    let scene = build_scene();

    println!("=== SCENE GRAPH ===");
    if let Some(root) = &scene.root {
        print_scene_graph(root, 0);
    }

    // ── Simulate what build_skeleton does ──
    // Collect bone names and assign indices
    let mut bone_names_ordered: Vec<String> = Vec::new();
    for mesh in &scene.meshes {
        for bone in &mesh.bones {
            if !bone_names_ordered.contains(&bone.name) {
                bone_names_ordered.push(bone.name.clone());
            }
        }
    }

    let name_to_idx: std::collections::HashMap<&str, usize> = bone_names_ordered
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();

    // Find the armature root for the first bone
    let first_bone = bone_names_ordered.first().unwrap();
    let armature_root =
        scene
            .root
            .as_ref()
            .and_then(|root| find_armature_root(root, first_bone));

    // Use armature subtree if found, else fall back to scene root
    let search_root = armature_root.as_ref().or(scene.root.as_ref());

    println!("\n=== BONE HIERARCHY (armature-restricted lookup) ===");
    println!("Search root: {:?}", search_root.as_ref().map(|n| &n.name));
    for (idx, bone_name) in bone_names_ordered.iter().enumerate() {
        let parent = search_root
            .as_ref()
            .and_then(|root| find_parent(root, bone_name, &name_to_idx));
        println!("  Bone[{}]: name='{}', parent={:?}", idx, bone_name, parent);
    }

    // ── Verify correct hierarchy ──
    // Root has no parent
    assert_eq!(
        name_to_idx.get("Root").copied(),
        Some(0),
        "Root should be bone index 0"
    );
    let root_parent = search_root
        .as_ref()
        .and_then(|root| find_parent(root, "Root", &name_to_idx));
    assert_eq!(root_parent, None, "Root should have no parent");

    // Body's parent should be Root (index 0)
    let body_parent = search_root
        .as_ref()
        .and_then(|root| find_parent(root, "Body", &name_to_idx));
    assert_eq!(body_parent, Some(0), "Body parent should be Root (index 0)");

    // Head's parent should be Neck
    let head_parent = search_root
        .as_ref()
        .and_then(|root| find_parent(root, "Head", &name_to_idx));
    let neck_idx = name_to_idx.get("Neck").copied();
    assert_eq!(
        head_parent,
        neck_idx,
        "Head parent should be Neck, not None (mesh node collision)"
    );
}
