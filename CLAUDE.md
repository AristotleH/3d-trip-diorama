# 3D Trip Diorama Renderer

A Rust/WASM application that converts natural language trip descriptions into interactive 3D visualizations using OpenStreetMap data and WebGPU.

## Quick Reference

```bash
# Build for web
wasm-pack build crates/app --target web

# Run dev server
cd web && python -m http.server 8080
```

## Architecture Overview

```
User Input (natural language)
       │
       ▼
   LLM Provider ──► TripPlan { segments: [...] }
       │
       ▼
   TileManager.fetch_tiles(bbox)
       │
       ▼
   MVT Parser → Building/Road/Rail data
       │
       ▼
   MeshBuilder: extrude to 3D
       │
       ▼
   Renderer: WebGPU draw loop
```

## Project Structure

```
3d-trip-diorama/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── core/               # Shared types, math (Vec3, Mat4), utilities
│   ├── llm/                # LLM provider abstraction (Claude, OpenAI)
│   ├── tiles/              # Vector tile fetching & MVT parsing
│   ├── renderer/           # WebGPU 3D renderer
│   └── app/                # WASM entry point, UI, orchestration
├── web/                    # HTML, JS glue, assets
├── docs/                   # Detailed design docs
└── CLAUDE.md               # This file
```

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Platform | WASM + WebGPU | Zero-install web app, share via URL |
| Tile API | MapTiler (free tier) | Good free tier, MVT format, building heights |
| Protobuf | Hand-rolled parser | MVT is simple, avoids dependencies |
| Math | Custom Vec3/Mat4 | Educational, minimal surface area |
| Triangulation | Ear clipping | Simple, sufficient for building footprints |
| UI | HTML + JS interop | Keep Rust focused on graphics |
| LLM | Multi-provider | Flexible (Claude, OpenAI, etc.) |

## Visual Style: Miniature Tilt-Shift Diorama

- Camera locked to 45-60° elevation ("looking at a model")
- Soft vignette at edges (post-process shader)
- Fake depth-of-field via distance-based blur
- Warm, saturated building colors
- Hard shadow from single directional light
- Bounded edges with fade/cutout border
- Optional subtle fog for depth

## Dependencies (Intentionally Minimal)

```toml
wgpu = "0.19"           # WebGPU bindings
winit = "0.29"          # Windowing (web-compatible)
wasm-bindgen = "0.2"    # Rust-JS interop
web-sys = "0.3"         # Web APIs
serde = "1.0"           # JSON serialization
serde_json = "1.0"      # JSON parsing
```

**Not using** (building from scratch for learning):
- Game engines (bevy, macroquad)
- Math libraries (glam, nalgebra)
- Protobuf libraries (prost)
- HTTP clients (reqwest) - using web-sys fetch

## Implementation Phases

See `docs/implementation-plan.md` for detailed phases.

1. **Renderer Foundation** - WebGPU init, render loop, camera, test cube
2. **Geometry Pipeline** - Math, meshes, triangulation, building extrusion
3. **Tile Integration** - MVT parser, MapTiler API, render buildings
4. **LLM Integration** - Provider trait, Claude/OpenAI, trip parsing
5. **Trip Visualization** - Route paths, markers, animation
6. **Polish** - Tilt-shift effects, UI, multiple plans

## API Keys Required

- **MapTiler**: Free tier at https://cloud.maptiler.com/ (100k req/month)
- **LLM Provider**: Claude API key or OpenAI API key

Store in `web/.env.local` (gitignored):
```
MAPTILER_API_KEY=your_key
CLAUDE_API_KEY=your_key
OPENAI_API_KEY=your_key
```

## Core Types

```rust
// Trip representation (from LLM)
pub struct TripPlan {
    pub name: String,
    pub segments: Vec<TripSegment>,
}

pub struct TripSegment {
    pub from: Coordinate,
    pub to: Coordinate,
    pub transport: TransportMode,  // Train, Walk, Bus, etc.
    pub datetime: Option<DateTime>,
    pub description: String,
}

// Tile data (from OSM)
pub struct Tile {
    pub buildings: Vec<BuildingPolygon>,
    pub roads: Vec<RoadLine>,
    pub water: Vec<WaterPolygon>,
    pub rail: Vec<RailLine>,
}

// LLM abstraction
pub trait TripPlanner: Send + Sync {
    async fn plan_trip(&self, input: &str) -> Result<Vec<TripPlan>, PlanError>;
}
```

## Coordinate Systems

1. **WGS84** (lat/lng) - User-facing, LLM output
2. **Web Mercator** (EPSG:3857) - Tile coordinates
3. **Local 3D** - Scene coordinates, origin at bbox center

## File Naming Conventions

- Modules: `snake_case.rs`
- Types: `PascalCase`
- Functions: `snake_case`
- Shaders: `purpose.wgsl` (e.g., `building.wgsl`)

## Testing Strategy

- Unit tests for math, triangulation, coordinate conversion
- Integration tests with cached tile data
- Visual regression via screenshot comparison (later)
