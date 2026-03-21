- https://poly.pizza/explore 
    - Played levels have model and their credits downloaded locally, to replay as much as you want
    - Menu for selecting levels with preview of the finished 3d models.
- Puzzle scrambling is broken, Scramble should work in a way that if the correct angle is found on the 3d model,
    it should be possible to see teh entire model without any visible triangle seams. At the moment the
    triangles all seem to move together when the angle is found. That should not be necessary.
- Abstractions and architecture is a mess: 
    - formosaic.rs should have game logic,
    - pipeline.rs should only be generic.
    - Clear color and Sun / lights and co should be configurable from formosaic.rs
    - Lights should be placable in the scenegraph and handled by the shaders.
- Remove Re-Scramble button
- Move android buttons to the right side.
- Menu background color should be the same as in-game puzzle clear color.
