#!/usr/bin/env bash
set -euo pipefail

require_env() {
  local key="$1"
  if [ -z "${!key:-}" ]; then
    echo "Missing required env: ${key}" >&2
    exit 1
  fi
}

escape_sed() {
  printf '%s' "$1" | sed -e 's/[\/&]/\\&/g'
}

render_template() {
  local src="$1"
  local dst="$2"

  local package_name_esc
  package_name_esc="$(escape_sed "$PACKAGE_NAME")"
  local version_esc
  version_esc="$(escape_sed "$PACKAGE_VERSION")"
  local arch_esc
  arch_esc="$(escape_sed "$ARCH")"
  local summary_esc
  summary_esc="$(escape_sed "$SUMMARY")"
  local desc1_esc
  desc1_esc="$(escape_sed "$DESCRIPTION_LINE_1")"
  local desc2_esc
  desc2_esc="$(escape_sed "$DESCRIPTION_LINE_2")"
  local binary_name_esc
  binary_name_esc="$(escape_sed "$BINARY_NAME")"

  sed \
    -e "s|__PACKAGE_NAME__|${package_name_esc}|g" \
    -e "s|__VERSION__|${version_esc}|g" \
    -e "s|__ARCH__|${arch_esc}|g" \
    -e "s|__SUMMARY__|${summary_esc}|g" \
    -e "s|__DESCRIPTION_LINE_1__|${desc1_esc}|g" \
    -e "s|__DESCRIPTION_LINE_2__|${desc2_esc}|g" \
    -e "s|__DESCRIPTION__|${desc1_esc} ${desc2_esc}|g" \
    -e "s|__BINARY_NAME__|${binary_name_esc}|g" \
    "$src" > "$dst"
}

require_env PACKAGE_NAME
require_env PACKAGE_VERSION
require_env RUNTIME_DIR
require_env BINARY_NAME
require_env DEB_CONTROL_TEMPLATE
require_env RPM_SPEC_TEMPLATE
require_env SUMMARY
require_env DESCRIPTION_LINE_1
require_env DESCRIPTION_LINE_2
require_env GITHUB_OUTPUT

ARCH="amd64"
if [ ! -d "$RUNTIME_DIR" ]; then
  echo "Runtime directory not found: $RUNTIME_DIR" >&2
  exit 1
fi
if [ ! -f "$DEB_CONTROL_TEMPLATE" ]; then
  echo "Debian control template not found: $DEB_CONTROL_TEMPLATE" >&2
  exit 1
fi
if [ ! -f "$RPM_SPEC_TEMPLATE" ]; then
  echo "RPM spec template not found: $RPM_SPEC_TEMPLATE" >&2
  exit 1
fi
if ! command -v dpkg-deb >/dev/null 2>&1; then
  echo "dpkg-deb is required but missing" >&2
  exit 1
fi
if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "rpmbuild is required but missing" >&2
  exit 1
fi

work_dir="${RUNNER_TEMP}/linux-packages-${PACKAGE_NAME}"
rm -rf "$work_dir"
mkdir -p "$work_dir"

runtime_dir_abs="$(cd "$RUNTIME_DIR" && pwd)"
runtime_root="/usr/lib/${PACKAGE_NAME}"
runtime_internal="${runtime_root}/_internal"

# Build .deb
pkgroot="${work_dir}/${PACKAGE_NAME}-deb-root"
mkdir -p "$pkgroot/DEBIAN" "$pkgroot/usr/lib/${PACKAGE_NAME}" "$pkgroot/usr/bin"
cp -a "${runtime_dir_abs}/." "$pkgroot/usr/lib/${PACKAGE_NAME}/"
cat > "$pkgroot/usr/bin/${BINARY_NAME}" <<EOF_WRAPPER
#!/usr/bin/env bash
export LD_LIBRARY_PATH="${runtime_internal}:${runtime_root}:\${LD_LIBRARY_PATH:-}"
exec "${runtime_root}/${BINARY_NAME}" "\$@"
EOF_WRAPPER
chmod 0755 "$pkgroot/usr/bin/${BINARY_NAME}"
render_template "$DEB_CONTROL_TEMPLATE" "$pkgroot/DEBIAN/control"
chmod 0755 "$pkgroot/usr/lib/${PACKAGE_NAME}/${BINARY_NAME}" || true

deb_name="${PACKAGE_NAME}_${PACKAGE_VERSION}_${ARCH}.deb"
deb_path="${work_dir}/${deb_name}"
dpkg-deb --build --root-owner-group "$pkgroot" "$deb_path"
deb_sha256="$(sha256sum "$deb_path" | awk '{print $1}')"

# Build .rpm
rpm_top="${work_dir}/rpmbuild"
mkdir -p "$rpm_top/BUILD" "$rpm_top/BUILDROOT" "$rpm_top/RPMS" "$rpm_top/SOURCES" "$rpm_top/SPECS" "$rpm_top/SRPMS"

src_root_name="${PACKAGE_NAME}-${PACKAGE_VERSION}"
src_root="${work_dir}/${src_root_name}"
mkdir -p "$src_root/usr/lib/${PACKAGE_NAME}" "$src_root/usr/bin"
cp -a "${runtime_dir_abs}/." "$src_root/usr/lib/${PACKAGE_NAME}/"
cat > "$src_root/usr/bin/${BINARY_NAME}" <<EOF_WRAPPER
#!/usr/bin/env bash
export LD_LIBRARY_PATH="${runtime_internal}:${runtime_root}:\${LD_LIBRARY_PATH:-}"
exec "${runtime_root}/${BINARY_NAME}" "\$@"
EOF_WRAPPER
chmod 0755 "$src_root/usr/bin/${BINARY_NAME}"

tar -czf "${rpm_top}/SOURCES/${src_root_name}.tar.gz" -C "$work_dir" "$src_root_name"
rpm_spec_path="${rpm_top}/SPECS/${PACKAGE_NAME}.spec"
render_template "$RPM_SPEC_TEMPLATE" "$rpm_spec_path"

rpmbuild --define "_topdir ${rpm_top}" -bb "$rpm_spec_path"

rpm_path_source="$(find "${rpm_top}/RPMS" -type f -name '*.rpm' | head -n 1)"
if [ -z "$rpm_path_source" ]; then
  echo "Failed to locate built RPM output" >&2
  exit 1
fi
rpm_name="$(basename "$rpm_path_source")"
rpm_path="${work_dir}/${rpm_name}"
cp "$rpm_path_source" "$rpm_path"
rpm_sha256="$(sha256sum "$rpm_path" | awk '{print $1}')"

{
  echo "deb_path=$(realpath "$deb_path")"
  echo "deb_name=${deb_name}"
  echo "deb_sha256=${deb_sha256}"
  echo "rpm_path=$(realpath "$rpm_path")"
  echo "rpm_name=${rpm_name}"
  echo "rpm_sha256=${rpm_sha256}"
} >> "$GITHUB_OUTPUT"
