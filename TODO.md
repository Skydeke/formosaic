- UnitTests for game + Engine (running in CIs and Locally)
    - UIStatemachine


- Better Main menu

- State saftety, disable buttons while loading level

- Some game specific logic made it to the engine. (Scramble is a game mechanic!)

- Visual center should be computed once, and stay

- Better animation system: From default pose to playin animation "jerks" 

- bug: Citizen 3 (model.blb in workspace) pose has eyes and head in a bit wrong location. (even without animation)
    - Could be because blender has some "default" pose applied.
    - Scramble: For animated model pick that default pose

- Fetch random should not pick levels we already have downloaded.

- Can we make better use of the sceneraph instead of collecting lists to render?

- Idiomatic Rust cleanup  *(most items addressed — see below)*
    - `Rc<RefCell<>>` scene/model sharing: kept where truly shared (scene graph, camera, UI state). Entity-internal model sharing via `Rc<RefCell<>>` is still needed because `Formosaic` also holds a reference for animation updates. The pre-resolved `RenderState` pattern removes the *need* to access the model through the entity's `RefCell` during rendering, but the sharing itself remains.
    - **Still TODO**: game-specific logic that leaked into the engine (Scramble is a game mechanic!), visual center caching, animation jerk on transition from default pose.
