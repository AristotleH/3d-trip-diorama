# LLM Prompt and Schema

System prompt and JSON schema for converting natural language trip descriptions.

## System Prompt

```
You are a travel planning assistant. Convert natural language trip descriptions into structured JSON data.

For each trip, generate 1-3 possible itinerary variations. Each variation should include:
- Specific locations with coordinates (latitude, longitude)
- Transport modes between locations
- Approximate timing if mentioned

Output valid JSON matching this schema exactly. Do not include any text outside the JSON.

## JSON Schema

{
  "plans": [
    {
      "name": "string - short descriptive name for this plan variation",
      "segments": [
        {
          "from": {
            "name": "string - location name",
            "lat": number,
            "lng": number
          },
          "to": {
            "name": "string - location name",
            "lat": number,
            "lng": number
          },
          "transport": "train" | "bus" | "walk" | "bike" | "car" | "plane" | "ferry",
          "departure": "ISO 8601 datetime or null",
          "duration_minutes": number or null,
          "notes": "string - any relevant details"
        }
      ]
    }
  ]
}

## Guidelines

1. Use real, accurate coordinates for named locations
2. Choose realistic transport modes (e.g., train for long distances in Japan)
3. If timing is vague, omit departure times
4. Include walking segments between transit and destinations
5. Generate varied plans when possible:
   - Different routes
   - Different transport modes
   - Different time allocations
```

## Example Input/Output

### Input
```
I want to spend 2 days in Tokyo. I'd like to visit Shibuya for shopping,
see the Senso-ji temple in Asakusa, and explore Akihabara for electronics.
Starting from Tokyo Station.
```

### Output
```json
{
  "plans": [
    {
      "name": "Classic Tokyo Loop",
      "segments": [
        {
          "from": { "name": "Tokyo Station", "lat": 35.6812, "lng": 139.7671 },
          "to": { "name": "Shibuya Station", "lat": 35.6580, "lng": 139.7016 },
          "transport": "train",
          "departure": null,
          "duration_minutes": 25,
          "notes": "JR Yamanote Line"
        },
        {
          "from": { "name": "Shibuya Station", "lat": 35.6580, "lng": 139.7016 },
          "to": { "name": "Shibuya Crossing", "lat": 35.6595, "lng": 139.7004 },
          "transport": "walk",
          "departure": null,
          "duration_minutes": 5,
          "notes": "Famous scramble crossing"
        },
        {
          "from": { "name": "Shibuya", "lat": 35.6580, "lng": 139.7016 },
          "to": { "name": "Asakusa Station", "lat": 35.7112, "lng": 139.7963 },
          "transport": "train",
          "departure": null,
          "duration_minutes": 35,
          "notes": "Tokyo Metro Ginza Line"
        },
        {
          "from": { "name": "Asakusa Station", "lat": 35.7112, "lng": 139.7963 },
          "to": { "name": "Senso-ji Temple", "lat": 35.7148, "lng": 139.7967 },
          "transport": "walk",
          "departure": null,
          "duration_minutes": 10,
          "notes": "Through Nakamise shopping street"
        },
        {
          "from": { "name": "Asakusa", "lat": 35.7112, "lng": 139.7963 },
          "to": { "name": "Akihabara Station", "lat": 35.6984, "lng": 139.7731 },
          "transport": "train",
          "departure": null,
          "duration_minutes": 15,
          "notes": "Tsukuba Express"
        },
        {
          "from": { "name": "Akihabara Station", "lat": 35.6984, "lng": 139.7731 },
          "to": { "name": "Tokyo Station", "lat": 35.6812, "lng": 139.7671 },
          "transport": "train",
          "departure": null,
          "duration_minutes": 5,
          "notes": "JR Yamanote Line"
        }
      ]
    },
    {
      "name": "East Side Focus",
      "segments": [
        {
          "from": { "name": "Tokyo Station", "lat": 35.6812, "lng": 139.7671 },
          "to": { "name": "Asakusa Station", "lat": 35.7112, "lng": 139.7963 },
          "transport": "train",
          "departure": null,
          "duration_minutes": 20,
          "notes": "Direct via Tokyo Metro"
        }
        // ... more segments
      ]
    }
  ]
}
```

## Rust Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LlmResponse {
    pub plans: Vec<TripPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripPlan {
    pub name: String,
    pub segments: Vec<TripSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripSegment {
    pub from: Location,
    pub to: Location,
    pub transport: TransportMode,
    pub departure: Option<String>,
    pub duration_minutes: Option<u32>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    Train,
    Bus,
    Walk,
    Bike,
    Car,
    Plane,
    Ferry,
}
```

## Validation

After parsing LLM response:

```rust
fn validate_plan(plan: &TripPlan) -> Result<(), ValidationError> {
    // Check coordinates are reasonable
    for seg in &plan.segments {
        validate_coord(seg.from.lat, seg.from.lng)?;
        validate_coord(seg.to.lat, seg.to.lng)?;
    }

    // Check segments connect
    for window in plan.segments.windows(2) {
        let end = &window[0].to;
        let start = &window[1].from;
        let dist = haversine_km(end.lat, end.lng, start.lat, start.lng);
        if dist > 1.0 {
            // Segments don't connect - might need walking segment
        }
    }

    Ok(())
}

fn validate_coord(lat: f64, lng: f64) -> Result<(), ValidationError> {
    if lat < -90.0 || lat > 90.0 || lng < -180.0 || lng > 180.0 {
        return Err(ValidationError::InvalidCoordinates);
    }
    Ok(())
}
```
