# Avian Physics Sandbox

## Prototype Specification

**Engine:** Bevy 0.19
**Physics:** Avian3D 0.7
**Language:** Rust
**Perspective:** Top-down / isometric 3D
**Purpose:** Technical and gameplay feasibility prototype

---

# 1. Objective

Create a small interactive 3D sandbox game whose primary purpose is to explore and evaluate Avian physics for a future physics-heavy sandbox game.

The prototype is not intended to be a polished game.

It should make it easy to:

* move a player around a 3D environment;
* select objects using the mouse;
* pick up and manipulate physical objects;
* push, throw, stack and knock over objects;
* modify physical properties at runtime;
* test different collider shapes;
* test rigid-body types;
* test forces and impulses;
* test collision events and sensors;
* test collision filtering;
* test all built-in Avian joint types;
* test spatial queries;
* test CCD;
* test sleeping;
* test physics timestep and substeps;
* inspect physics using debug visualization;
* stress-test many bodies;
* experiment with game-specific fake physics such as wind and breakable objects.

The final result should function as a **physics laboratory that happens to control like the intended game**.

---

# 2. Core Principle

Do not attempt to build the final game's systems.

The prototype exists to answer:

> Can Avian provide the tactile object interaction required by the game?

The main question is not whether the simulation is realistic.

The desired result is:

* understandable;
* predictable;
* stable;
* tactile;
* fun to manipulate;
* easy to customize.

Game-specific physics may intentionally override physically accurate behavior.

---

# 3. Prototype World

Create a single test map divided into multiple clearly identifiable areas.

Example layout:

```text
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   A                  B                   C                  │
│ Rigid Bodies      Materials           Colliders             │
│                                                             │
│                                                             │
│   D                  E                   F                  │
│ Forces            Joints             Sensors                │
│                                                             │
│                                                             │
│   G                  H                   I                  │
│ Spatial Queries    CCD               Stress Test             │
│                                                             │
│                                                             │
│                    J                                        │
│              Free Sandbox                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

The player can freely walk between areas.

No level loading should be required.

Each station should have:

* a sign or floating text identifying the tested feature;
* physical objects ready to interact with;
* optional buttons/toggles;
* reset functionality.

---

# 4. Camera

Use a perspective camera with a fixed top-down/isometric orientation.

Approximate orientation:

```text
        camera
           \
            \
             \
              ↓
        ┌──────────┐
        │  world   │
        └──────────┘
```

Camera requirements:

* tilted downward;
* follows the player;
* smooth positional interpolation;
* does not rotate during normal gameplay;
* mouse remains available for world interaction.

Optional debug controls:

* mouse wheel: zoom;
* middle mouse drag: rotate camera;
* key to reset camera orientation.

Camera debug controls must be separable from normal game controls.

---

# 5. Player

Represent the player using a simple shape.

For example:

```text
   ___
 /     \
|       |
|       |
 \_____/
```

A capsule is recommended.

No character art or animation is required.

## Movement

Use:

```text
WASD
```

or:

```text
W
A S D
```

movement projected onto the ground plane relative to camera orientation.

The player should:

* collide with walls;
* collide with static geometry;
* be able to push lightweight dynamic objects;
* not easily be knocked over by objects;
* move predictably when surrounded by physics bodies.

Prefer a controlled/kinematic character solution rather than allowing the player to behave as an uncontrolled dynamic rigid body.

---

# 6. Mouse Interaction

Mouse interaction is one of the most important parts of the prototype.

## Hover

Raycast from the camera through the cursor.

A hovered physical object should be highlighted.

Display basic information:

```text
Wooden Crate

Mass: 4.0 kg
Body: Dynamic
Collider: Cuboid
Friction: 0.8
Restitution: 0.1
Sleeping: No
```

---

# 7. Picking Up Objects

Left-click and hold on a dynamic object to grab it.

Do NOT teleport objects directly to the cursor.

Instead, create a target point in the world and pull the object toward it.

Conceptually:

```text
mouse cursor
     │
     │ ray
     ↓

 grab target ●

        ↖ force

        [object]
```

Use a spring-like interaction.

Conceptual force:

```text
force =
    displacement * stiffness
    - velocity * damping
```

The exact implementation can use whichever Avian mechanism produces the best result.

Important behavior:

* grabbed objects retain physics;
* grabbed objects collide with the environment;
* grabbed objects collide with other objects;
* heavy objects should be harder to move;
* objects should lag behind the cursor slightly;
* rapid mouse movement should create momentum;
* releasing the mouse should release the object naturally.

This system is one of the main feasibility tests.

---

# 8. Grab Distance

The player should not be able to interact with objects anywhere on screen.

Define:

```text
MAX_INTERACTION_DISTANCE
```

Example:

```text
3–5 metres
```

Objects outside this radius can still be hovered but cannot be grabbed.

Display a different highlight for unreachable objects.

---

# 9. Throwing

While holding an object:

```text
Left Mouse Release
```

normally drops it.

Optional:

```text
Right Mouse
```

or:

```text
Space
```

throws the object.

Throw strength can depend on:

* recent grab-target velocity;
* configured throw strength;
* object mass.

This is useful for testing:

* impulses;
* CCD;
* collisions;
* restitution;
* breakage.

---

# 10. Physics Inspector

The selected object should expose a small debug UI.

Editable properties:

```text
Rigid Body
[ Dynamic ▼ ]

Mass
[ 5.0 ]

Gravity Scale
[ 1.0 ]

Linear Damping
[ 0.2 ]

Angular Damping
[ 1.0 ]

Friction
[ 0.7 ]

Restitution
[ 0.1 ]

Lock Translation X [ ]
Lock Translation Y [ ]
Lock Translation Z [ ]

Lock Rotation X [ ]
Lock Rotation Y [ ]
Lock Rotation Z [ ]

CCD [ ]

Sleeping Disabled [ ]

Dominance
[ 0 ]
```

Values should update immediately where Avian supports runtime changes.

The inspector is a development tool, not final game UI.

---

# 11. Test Area A — Rigid Bodies

Create objects representing:

### Static

Example:

```text
floor
walls
ramps
platforms
```

### Dynamic

Example:

```text
cube
sphere
barrel
crate
pot
```

### Kinematic

Create a moving platform.

Example:

```text
        ←──────→

    ┌──────────────┐
    │   platform   │
    └──────────────┘
```

The platform repeatedly moves between two positions.

Test:

* dynamic objects standing on it;
* player interaction;
* contacts;
* transform updates.

---

# 12. Velocity Testing

Add launch buttons that give objects predefined:

* linear velocity;
* angular velocity.

Example:

```text
[Launch Cube]
[Spin Cube]
[Launch + Spin]
```

Allow current velocity values to appear in the inspector.

---

# 13. Force Testing

Have buttons demonstrating:

* persistent force;
* impulse;
* torque;
* angular impulse;
* acceleration.

Example:

```text
         ↑ continuous force
         │
      ┌─────┐
      │ box │
      └─────┘
```

Controls:

```text
Apply Force
Apply Impulse
Apply Torque
Apply Angular Impulse
```

Use visually identical objects to make differences obvious.

---

# 14. Gravity

Global UI:

```text
Gravity

X [ 0.0 ]
Y [-9.81]
Z [ 0.0 ]

[ Earth ]
[ Moon ]
[ Zero G ]
[ Reverse ]
```

Individual objects should also test `GravityScale`.

Examples:

```text
Object A: 1.0
Object B: 0.5
Object C: 0.0
Object D: -1.0
```

---

# 15. Damping

Create several spinning or sliding objects with different damping values.

Example:

```text
Low damping

──────────────→


Medium damping

──────→


High damping

──→
```

Test independently:

* linear damping;
* angular damping.

---

# 16. Locked Axes

Create demonstrations for:

```text
translation X locked
translation Y locked
translation Z locked

rotation X locked
rotation Y locked
rotation Z locked
```

A particularly useful test is an object that can only move along one axis.

Example:

```text
──────────── rail ────────────

          [ BLOCK ]

              ↔
```

---

# 17. Dominance

Create a station demonstrating rigid-body dominance.

Use objects with clear visual labels:

```text
Dominance 0

Dominance 5
```

Allow them to collide so the behavioral difference can be observed.

---

# 18. Test Area B — Physics Materials

Create several lanes.

## Friction

Same shape and mass.

Different friction.

Example:

```text
ICE
friction ≈ 0

WOOD
medium friction

RUBBER
high friction
```

Create an inclined ramp.

Objects should slide different distances.

---

# 19. Restitution

Drop identical balls onto surfaces with different restitution values.

Example:

```text
       O       O       O
       ↓       ↓       ↓

     stone   wood    rubber
```

Demonstrate:

```text
0.0
0.5
1.0
```

or appropriate stable values.

---

# 20. Density and Mass

Create identical-looking objects using different densities.

Example:

```text
FOAM       WOOD       STONE

[ cube ]   [ cube ]   [ cube ]

light      medium     heavy
```

The player should be able to feel the difference while grabbing and pushing them.

---

# 21. Test Area C — Collider Shapes

Create visible examples of the major collider shapes useful to the game.

At minimum:

```text
Sphere
Cuboid
Capsule
Cylinder
Cone
```

Also include supported complex collider forms where useful.

Enable Avian collider debug rendering.

The user must be able to see the difference between:

```text
visual mesh
```

and:

```text
physics collider
```

---

# 22. Compound Objects

Create an object made from multiple colliders.

Example:

```text
     ┌─────┐
     │     │
─────┴─────┴─────
```

Potential example:

* table;
* chair;
* shelf;
* irregular tool.

Use child colliders attached to the same rigid body where appropriate.

---

# 23. Mesh Collider

Include at least one irregular mesh.

Generate its collider using Avian's mesh collider functionality.

Example:

```text
rock
stairs
irregular sculpture
```

Test objects falling and sliding on the generated collider.

---

# 24. Scene Collider Generation

Create or load one simple Bevy scene containing several meshes.

Use Avian's collider hierarchy/scene collider generation functionality.

This station verifies that imported game assets can conveniently receive physics colliders.

---

# 25. Test Area D — Stacking

Create many objects:

```text
cube
brick
plank
cylinder
sphere
```

Allow free construction.

The player should be able to make things like:

```text
        [ ]
      [ ][ ]
    [ ][ ][ ]
```

and:

```text
   ┌────────────┐
   │   plank    │
   └────────────┘
     █        █
     █        █
```

Evaluate:

* jitter;
* stacking stability;
* sliding;
* tipping;
* sleeping;
* performance.

---

# 26. Test Area E — Joints

Provide one demonstration for every built-in Avian joint type.

---

## 26.1 Fixed Joint

Two objects behave as one connected structure.

Example:

```text
[BOX]──[BOX]
```

Test:

* pushing;
* throwing;
* collisions;
* heavy loads.

---

## 26.2 Distance Joint

Objects remain within a configured distance.

Example:

```text
[Ball]──────[Ball]
```

Use this to approximate:

* rope-like behavior;
* hanging signs;
* connected objects.

---

## 26.3 Revolute Joint

Create a hinge.

Example:

```text
WALL │ DOOR
     │
     ●────────
```

Potential gameplay relevance:

* doors;
* gates;
* rotating mechanisms.

---

## 26.4 Prismatic Joint

Object moves along one axis.

Example:

```text
───────────────

    [ BLOCK ]
       ↔
```

Potential gameplay relevance:

* drawer;
* sliding door;
* piston;
* machine mechanism.

---

## 26.5 Spherical Joint

Create a hanging object capable of rotating freely around an anchor.

Example:

```text
 ceiling
──────────
    |
    ●
    |
  [lamp]
```

Potential gameplay relevance:

* lamps;
* suspended objects;
* loose attachments.

---

# 27. Joint Runtime Controls

Selecting a joint should display:

```text
Joint Type

Connected Entity A
Connected Entity B

Anchor A
Anchor B

Compliance

Damping

Force / stress information where available

Enabled
```

Allow the joint to be:

```text
[ Disable ]
[ Enable ]
[ Delete ]
```

This verifies runtime joint manipulation.

---

# 28. Joint Creation Tool

Add a simple debug tool.

Workflow:

```text
Press J

click object A
click object B

choose:

Fixed
Distance
Revolute
Prismatic
Spherical
```

Create the joint.

This is especially valuable for exploring the future object-composition mechanic.

---

# 29. Test Area F — Sensors

Create invisible or transparent sensor zones.

Example:

```text
┌─────────────────────┐
│                     │
│       SENSOR        │
│                     │
└─────────────────────┘
```

When an object enters:

```text
Object entered sensor
```

When it leaves:

```text
Object left sensor
```

Display events in an on-screen event log.

Potential game uses:

* interaction zones;
* workshop stations;
* storage areas;
* water detection;
* NPC triggers;
* object placement areas.

---

# 30. Collision Events

Maintain a small event log.

Example:

```text
Physics Events

Cube → Floor ENTER
Cube → Floor CONTACT
Cube → Floor EXIT

Player → Crate ENTER

Ball → Sensor ENTER
```

Allow the log to be cleared.

---

# 31. Contact Information

When selecting an object involved in a collision, optionally display contact data such as:

```text
Other Entity
Contact Position
Normal
Relative Speed
Impulse
```

This will be useful for later systems such as breakage.

---

# 32. Collision Layers

Create several categories:

```text
Player
World
Objects
Sensors
Projectiles
Ghost
```

Demonstrate collision filtering.

Example:

```text
red ball:
collides with wall A
passes through wall B

blue ball:
passes through wall A
collides with wall B
```

---

# 33. Collision Hooks

Include at least one demonstration of collision filtering or modification using Avian collision hooks.

Example gameplay rule:

```text
Ghost objects pass through normal objects
until activated.
```

Another possible example:

```text
One-way barrier
```

The purpose is not to create production-quality gameplay but to verify that custom collision rules are practical.

---

# 34. Temporarily Disable Collider

Selected object inspector:

```text
Collider Enabled [x]
```

When disabled:

* rigid body may continue moving;
* object should pass through other colliders.

Re-enable at runtime.

---

# 35. Temporarily Disable Rigid Body

Selected object inspector:

```text
Physics Enabled [x]
```

Verify the behavior of temporarily disabling and restoring body simulation.

---

# 36. Test Area G — Spatial Queries

Create a dedicated test area for Avian's query functionality.

---

# 37. Raycast

Display the player's mouse ray.

Example:

```text
camera
   \
    \
     \──────────────X
                  hit
```

Show:

```text
entity
distance
hit position
surface normal
```

This should also power object selection.

---

# 38. RayCaster Component

Create at least one continuously active ray caster attached to an entity.

Example:

```text
robot/object
     |
     ↓
 raycast
     |
   ground
```

Use it to demonstrate continuously updated query results.

---

# 39. Shape Cast

Allow a sphere/capsule shape to sweep through the world.

Example:

```text
(start) O ─────────────→ O (end)
               X
            collision
```

Visualize:

* cast start;
* path;
* first hit;
* hit normal.

---

# 40. ShapeCaster Component

Create one moving entity using an attached shape caster.

Potential future relevance:

* character movement;
* object placement;
* obstacle detection.

---

# 41. Point Projection

Allow clicking anywhere in the environment.

Display the closest point on a collider.

Example:

```text
cursor ●

        ↓

      [ cube ]
        × nearest point
```

---

# 42. Intersection Test

Create a movable debug volume.

Color it:

```text
green = no intersection
red   = intersecting
```

Use this to evaluate potential future object-placement validation.

---

# 43. Spatial Query Filters

Add UI allowing query filtering by collision layer.

Example:

```text
Raycast against:

[x] World
[x] Objects
[ ] Player
[ ] Sensors
```

---

# 44. Test Area H — CCD

Create a launcher.

Without swept CCD:

```text
small ball ───────────────→ | thin wall |
```

High velocity should make tunneling easy to observe where possible.

Controls:

```text
Speed: 10 / 50 / 100 / 500

CCD:
[ Speculative ]
[ Swept Linear ]
[ Swept NonLinear ]
```

Demonstrate the differences between collision approaches.

---

# 45. Rotational CCD Test

Create a long rotating body.

Example:

```text
       |
       |
───────●───────
       |
       |
```

Rotate it very quickly near another collider.

Compare linear and nonlinear swept CCD where appropriate.

---

# 46. Sleeping

Create a pile of bodies.

Display:

```text
awake objects
sleeping objects
```

Optionally change their debug appearance based on state.

Controls:

```text
Wake All
Sleep All
Disable Sleeping
Enable Sleeping
```

Expose:

```text
sleep threshold
time to sleep
```

where practical.

---

# 47. Physics Time Controls

Global debug panel:

```text
Physics

[ Pause ]
[ Resume ]
[ Step ]

Speed
0.25x
0.5x
1x
2x
```

Single-step should advance physics one simulation step while paused.

This is very useful for debugging collisions and joints.

---

# 48. Substeps

Provide runtime configuration for physics substeps.

Example:

```text
Substeps

1
2
4
8
```

Run the same unstable scenario at different settings.

Possible scenario:

```text
stack of thin objects
```

or:

```text
jointed hanging structure
```

Observe stability/performance trade-offs.

---

# 49. Interpolation / Extrapolation

Create fast-moving objects that make rendering smoothness easy to observe.

Provide toggles/configuration allowing the developer to evaluate Avian's transform interpolation/extrapolation behavior.

The test should help determine an appropriate setup for the final game camera and rendering loop.

---

# 50. Physics Debug Rendering

Add global hotkey:

```text
F1
```

Toggle Avian physics debug visualization.

Show at least:

* colliders;
* contact points where available;
* spatial queries where useful;
* physics-related gizmos supported by Avian.

The game should remain playable with debug visualization enabled.

---

# 51. Diagnostics

Include Avian physics diagnostics.

Debug HUD should contain at minimum:

```text
FPS

Dynamic bodies
Sleeping bodies
Collider count
Contact count

Physics time
Collision detection time
Solver time

Total entities
```

Use Avian diagnostics where available rather than duplicating engine metrics.

---

# 52. Test Area I — Stress Test

Create a body-spawning station.

Buttons:

```text
Spawn 10 Cubes
Spawn 100 Cubes
Spawn 500 Cubes
Spawn 1000 Cubes

Clear
```

Objects spawn above a container:

```text
      [] [] []
    [] [] [] []
   ↓ ↓ ↓ ↓ ↓ ↓

┌─────────────────┐
│                 │
│       pit       │
│                 │
└─────────────────┘
```

Measure:

* FPS;
* physics simulation time;
* sleeping effectiveness;
* contact count.

The goal is not a specific performance number.

The goal is to understand practical limits.

---

# 53. Stress-Test Object Types

Allow spawning:

```text
Cubes
Spheres
Mixed
Compound objects
```

Optional:

```text
Joint chains
```

---

# 54. Physics Picking

Test Avian's Bevy picking integration separately from the custom gameplay grab system.

The prototype should make it clear which mechanism is responsible for:

```text
mouse entity selection
```

and which is responsible for:

```text
physical grabbing
```

This allows evaluating whether Avian/Bevy picking should become part of the final interaction architecture.

---

# 55. Free Sandbox

The final area contains:

* flat floor;
* ramps;
* stairs;
* walls;
* shelves;
* tables;
* platforms.

Object spawning palette:

```text
Cube
Sphere
Cylinder
Capsule
Plank
Heavy Cube
Light Cube
Bouncy Ball
High-Friction Block
```

The player should be able to freely experiment.

---

# 56. Object Spawner

Press:

```text
Tab
```

to open a simple developer palette.

Example:

```text
SPAWN

[ Cube ]
[ Ball ]
[ Plank ]
[ Barrel ]
[ Heavy Block ]
[ Bouncy Ball ]
```

Spawn at:

```text
mouse world position
```

or near the player.

---

# 57. Object Deletion

Select an object and press:

```text
Delete
```

to remove it.

Add:

```text
Clear spawned objects
```

to the debug menu.

---

# 58. Scene Reset

Press:

```text
R
```

to reset the current test station.

Use:

```text
Shift + R
```

to reset the entire sandbox.

Avoid requiring application restart during experimentation.

---

# 59. Game-Specific Experiment — Wind

This is not an Avian feature itself.

Implement wind by applying forces to eligible rigid bodies.

Component:

```rust
WindAffected {
    factor: f32,
}
```

Concept:

```text
force =
wind_direction
× wind_strength
× wind_factor
```

Objects:

```text
Paper      3.0
Wood       1.0
Ceramic    0.3
Stone      0.05
```

Create a wind tunnel.

```text
FAN

>>>>>>>>>>

paper
crate
stone
```

Global controls:

```text
Wind Strength
Wind Direction
Wind Enabled
```

This evaluates how well custom gameplay physics can sit on top of Avian.

---

# 60. Game-Specific Experiment — Breakable Objects

Implement simple impact-based breakage.

Do NOT simulate actual fracture.

Component:

```rust
Breakable {
    impulse_threshold: f32,
}
```

When collision impulse exceeds threshold:

```text
whole pot
   ↓

  impact

   ↓

several predefined pieces
```

Create:

```text
Ceramic Pot
Wooden Box
Stone Object
```

with different thresholds.

This is an important experiment for the intended game.

---

# 61. Game-Specific Experiment — Basic Containers

No fluid simulation is required.

Implement:

```rust
LiquidContainer {
    capacity: f32,
    amount: f32,
}
```

Example:

```text
Bucket
capacity: 10 L
amount: 6 L
```

Display amount visually or as UI text.

The purpose is simply to establish that physical objects can also hold gameplay state.

Full fluid mechanics are explicitly outside this prototype.

---

# 62. Game-Specific Experiment — Object Composition

Create a primitive construction mode.

Two physical objects can be selected and connected using a fixed joint.

Workflow:

```text
Build Mode

click object A

click object B

[ Connect ]
```

Example:

```text
plank + blocks

   ┌────────────┐
   │   plank    │
   └────────────┘
       │   │
       │   │
```

The resulting structure should remain fully physical.

This is one of the most important prototype experiments because it approximates the future game's object-composition system.

---

# 63. Material Gameplay Component

Separate gameplay material properties from Avian's physics components.

Example:

```rust
enum GameMaterial {
    Wood,
    Ceramic,
    Stone,
    Metal,
    Glass,
}
```

A material can configure defaults for:

```text
density
friction
restitution
break threshold
wind response
```

Example conceptual configuration:

```text
WOOD

density         medium
friction        high
restitution     low
breakable       medium
wind response   medium
```

This separation is important.

Avian should simulate the object.

The game's material system should decide how the object behaves conceptually.

---

# 64. Architecture

Recommended structure:

```text
src/

main.rs

game/
    mod.rs

    camera.rs
    input.rs
    player.rs

physics/
    mod.rs

    setup.rs
    grabbing.rs
    picking.rs
    forces.rs
    collisions.rs
    joints.rs
    queries.rs

sandbox/
    mod.rs

    rigid_body_lab.rs
    material_lab.rs
    collider_lab.rs
    joint_lab.rs
    sensor_lab.rs
    query_lab.rs
    ccd_lab.rs
    stress_lab.rs
    free_lab.rs

gameplay/
    mod.rs

    wind.rs
    breakable.rs
    materials.rs
    containers.rs
    composition.rs

debug/
    mod.rs

    ui.rs
    inspector.rs
    diagnostics.rs
```

Do not over-engineer this structure.

Modules may be combined if individual files remain small.

---

# 65. Core Components

Potential project components:

```rust
Player

Selectable

Selected

Grabbable

Grabbed

SpawnedObject

Resettable

GameMaterial

WindAffected

Breakable

LiquidContainer

TestStation
```

Avoid wrapping every Avian component behind custom abstractions.

The purpose of the prototype is to become familiar with Avian itself.

---

# 66. Physics Configuration

Start with conservative game-oriented physics.

Suggested starting assumptions:

```text
Precision
f32

Physics rate
60 Hz

Gravity
-9.81 m/s²

Substeps
1 initially

Linear damping
small

Angular damping
moderate

Restitution
generally low

Sleeping
enabled

Speculative collision
enabled
```

Adjust based on experimentation.

---

# 67. Dependencies

Baseline:

```toml
[dependencies]
bevy = "0.19"
avian3d = "0.7"
```

Enable additional Avian feature flags required for diagnostics/debug tooling.

Useful Avian functionality to consider enabling includes:

```text
debug-plugin
bevy_picking
collider-from-mesh
bevy_scene
bevy_diagnostic
diagnostic_ui
xpbd_joints
```

Use `f32` for this prototype.

Do not use `f64` unless a concrete issue demonstrates a need for it.

---

# 68. Debug UI

A persistent debug UI is important.

Suggested layout:

```text
┌──────────────────────┐
│ Physics Sandbox      │
│                      │
│ FPS: 144             │
│ Bodies: 182          │
│ Sleeping: 143        │
│ Contacts: 48         │
│ Physics: 1.2 ms      │
│                      │
│ [Pause] [Step]       │
│                      │
│ Gravity: -9.81       │
│ Substeps: 1          │
│                      │
│ Debug Draw [x]       │
└──────────────────────┘
```

Selected object inspector can appear on the opposite side.

---

# 69. Recommended Controls

```text
WASD
Move

Mouse
Aim / world interaction

Left Mouse
Grab / manipulate object

Right Mouse
Throw or alternate interaction

Mouse Wheel
Change grab distance or camera zoom

E
Interact

J
Joint tool

Delete
Delete selected object

Tab
Spawn menu

F1
Physics debug rendering

F2
Diagnostics

F3
Inspector

Space
Pause physics

.
Single physics step while paused

R
Reset current station

Shift + R
Reset sandbox

Esc
Menu
```

Exact controls may change during implementation.

---

# 70. Feature Checklist

The prototype is considered complete when the following can all be demonstrated.

## Rigid Bodies

* [ ] Static bodies
* [ ] Dynamic bodies
* [ ] Kinematic bodies
* [ ] Linear velocity
* [ ] Angular velocity
* [ ] Forces
* [ ] Impulses
* [ ] Acceleration
* [ ] Torque
* [ ] Gravity
* [ ] Gravity scale
* [ ] Mass
* [ ] Density
* [ ] Linear damping
* [ ] Angular damping
* [ ] Locked translation axes
* [ ] Locked rotation axes
* [ ] Dominance
* [ ] Sleeping
* [ ] Disable/enable body
* [ ] CCD
* [ ] interpolation/extrapolation

## Collisions

* [ ] Primitive colliders
* [ ] Compound colliders
* [ ] Mesh collider generation
* [ ] Scene collider generation
* [ ] Friction
* [ ] Restitution
* [ ] Collision layers
* [ ] Sensors
* [ ] Collision events
* [ ] Colliding entity access
* [ ] Contact information
* [ ] Collision hooks
* [ ] Disable/enable collider

## Joints

* [ ] Fixed
* [ ] Distance
* [ ] Revolute
* [ ] Prismatic
* [ ] Spherical
* [ ] Runtime joint creation
* [ ] Runtime joint deletion
* [ ] Runtime disable/enable
* [ ] Joint damping / force information where useful

## Spatial Queries

* [ ] Raycast
* [ ] RayCaster
* [ ] Shape cast
* [ ] ShapeCaster
* [ ] Point projection
* [ ] Intersection tests
* [ ] Query filters

## Simulation

* [ ] Physics pause
* [ ] Physics resume
* [ ] Single stepping
* [ ] Physics speed
* [ ] Substeps
* [ ] Physics diagnostics
* [ ] Physics debug rendering

## Integration

* [ ] Bevy physics picking
* [ ] Mouse world picking
* [ ] Physics inspector
* [ ] Runtime object spawning

## Game Experiments

* [ ] Mouse spring grabbing
* [ ] Throwing
* [ ] Stacking
* [ ] Wind
* [ ] Breakable object
* [ ] Simplified liquid container
* [ ] Object composition
* [ ] Gameplay material definitions

---

# 71. Optional Advanced Tests

These are useful but not required before evaluating the prototype.

## Custom XPBD Constraint

Implement one very simple custom constraint to understand Avian's extensibility.

Do this only after all built-in joints work.

---

## Serialization

Experiment with saving:

```text
object type
position
rotation
material
gameplay properties
```

and reloading the sandbox.

Full physics-state serialization is not required.

---

## Determinism

Run the same predefined test multiple times and compare results.

Optionally experiment with Avian's enhanced determinism feature.

This is low priority unless deterministic simulation becomes important to the final game.

---

## Parallel Physics

Stress-test with parallel physics enabled and disabled.

Measure whether it meaningfully affects the expected scale of the future game.

---

## Validation Mode

During development, experiment with Avian's additional validation feature if useful for diagnosing incorrect physics configurations.

---

# 72. Explicit Non-Goals

Do NOT implement:

* real fluid simulation;
* deformable bodies;
* realistic destruction;
* cloth simulation;
* aerodynamic simulation;
* realistic structural engineering;
* realistic character locomotion;
* combat;
* inventory;
* crafting;
* NPCs;
* quests;
* saving a real game;
* production UI;
* production art;
* multiplayer.

Do not allow feature creep into these systems.

---

# 73. Visual Style

Everything should use simple primitives.

Example:

```text
Player
capsule

Crate
cube

Ball
sphere

Plank
long cuboid

Pot
cylinder / simple mesh

World
large cuboids
```

Use simple colors to communicate function.

For example:

```text
Static          gray
Dynamic         light neutral
Kinematic       blue
Sensor          transparent
Selected        yellow outline
Sleeping        darker
Query           bright debug line
Joint           debug line
```

Actual art direction is irrelevant.

---

# 74. Development Order

Implement incrementally.

## Milestone 1 — World

Implement:

```text
Bevy application
Avian
camera
floor
player
movement
```

Success condition:

> Player can walk around a simple 3D world.

---

## Milestone 2 — Basic Physics

Implement:

```text
dynamic cubes
spheres
collisions
gravity
pushing
debug collider rendering
```

Success condition:

> Player can walk into objects and objects respond physically.

---

## Milestone 3 — Mouse Interaction

Implement:

```text
cursor raycast
hover
selection
grab
drag
drop
throw
```

Success condition:

> Manipulating objects feels enjoyable enough to justify continuing with Avian.

This is the first major go/no-go milestone.

---

## Milestone 4 — Physics Inspector

Implement:

```text
mass
friction
restitution
gravity scale
damping
body type
velocity
```

Success condition:

> Physics parameters can be experimented with without recompiling.

---

## Milestone 5 — Physics Laboratories

Implement:

```text
rigid body lab
material lab
collider lab
force lab
```

Success condition:

> Core Avian rigid-body behavior can be visually compared.

---

## Milestone 6 — Joints

Implement all five joint demonstrations.

Then implement runtime fixed-joint composition.

Success condition:

> Multiple objects can form stable compound constructions.

This is the second major go/no-go milestone.

---

## Milestone 7 — Queries and Sensors

Implement:

```text
sensors
collision events
raycasts
shape casts
point projection
intersection tests
filters
```

Success condition:

> The prototype provides enough query functionality for future interaction and placement mechanics.

---

## Milestone 8 — Advanced Physics

Implement:

```text
CCD
sleeping
substeps
pause
step
interpolation
diagnostics
```

Success condition:

> Simulation behavior and performance can be meaningfully evaluated.

---

## Milestone 9 — Game-Specific Experiments

Implement:

```text
wind
breakage
liquid container state
object composition
```

Success condition:

> Custom gameplay rules coexist cleanly with Avian.

---

## Milestone 10 — Stress Test

Test increasing object counts.

Record observations.

Success condition:

> There is enough information to determine practical limits for the real game.

---

# 75. Prototype Evaluation

After completing the prototype, answer these questions.

## Object Interaction

Does picking up an object feel tactile?

Can objects be dragged without excessive instability?

Can heavy and light objects feel noticeably different?

Can the player easily:

```text
pick
drag
drop
throw
push
stack
```

objects?

---

## Stacking

Can objects form stable stacks?

Do small inaccuracies cause structures to explode?

How many stacked objects remain reliable?

What solver/substep configuration works best?

---

## Object Composition

Are fixed-joint constructions stable?

Can players create objects from other objects?

Can composed structures still behave naturally as physical objects?

Can components be detached cleanly?

---

## Performance

How many dynamic bodies are practical?

Example results table:

```text
100 bodies
500 bodies
1000 bodies
2000 bodies
```

Record:

```text
FPS
physics ms
active bodies
sleeping bodies
contacts
```

---

## Interaction Architecture

Determine whether final mouse interaction should use:

```text
Avian/Bevy picking
```

or:

```text
custom spatial queries
```

or a combination of both.

---

## Physics Style

Determine whether the future game should lean toward:

```text
mostly natural physics
```

or:

```text
heavily game-controlled physics
```

The likely target is somewhere between the two.

---

# 76. Final Deliverable

The completed prototype should launch directly into the physics sandbox.

Within a few minutes, a developer should be able to:

1. walk around;
2. grab and throw objects;
3. stack objects;
4. modify physics parameters;
5. connect objects together;
6. experiment with every major joint;
7. trigger sensors;
8. inspect collisions;
9. visualize raycasts and shape casts;
10. launch high-speed CCD tests;
11. change gravity;
12. pause and step physics;
13. change substeps;
14. enable collider/debug visualization;
15. spawn hundreds of objects;
16. evaluate sleeping and performance;
17. test wind;
18. break objects;
19. build small physical structures.

At the end of the prototype, the developer should have enough hands-on information to answer:

> **Is Avian a suitable physics foundation for the game's tactile sandbox and object-construction mechanics?**

That—not graphical polish—is the definition of success.
