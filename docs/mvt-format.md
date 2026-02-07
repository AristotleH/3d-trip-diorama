# MVT (Mapbox Vector Tile) Format Reference

Quick reference for implementing a minimal MVT parser.

## Overview

MVT is a protobuf-based format for vector map tiles. Each tile contains multiple layers (buildings, roads, etc.), and each layer contains features with geometry and properties.

## Protobuf Structure

```protobuf
message Tile {
  repeated Layer layers = 3;
}

message Layer {
  required string name = 1;
  repeated Feature features = 2;
  repeated string keys = 3;      // Property key strings
  repeated Value values = 4;     // Property values
  optional uint32 extent = 5;    // Tile coordinate space (default 4096)
}

message Feature {
  optional uint64 id = 1;
  repeated uint32 tags = 2;      // Alternating key/value indices
  optional GeomType type = 3;
  repeated uint32 geometry = 4;  // Encoded geometry commands
}

enum GeomType {
  UNKNOWN = 0;
  POINT = 1;
  LINESTRING = 2;
  POLYGON = 3;
}

message Value {
  optional string string_value = 1;
  optional float float_value = 2;
  optional double double_value = 3;
  optional int64 int_value = 4;
  optional uint64 uint_value = 5;
  optional sint64 sint_value = 6;
  optional bool bool_value = 7;
}
```

## Geometry Encoding

Geometry is encoded as a sequence of commands:

```
command_integer = (command_id & 0x7) | (command_count << 3)
```

### Commands

| ID | Name | Parameters | Description |
|----|------|------------|-------------|
| 1 | MoveTo | (dx, dy) | Move cursor |
| 2 | LineTo | (dx, dy) | Draw line to |
| 7 | ClosePath | none | Close polygon ring |

### Parameter Encoding (ZigZag)

Parameters are zigzag-encoded signed integers:
```rust
fn decode_zigzag(n: u32) -> i32 {
    ((n >> 1) as i32) ^ (-((n & 1) as i32))
}
```

### Decoding Algorithm

```rust
fn decode_geometry(commands: &[u32], geom_type: GeomType) -> Vec<Vec<(i32, i32)>> {
    let mut cursor = (0i32, 0i32);
    let mut rings = Vec::new();
    let mut current_ring = Vec::new();
    let mut i = 0;

    while i < commands.len() {
        let cmd = commands[i];
        let cmd_id = cmd & 0x7;
        let cmd_count = cmd >> 3;
        i += 1;

        match cmd_id {
            1 => { // MoveTo
                if !current_ring.is_empty() {
                    rings.push(current_ring);
                    current_ring = Vec::new();
                }
                for _ in 0..cmd_count {
                    let dx = decode_zigzag(commands[i]);
                    let dy = decode_zigzag(commands[i + 1]);
                    cursor.0 += dx;
                    cursor.1 += dy;
                    current_ring.push(cursor);
                    i += 2;
                }
            }
            2 => { // LineTo
                for _ in 0..cmd_count {
                    let dx = decode_zigzag(commands[i]);
                    let dy = decode_zigzag(commands[i + 1]);
                    cursor.0 += dx;
                    cursor.1 += dy;
                    current_ring.push(cursor);
                    i += 2;
                }
            }
            7 => { // ClosePath
                // Ring is implicitly closed
            }
            _ => {}
        }
    }

    if !current_ring.is_empty() {
        rings.push(current_ring);
    }

    rings
}
```

## Coordinate System

- Tile coordinates range from 0 to `extent` (usually 4096)
- Origin is top-left
- Y increases downward
- Convert to world coordinates:

```rust
fn tile_coord_to_world(
    tile_x: i32, tile_y: i32,  // Coordinate within tile
    tile: TileCoord,           // z/x/y
    extent: u32,
) -> (f64, f64) {
    let n = 2.0_f64.powi(tile.z as i32);
    let lng = (tile.x as f64 + tile_x as f64 / extent as f64) / n * 360.0 - 180.0;
    let lat_rad = ((1.0 - 2.0 * (tile.y as f64 + tile_y as f64 / extent as f64) / n) * PI).sinh().atan();
    let lat = lat_rad.to_degrees();
    (lng, lat)
}
```

## Minimal Protobuf Parser

For MVT, you only need to parse a subset of protobuf:

```rust
fn parse_varint(data: &[u8], pos: &mut usize) -> u64 {
    let mut result = 0u64;
    let mut shift = 0;
    loop {
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 { break; }
        shift += 7;
    }
    result
}

fn parse_field(data: &[u8], pos: &mut usize) -> (u32, WireType, &[u8]) {
    let tag = parse_varint(data, pos) as u32;
    let field_num = tag >> 3;
    let wire_type = tag & 0x7;

    let value = match wire_type {
        0 => { /* varint */ }
        2 => { /* length-delimited */
            let len = parse_varint(data, pos) as usize;
            let slice = &data[*pos..*pos + len];
            *pos += len;
            slice
        }
        _ => { /* skip */ }
    };

    (field_num, wire_type, value)
}
```

## Relevant Layers

| Layer Name | Contents |
|------------|----------|
| `building` | Building footprints with height |
| `transportation` | Roads, railways, paths |
| `water` | Water bodies |
| `landuse` | Parks, residential areas |
| `place` | City/neighborhood labels |

## Useful Properties

### Buildings
- `render_height` - Height in meters
- `render_min_height` - Base height (for buildings on hills)
- `class` - "residential", "commercial", etc.

### Transportation
- `class` - "rail", "primary", "secondary", "path"
- `subclass` - More specific type

## Resources

- [MVT Specification](https://github.com/mapbox/vector-tile-spec)
- [MapTiler API](https://docs.maptiler.com/cloud/api/tiles/)
