# Game data export and import

The Android app never bundles game data. You prepare it on the PC with the normal setup, export it to a zip, and import that zip on the phone.

## Export on the PC

Run setup first so `installation.json` exists in your data directory. Then either:

- Click **Export for Android** in the setup window and choose where to save the zip, or
- Run `python tools/export_android.py --base <data dir> --output skate3-android.zip [--exclude-speech]`, or
- Run `python tools/setup.py --base <data dir> --export-android skate3-android.zip` (add `--exclude-speech` to drop decoded speech).

The zip uses the store method (no recompression). Its root holds one folder, `installation/`, with the contents of the active installation (`maps/`, `maps.json`, `settings/`, `assets/`). Logs, caches and temp files are skipped. Everything the installation has is included by default, including decoded speech audio. With `--exclude-speech` (command line only) the decoded speech audio is dropped, the speech index JSON stays, and `audio_manifest.json` is rewritten in the zip with each speech entry's `audio` set to `null`, which the engine treats as no decoded speech. The manifest hashes cover the rewritten bytes. The setup button always exports everything.

The last entry is `installation/android-manifest.json`:

```json
{"format": 1, "created": "...", "installation_id": "...", "pipelines": {"...": "..."},
 "files": {"maps/a.skate": {"size": 123, "sha256": "..."}}}
```

`pipelines` copies the fingerprints from `installation.json`. The app compares them with the engine's own and asks you to re-export when they differ.

## Import on the phone

The launcher streams the picked zip into a staging folder, then checks every file listed in the manifest with `skate_data::android_import::verify_progress` (size and SHA-256, with a progress callback). Missing or mismatched files reject the import. After the asset check passes, the staging folder is renamed to `installation/`.

## Developer path with adb

Extract the zip on the PC, then push the inner folder:

```
adb push installation /sdcard/Android/data/com.skate3.engine/files/installation/
```

Make sure `android-manifest.json` ends up directly inside that `installation/` folder. Push the contents if adb nests the folder (`installation/installation`).
