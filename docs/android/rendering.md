# Android rendering and performance (Phase 4)

Target: Galaxy S26 Ultra (recent Adreno, Vulkan 1.3). Rendering is Vulkan only; the same code path runs on desktop and Android. Android differences are defaults and diagnostics, not a second renderer.

## Startup capability log

`render_caps.rs` runs in `PreStartup` and logs one `GPU_CAPS` line (logcat tag `skate3`):

```
GPU_CAPS adapter="Adreno (TM) 8xx" backend=Vulkan type=IntegratedGpu driver="..." max_texture_array_layers=2048 max_storage_buffers_per_shader_stage=... max_sampled_textures_per_shader_stage=... max_texture_2d=... cube_array=true compute=true
```

What it guards:

| Check | Why | On failure |
|---|---|---|
| `max_texture_array_layers` | World textures are packed into `texture_2d_array` pages (`retail_render.rs`, `MAX_PAGE_LAYERS = 2048`). | The effective limit is `min(2048, device limit)`. A lower limit seals material slabs sooner, so the map uses more slabs instead of failing. If one material alone needs more layers of a size class than the limit, an `error!` is logged. |
| `CUBE_ARRAY_TEXTURES` | `retail_material_bindings.wgsl` binds `texture_cube_array`. | Warning only. Adreno 7xx and 8xx support it. |
| Compute | `retail_exposure.wgsl` and GPU mesh preprocessing. | Warning only. |
| Sampled textures per stage < 16, storage buffers per stage < 8 | The world material binds 8 pages, cubes and 3 storage buffers on top of Bevy's own. | Warning only. |

## Settings

Stored in `settings/graphics.json` next to the installation. All new fields have serde defaults, so older files keep working.

| Setting | Desktop default | Android first-run default |
|---|---|---|
| Internal resolution (`scale`) | 100% | 70% (Android adds a 70% step to the 25-100% list) |
| FPS limit (`fps`) | Unlimited | 60 (30 is in the list) |
| Present mode | AutoNoVsync | AutoVsync |
| Shadow quality (`shadow_quality`) | High | Medium |
| `gpu_culling` (file only) | true | true |

"First run" means no `graphics.json` yet. Once the file exists, its values win.

### Shadow quality (`shadow_quality.rs`)

| Tier | Scene sun cascades / distance | Receiver light (layer 28) | Shadow map |
|---|---|---|---|
| High | 4 / 100 m (current behaviour) | 1 / 24 m | 2048 |
| Medium | 2 / 60 m | 1 / 24 m | 2048 |
| Low | 1 / 30 m | 1 / 16 m | 1024 |
| Off | shadows disabled on all lights | disabled | unchanged |

Off is safe for both shaders: they only sample lights whose shadow flag is set, so characters and the world fall back to unshadowed lighting. The fallback test-world light in `world.rs` follows the same tiers.

## GPU preprocessing switch

The game never adds `OcclusionCulling` to its cameras; the vendored depth pyramid (`vendor/bevy_core_pipeline`) and the late-pass fix in `vendor/bevy_pbr` only matter if something opts in. What is on by default is Bevy's GPU mesh preprocessing (compute culling plus indirect draws).

For on-device debugging on Android, set `"gpu_culling": false` in `settings/graphics.json`. At startup `app.rs` then builds `PbrPlugin { use_gpu_instance_buffer_builder: false }`, which moves instance building and culling to the CPU. It is read before the app is created, so restart after editing. Desktop ignores the field. Leave it true unless GPU preprocessing misbehaves on a driver (symptoms: missing or flickering geometry, validation errors naming `mesh_preprocess`).

## Reading the perf log

On Android `android_perf.rs` logs every 10 s at info level:

```
PERF fps=59.8 frame_p95=17.9 ms rss=1312 MiB
```

- `fps` and `frame_p95` come from the `frame_timing` window (last 5 s), p95 as the nearest-rank 95th percentile frame time.
- `rss` is the resident set from `/proc/self/statm` (assumes 4 KiB pages). Compare with `adb shell dumpsys meminfo <package>` for the GPU and graphics breakdown.
- `adb logcat -s skate3 | grep -E "PERF|GPU_CAPS"`

The on-screen frame-time counter (Graphics menu, "Frame-time counter") is regular Bevy UI on the existing UI camera, so it works on Android unchanged. Press START on the pad to open the menu and toggle it. Test with Samsung Game Booster on and off; it can cap the frame rate.

## Future work: ASTC textures

Textures are RGBA8 (no BCn problem), so memory is the only cost. Phase 1.4 of `PLAN.md` describes converting map textures to ASTC 6x6 on the PC during export (new codec in `texture_decode.rs`, map pipeline version bump). Not started; do it only if `rss`/`dumpsys meminfo` on DownTown shows memory pressure.
