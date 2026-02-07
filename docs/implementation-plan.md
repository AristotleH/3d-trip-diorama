# Implementation Plan

Detailed breakdown of each phase with specific tasks and verification steps.

---

## Phase 1: Renderer Foundation

**Goal**: Get a spinning cube rendering in the browser via WebGPU.

### Tasks

1. **Workspace setup**
   - Create `Cargo.toml` workspace with `crates/` structure
   - Add `crates/core`, `crates/renderer`, `crates/app`
   - Configure wasm-pack build

2. **WebGPU initialization** (`crates/renderer`)
   - Request adapter and device
   - Create surface from canvas element
   - Configure swap chain format (typically Bgra8Unorm)
   - Set up render loop via `requestAnimationFrame`

3. **Basic pipeline**
   - Create shader module (hardcoded cube WGSL)
   - Define vertex buffer layout
   - Create render pipeline with depth testing
   - Bind group for uniforms (MVP matrix)

4. **Camera** (`crates/core`)
   - Implement basic `Camera` struct
   - Perspective projection matrix
   - View matrix from position/target
   - Orbit controls (mouse drag to rotate)

5. **Test cube**
   - Hardcoded cube vertices with colors
   - Render with basic lighting
   - Verify rotation works

### Verification
- Open `localhost:8080` in Chrome/Firefox
- See a colored cube rotating
- Mouse drag rotates view

### Files
```
crates/core/src/lib.rs
crates/core/src/math.rs        # Vec3, Mat4 stubs
crates/core/src/camera.rs
crates/renderer/src/lib.rs
crates/renderer/src/pipeline.rs
crates/renderer/src/shaders/cube.wgsl
crates/app/src/lib.rs          # WASM entry
web/index.html
web/index.js                   # JS glue
```

---

## Phase 2: Geometry Pipeline

**Goal**: Build infrastructure to create arbitrary meshes from polygon data.

### Tasks

1. **Math library** (`crates/core/src/math.rs`)
   ```rust
   pub struct Vec2 { pub x: f32, pub y: f32 }
   pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
   pub struct Vec4 { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }
   pub struct Mat4 { pub data: [f32; 16] }
   ```
   - Implement: add, sub, mul, dot, cross, normalize
   - Mat4: identity, translation, rotation, scale, perspective, look_at
   - Mat4 * Vec4 multiplication

2. **Mesh abstraction** (`crates/renderer/src/mesh.rs`)
   ```rust
   pub struct Mesh {
       vertices: wgpu::Buffer,
       indices: wgpu::Buffer,
       index_count: u32,
   }

   pub struct Vertex {
       position: [f32; 3],
       normal: [f32; 3],
       color: [f32; 3],
   }
   ```

3. **Polygon triangulation** (`crates/core/src/triangulate.rs`)
   - Ear clipping algorithm
   - Input: `Vec<Vec2>` (2D polygon, CCW winding)
   - Output: `Vec<[usize; 3]>` (triangle indices)
   - Handle simple polygons (no holes initially)

4. **Building extrusion** (`crates/renderer/src/building.rs`)
   ```rust
   pub fn extrude_polygon(
       footprint: &[Vec2],
       height: f32,
   ) -> (Vec<Vertex>, Vec<u32>)
   ```
   - Bottom face (optional, usually not visible)
   - Top face (triangulated footprint at y=height)
   - Side walls (quads between consecutive vertices)

5. **Test with hardcoded polygon**
   - Define an L-shaped building footprint
   - Extrude to 3D
   - Render alongside cube

### Verification
- L-shaped building renders correctly
- Proper normals (lighting looks right)
- No gaps or inverted faces

### Files
```
crates/core/src/math.rs        # Full implementation
crates/core/src/triangulate.rs
crates/renderer/src/mesh.rs
crates/renderer/src/building.rs
crates/renderer/src/shaders/building.wgsl
```

---

## Phase 3: Tile Integration

**Goal**: Fetch real OSM data and render actual buildings.

### Tasks

1. **Coordinate conversion** (`crates/core/src/geo.rs`)
   ```rust
   pub struct Coordinate { pub lat: f64, pub lng: f64 }
   pub struct TileCoord { pub z: u8, pub x: u32, pub y: u32 }

   pub fn lat_lng_to_tile(coord: Coordinate, zoom: u8) -> TileCoord
   pub fn tile_to_lat_lng(tile: TileCoord) -> Coordinate
   pub fn lat_lng_to_meters(coord: Coordinate) -> (f64, f64)  // Web Mercator
   ```

2. **MVT protobuf parser** (`crates/tiles/src/mvt.rs`)
   - Parse .mvt binary format (subset of protobuf)
   - Extract layers: buildings, roads, water, landuse
   - Decode geometry commands (MoveTo, LineTo, ClosePath)
   - Convert to polygon/line vectors

3. **Tile fetching** (`crates/tiles/src/fetch.rs`)
   - HTTP fetch via web-sys
   - MapTiler URL format: `https://api.maptiler.com/tiles/v3/{z}/{x}/{y}.pbf?key=KEY`
   - Async tile loading with caching
   - Handle errors gracefully

4. **Tile manager** (`crates/tiles/src/manager.rs`)
   ```rust
   pub struct TileManager {
       cache: HashMap<TileCoord, Tile>,
       pending: HashSet<TileCoord>,
       api_key: String,
   }

   impl TileManager {
       pub async fn get_tiles(&mut self, bbox: BoundingBox, zoom: u8) -> Vec<&Tile>
   }
   ```

5. **Mesh generation from tiles**
   - Convert building polygons to meshes
   - Use height from `render_height` or `height` property (default: 10m)
   - Position buildings in local coordinate system

6. **Scene assembly**
   - Calculate bounding box for a test location
   - Fetch 3x3 tile grid
   - Render all buildings

### Verification
- Pick a known location (e.g., Tokyo Station)
- Buildings render in approximately correct locations
- Tall buildings are taller

### Files
```
crates/core/src/geo.rs
crates/tiles/Cargo.toml
crates/tiles/src/lib.rs
crates/tiles/src/mvt.rs
crates/tiles/src/fetch.rs
crates/tiles/src/manager.rs
```

---

## Phase 4: LLM Integration

**Goal**: Convert natural language trip descriptions to structured data.

### Tasks

1. **Trip data types** (`crates/core/src/trip.rs`)
   ```rust
   pub struct TripPlan {
       pub name: String,
       pub segments: Vec<TripSegment>,
   }

   pub struct TripSegment {
       pub from: Location,
       pub to: Location,
       pub transport: TransportMode,
       pub departure: Option<DateTime>,
       pub notes: String,
   }

   pub struct Location {
       pub name: String,
       pub coord: Coordinate,
   }

   pub enum TransportMode {
       Train, Bus, Walk, Bike, Car, Plane, Ferry
   }
   ```

2. **LLM provider trait** (`crates/llm/src/lib.rs`)
   ```rust
   #[async_trait]
   pub trait TripPlanner {
       async fn plan_trip(&self, input: &str) -> Result<Vec<TripPlan>, PlanError>;
   }
   ```

3. **JSON schema for LLM** (`crates/llm/src/schema.rs`)
   - Define expected JSON output format
   - Include in system prompt
   - Parse and validate response

4. **Claude provider** (`crates/llm/src/claude.rs`)
   - HTTP POST to `https://api.anthropic.com/v1/messages`
   - System prompt with JSON schema
   - Parse response, extract JSON from content

5. **OpenAI provider** (`crates/llm/src/openai.rs`)
   - HTTP POST to `https://api.openai.com/v1/chat/completions`
   - Use JSON mode or function calling
   - Parse response

6. **Error handling**
   - Retry on rate limits
   - Validate coordinates are reasonable
   - Handle malformed JSON gracefully

### Verification
- Input: "3 day trip to Tokyo, visiting Shibuya, Akihabara, and Asakusa"
- Output: Valid `TripPlan` with 3+ segments
- Coordinates are in Tokyo area

### Files
```
crates/core/src/trip.rs
crates/llm/Cargo.toml
crates/llm/src/lib.rs
crates/llm/src/schema.rs
crates/llm/src/claude.rs
crates/llm/src/openai.rs
```

---

## Phase 5: Trip Visualization

**Goal**: Render trip routes as 3D paths over the city.

### Tasks

1. **Route geometry** (`crates/renderer/src/path.rs`)
   ```rust
   pub fn generate_path_mesh(
       points: &[Vec3],
       width: f32,
       color: [f32; 3],
   ) -> (Vec<Vertex>, Vec<u32>)
   ```
   - Create ribbon/tube along path
   - Consistent width in screen space (or world space)
   - Smooth corners with bevels or curves

2. **Transport mode styling**
   - Train: solid blue line
   - Walk: dashed yellow line
   - Bus: solid green line
   - Different line widths

3. **Markers** (`crates/renderer/src/marker.rs`)
   - Spheres or pins at stop locations
   - Labels (text rendering - could use HTML overlay initially)

4. **Animation system**
   - Progress parameter (0.0 to 1.0)
   - Animate path drawing
   - Moving marker along path

5. **Camera framing**
   - Automatically frame camera to show entire trip
   - Smooth transitions between segments

### Verification
- Trip path renders over buildings
- Different transport modes visually distinct
- Animation plays smoothly

### Files
```
crates/renderer/src/path.rs
crates/renderer/src/marker.rs
crates/renderer/src/animation.rs
crates/renderer/src/shaders/path.wgsl
```

---

## Phase 6: Polish

**Goal**: Achieve the miniature tilt-shift diorama aesthetic.

### Tasks

1. **Post-processing pipeline**
   - Render scene to texture
   - Apply post-process effects
   - Output to screen

2. **Tilt-shift blur** (`crates/renderer/src/shaders/tiltshift.wgsl`)
   - Blur based on distance from focal plane
   - Adjustable focal distance and blur amount

3. **Vignette**
   - Darken edges of screen
   - Soft circular gradient

4. **Color grading**
   - Warm color temperature
   - Increased saturation
   - Optional: miniature look with contrast boost

5. **Diorama borders**
   - Fade to background color at edges
   - Or hard cutoff with shadow

6. **UI implementation**
   - Text input for trip description
   - Loading indicator during LLM call
   - Multiple plan tabs/selection
   - Basic controls (zoom, rotate)

7. **Multiple trip plans**
   - Display multiple LLM suggestions
   - Toggle between them
   - Compare side-by-side (stretch goal)

### Verification
- Scene looks like a miniature model
- Tilt-shift effect is convincing
- UI is functional and responsive

### Files
```
crates/renderer/src/postprocess.rs
crates/renderer/src/shaders/tiltshift.wgsl
crates/renderer/src/shaders/vignette.wgsl
crates/app/src/ui.rs
web/styles.css
```
