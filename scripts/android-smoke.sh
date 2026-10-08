#!/usr/bin/env bash
# Android smoke test: build the APK, install, launch, tail logcat.
#   scripts/android-smoke.sh                  build, install (if a device is attached), launch, tail logs
#   scripts/android-smoke.sh push-data DIR    adb push DIR (an exported installation folder) to the app
set -euo pipefail
PKG=com.skate3.engine
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export ANDROID_HOME="${ANDROID_HOME:-/opt/android-sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.2.12479018}"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"

case "${1:-run}" in
push-data)
  dir="${2:?usage: android-smoke.sh push-data <installation dir>}"
  adb push "$dir/." "/sdcard/Android/data/$PKG/files/installation/"
  ;;
run)
  (cd "$ROOT/android" && ./gradlew assembleDebug)
  apk="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
  ls -l "$apk"
  if ! command -v adb >/dev/null || [ -z "$(adb devices | sed 1d | grep -w device || true)" ]; then
    echo "No device attached; built $apk only."
    exit 0
  fi
  adb install -r "$apk"
  adb logcat -c
  adb shell am start -n "$PKG/.LauncherActivity"
  adb logcat -s skate3 RustStdoutStderr AndroidRuntime DEBUG
  ;;
*) echo "usage: $0 [run|push-data DIR]" >&2; exit 2 ;;
esac
