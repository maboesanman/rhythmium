
compilation_output=$(cargo build --bin rhythmium --message-format json)

declare -a "success=$(
  echo "$compilation_output" \
  | jq -r 'select(.reason=="build-finished") | .success' \
)"

# exit if compilation failed
if [ "$success" != "true" ]; then
  echo "compilation failed"
  exit 1
fi

declare -a "scratch_dir=$(
  echo "$compilation_output" \
  | jq -r 'select((.reason=="build-script-executed") and (.package_id | startswith("scratch "))) | .out_dir' \
)/cef_wrapper"

declare -a "rhythmium_artifact=$(
  echo "$compilation_output" \
  | jq -r 'select((.reason=="compiler-artifact") and (.target.kind[] | contains("bin"))) | .filenames[]' \
)"

# make new folder next to the rhythmium artifact called bundle
bundle_dir="$(dirname "$rhythmium_artifact")/bundle"
rm -rf "$bundle_dir"
mkdir -p "$bundle_dir"

# copy the scratch dir into the bundle dir
cp -r "$scratch_dir/rhythmium.app" "$bundle_dir"

# copy the rhythmium artifact into the bundle dir
mkdir -p "$bundle_dir/rhythmium.app/Contents/MacOS"
cp "$rhythmium_artifact" "$bundle_dir/rhythmium.app/Contents/MacOS/rhythmium"
