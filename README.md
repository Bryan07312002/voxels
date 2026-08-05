```sh
voxels/
├── Cargo.toml                   # Workspace manifest
├── mods/                        # Runtime assets & WASM plugins
│   └── base_game/
│       └── blocks.json
└── crates/
    ├── core_types/              # Zero-dependency primitives: BlockId, ChunkPos, Math wrappers
    ├── voxel_mesh/              # Pure CPU Greedy Meshing & AABB extraction (no GPU code)
    ├── engine_core/             # Chunks, WorldStore, ECS components/resources, Registry
    ├── physics/                 # Voxel Raycasting, Swept AABB collision
    ├── world_gen/               # Noise pipelines, Biomes, Terrain generators
    ├── net/                     # Transport layer, packet serialization, sync logic
    ├── mod_api/                 # Extism/Wasmtime runtime, JSON schema definitions
    ├── render_pipeline/        # WGPU / Bevy rendering, Shaders, Texture Arrays
    ├── server/                  # [BINARY] Headless dedicated server
    └── client/                  # [BINARY] Desktop game executable
```

```sh
                  ┌─────────────────┐
                  │   core_types    │  (Primitives: BlockId, ChunkPos, Math)
                  └────────┬────────┘
                           │
             ┌─────────────┼────────────────────────┐
             ▼             ▼                        ▼
     ┌──────────────┐  ┌──────────────┐    ┌─────────────────┐
     │  voxel_mesh  │  │   mod_api    │    │   engine_core   │ (Chunk, WorldStore, Registry)
     └───────┬──────┘  └──────┬───────┘    └────────┬────────┘
             │                │                     │
             │                ├─────────────────────┴────────────┐
             │                ▼                                  ▼
             │       ┌─────────────────┐                ┌─────────────────┐
             │       │    world_gen    │                │     physics     │
             │       └────────┬────────┘                └────────┬────────┘
             │                │                                  │
             │                └─────────────────┬────────────────┘
             │                                  │
             │                                  ▼
             │                         ┌─────────────────┐
             │                         │       net       │ (Protocol & Transport)
             │                         └────────┬────────┘
             │                                  │
             ▼                                  │
    ┌─────────────────┐                         │
    │ render_pipeline │                         │
    └────────┬────────┘                         │
             │                                  │
             ├──────────────────────────────────┘
             │                         │
             ▼                         ▼
    ┌─────────────────┐       ┌─────────────────┐
    │     client      │       │     server      │
    │    [BINARY]     │       │    [BINARY]     │
    └─────────────────┘       └─────────────────┘
```
