#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixtures_root="$repo_root/tests/fixtures"
apk_dir="$fixtures_root/apks"
keystore_path="$fixtures_root/debug.keystore"

sdk_root="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-/usr/local/android-sdk}}"

find_latest_android_jar() {
    local preferred="$sdk_root/platforms/android-34/android.jar"
    if [[ -f "$preferred" ]]; then
        printf '%s\n' "$preferred"
        return 0
    fi

    find "$sdk_root/platforms" -maxdepth 2 -name android.jar 2>/dev/null | sort -V | tail -n 1
}

find_latest_build_tools() {
    find "$sdk_root/build-tools" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | sort -V | tail -n 1
}

require_file() {
    local path="$1"
    local description="$2"
    if [[ -z "$path" || ! -e "$path" ]]; then
        echo "missing $description" >&2
        exit 1
    fi
}

require_command() {
    local command_name="$1"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required command: $command_name" >&2
        exit 1
    fi
}

require_command aapt
require_command javac
require_command keytool

android_jar="$(find_latest_android_jar)"
build_tools_dir="$(find_latest_build_tools)"
d8="$build_tools_dir/d8"
zipalign="$build_tools_dir/zipalign"
apksigner="$build_tools_dir/apksigner"

require_file "$android_jar" "android.jar"
require_file "$build_tools_dir" "Android build-tools directory"
require_file "$d8" "d8"
require_file "$zipalign" "zipalign"
require_file "$apksigner" "apksigner"

if [[ ! -f "$keystore_path" ]]; then
    keytool -genkeypair \
        -keystore "$keystore_path" \
        -storepass android \
        -keypass android \
        -alias androiddebugkey \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -dname "CN=RustDroid Fixture Debug,O=RustDroid,C=US" \
        >/dev/null 2>&1
fi

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

rm -rf "$apk_dir"
mkdir -p "$apk_dir"

sign_apk() {
    local unsigned_apk="$1"
    local output_apk="$2"
    local aligned_apk="${unsigned_apk%.apk}-aligned.apk"

    "$zipalign" -f 4 "$unsigned_apk" "$aligned_apk"
    "$apksigner" sign \
        --ks "$keystore_path" \
        --ks-pass pass:android \
        --key-pass pass:android \
        --ks-key-alias androiddebugkey \
        --out "$output_apk" \
        "$aligned_apk" \
        >/dev/null 2>&1
    rm -f "$output_apk.idsig"
}

write_activity_source() {
    local src_dir="$1"
    local package_name="$2"
    local activity_name="$3"
    local screen_text="$4"
    local package_path="${package_name//./\/}"

    mkdir -p "$src_dir/$package_path"
    cat >"$src_dir/$package_path/$activity_name.java" <<EOF_ACTIVITY
package $package_name;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

public final class $activity_name extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        TextView view = new TextView(this);
        view.setText("$screen_text");
        setContentView(view);
    }
}
EOF_ACTIVITY
}

compile_activity() {
    local src_dir="$1"
    local out_dir="$2"

    mkdir -p "$out_dir/classes" "$out_dir/dex"
    javac -source 8 -target 8 -Xlint:-options \
        -cp "$android_jar" \
        -d "$out_dir/classes" \
        $(find "$src_dir" -name '*.java' | sort)
    "$d8" \
        --min-api 24 \
        --lib "$android_jar" \
        --output "$out_dir/dex" \
        $(find "$out_dir/classes" -name '*.class' | sort)
    mv "$out_dir/dex/classes.dex" "$out_dir/classes.dex"
}

write_manifest() {
    local manifest_path="$1"
    local package_name="$2"
    local activity_name="$3"
    local include_launcher="$4"

    if [[ "$include_launcher" == "true" ]]; then
        cat >"$manifest_path" <<EOF_MANIFEST
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="$package_name"
    android:versionCode="1"
    android:versionName="1.0">
    <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="30" />
    <application android:label="@string/app_name">
        <activity android:name=".$activity_name" android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
EOF_MANIFEST
    else
        cat >"$manifest_path" <<EOF_MANIFEST
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="$package_name"
    android:versionCode="1"
    android:versionName="1.0">
    <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="30" />
    <application android:label="@string/app_name">
        <activity android:name=".$activity_name" />
    </application>
</manifest>
EOF_MANIFEST
    fi
}

write_strings() {
    local res_dir="$1"
    local app_label="$2"

    mkdir -p "$res_dir/values"
    cat >"$res_dir/values/strings.xml" <<EOF_STRINGS
<resources>
    <string name="app_name">$app_label</string>
</resources>
EOF_STRINGS
}

build_single_apk_fixture() {
    local fixture_name="$1"
    local package_name="$2"
    local app_label="$3"
    local activity_name="$4"
    local screen_text="$5"
    local include_launcher="$6"
    local native_abi="${7:-}"

    local fixture_dir="$workdir/$fixture_name"
    mkdir -p "$fixture_dir/src" "$fixture_dir/res" "$fixture_dir/build"

    write_manifest "$fixture_dir/AndroidManifest.xml" "$package_name" "$activity_name" "$include_launcher"
    write_strings "$fixture_dir/res" "$app_label"
    write_activity_source "$fixture_dir/src" "$package_name" "$activity_name" "$screen_text"
    compile_activity "$fixture_dir/src" "$fixture_dir/build"

    local unsigned_apk="$fixture_dir/build/$fixture_name-unsigned.apk"
    aapt package \
        -f \
        -M "$fixture_dir/AndroidManifest.xml" \
        -S "$fixture_dir/res" \
        -I "$android_jar" \
        -F "$unsigned_apk"

    (
        cd "$fixture_dir/build"
        aapt add "$unsigned_apk" classes.dex >/dev/null
        if [[ -n "$native_abi" ]]; then
            mkdir -p "lib/$native_abi"
            printf 'fixture-%s\n' "$native_abi" >"lib/$native_abi/libfixture.so"
            aapt add "$unsigned_apk" "lib/$native_abi/libfixture.so" >/dev/null
        fi
    )

    sign_apk "$unsigned_apk" "$apk_dir/$fixture_name.apk"
}

build_split_fixture() {
    local fixture_dir="$workdir/split"
    local package_name="com.rustdroid.fixture.split"
    local activity_name="MainActivity"
    mkdir -p "$fixture_dir/src" "$fixture_dir/res/values" "$fixture_dir/res/values-en" "$fixture_dir/build"

    write_manifest "$fixture_dir/AndroidManifest.xml" "$package_name" "$activity_name" "true"
    cat >"$fixture_dir/res/values/strings.xml" <<'EOF_STRINGS'
<resources>
    <string name="app_name">RustDroid Split Fixture</string>
    <string name="greeting">Hello</string>
</resources>
EOF_STRINGS
    cat >"$fixture_dir/res/values-en/strings.xml" <<'EOF_STRINGS'
<resources>
    <string name="app_name">RustDroid Split Fixture EN</string>
    <string name="greeting">Hello EN</string>
</resources>
EOF_STRINGS
    write_activity_source "$fixture_dir/src" "$package_name" "$activity_name" "RustDroid split fixture"
    compile_activity "$fixture_dir/src" "$fixture_dir/build"

    local unsigned_base="$fixture_dir/build/split-base-unsigned.apk"
    local unsigned_split="$fixture_dir/build/split-base-unsigned_en.apk"
    aapt package \
        -f \
        -M "$fixture_dir/AndroidManifest.xml" \
        -S "$fixture_dir/res" \
        -I "$android_jar" \
        --split en \
        -F "$unsigned_base"

    (
        cd "$fixture_dir/build"
        aapt add "$unsigned_base" classes.dex >/dev/null
    )

    sign_apk "$unsigned_base" "$apk_dir/split-base.apk"
    sign_apk "$unsigned_split" "$apk_dir/split-config.en.apk"
}

build_single_apk_fixture \
    "launch-success" \
    "com.rustdroid.fixture.launch" \
    "RustDroid Launch Fixture" \
    "MainActivity" \
    "RustDroid launch fixture" \
    "true"

build_single_apk_fixture \
    "missing-launcher" \
    "com.rustdroid.fixture.missinglauncher" \
    "RustDroid Missing Launcher Fixture" \
    "HiddenActivity" \
    "RustDroid missing launcher fixture" \
    "false"

build_single_apk_fixture \
    "x86_64-native" \
    "com.rustdroid.fixture.x86native" \
    "RustDroid x86_64 Fixture" \
    "MainActivity" \
    "RustDroid x86_64 native fixture" \
    "true" \
    "x86_64"

build_single_apk_fixture \
    "arm64-native" \
    "com.rustdroid.fixture.armnative" \
    "RustDroid ARM64 Fixture" \
    "MainActivity" \
    "RustDroid arm64 native fixture" \
    "true" \
    "arm64-v8a"

build_split_fixture

echo "generated fixture APKs:"
find "$apk_dir" -maxdepth 1 -type f -name '*.apk' | sort
