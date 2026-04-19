#!/usr/bin/env bash
set -euo pipefail

require_env() {
  local key="$1"
  if [ -z "${!key:-}" ]; then
    echo "Missing required env: ${key}" >&2
    exit 1
  fi
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Required command missing: ${cmd}" >&2
    exit 1
  fi
}

download_asset_with_retry() {
  local url="$1"
  local target="$2"
  local attempt

  for attempt in $(seq 1 12); do
    if curl --fail --silent --show-error --location --head --max-time 30 "${url}" >/dev/null; then
      break
    fi
    if [ "${attempt}" -eq 12 ]; then
      echo "Release asset did not propagate in time: ${url}" >&2
      exit 1
    fi
    sleep 10
  done

  curl --fail --silent --show-error --location "${url}" -o "${target}"
}

require_env PACKAGES_TOKEN
require_env PACKAGES_REPO
require_env PACKAGES_BRANCH
require_env PACKAGE_NAME
require_env PACKAGE_COMPONENT
require_env PACKAGE_VERSION
require_env SOURCE_RELEASE_REPO
require_env SOURCE_RELEASE_TAG
require_env DEB_PATH
require_env RPM_PATH
require_env GPG_PRIVATE_KEY
require_env GPG_PASSPHRASE
require_env PAGES_BASE_URL
require_env APT_SUITE
require_env GITHUB_OUTPUT

PUBLISH_DNF_REPO="${PUBLISH_DNF_REPO:-true}"
if [ "${PUBLISH_DNF_REPO}" != "true" ] && [ "${PUBLISH_DNF_REPO}" != "false" ]; then
  echo "PUBLISH_DNF_REPO must be 'true' or 'false', got: ${PUBLISH_DNF_REPO}" >&2
  exit 1
fi

if [ "${PACKAGE_COMPONENT}" != "ocr" ] && [ "${PACKAGE_COMPONENT}" != "stt" ]; then
  echo "PACKAGE_COMPONENT must be 'ocr' or 'stt', got: ${PACKAGE_COMPONENT}" >&2
  exit 1
fi

if [ ! -f "${DEB_PATH}" ]; then
  echo "Debian package not found: ${DEB_PATH}" >&2
  exit 1
fi

if [ ! -f "${RPM_PATH}" ]; then
  echo "RPM package not found: ${RPM_PATH}" >&2
  exit 1
fi

for cmd in git gpg dpkg-scanpackages apt-ftparchive createrepo_c gzip curl; do
  require_cmd "$cmd"
done

repo_dir="${RUNNER_TEMP}/squigit-packages-repo"
rm -rf "${repo_dir}"

export GNUPGHOME="${RUNNER_TEMP}/squigit-packages-gnupg"
rm -rf "${GNUPGHOME}"
mkdir -p "${GNUPGHOME}"
chmod 700 "${GNUPGHOME}"

printf '%s' "${GPG_PRIVATE_KEY}" | gpg --batch --import

gpg_key_fpr="$(gpg --batch --with-colons --list-secret-keys | awk -F: '/^fpr:/ {print $10; exit}')"
if [ -z "${gpg_key_fpr}" ]; then
  echo "Failed to resolve imported GPG key fingerprint" >&2
  exit 1
fi

git clone "https://x-access-token:${PACKAGES_TOKEN}@github.com/${PACKAGES_REPO}.git" "${repo_dir}"
cd "${repo_dir}"
git checkout "${PACKAGES_BRANCH}"

raw_base_url="https://github.com/${PACKAGES_REPO}/raw/${PACKAGES_BRANCH}"
raw_main_base_url="https://github.com/${PACKAGES_REPO}/raw/main"
public_base_url="${PAGES_BASE_URL%/}"
source_release_root="https://github.com/${SOURCE_RELEASE_REPO}/releases/download"

mkdir -p "apt/dists/${APT_SUITE}/ocr/binary-amd64"
mkdir -p "apt/dists/${APT_SUITE}/stt/binary-amd64"
mkdir -p "rpm/ocr" "rpm/stt" "keys" "metadata"

rm -rf apt/pool
if [ "${PUBLISH_DNF_REPO}" = "true" ]; then
  find rpm/ocr rpm/stt -maxdepth 1 -type f -name '*.rpm' -delete || true
fi

manifest_path="metadata/package-assets.env"
if [ -f "${manifest_path}" ]; then
  # shellcheck disable=SC1090
  source "${manifest_path}"
fi

OCR_DEB_TAG="${OCR_DEB_TAG:-}"
OCR_DEB_NAME="${OCR_DEB_NAME:-}"
OCR_RPM_TAG="${OCR_RPM_TAG:-}"
OCR_RPM_NAME="${OCR_RPM_NAME:-}"
STT_DEB_TAG="${STT_DEB_TAG:-}"
STT_DEB_NAME="${STT_DEB_NAME:-}"
STT_RPM_TAG="${STT_RPM_TAG:-}"
STT_RPM_NAME="${STT_RPM_NAME:-}"

if [ "${PACKAGE_COMPONENT}" = "ocr" ]; then
  OCR_DEB_TAG="${SOURCE_RELEASE_TAG}"
  OCR_DEB_NAME="$(basename "${DEB_PATH}")"
  OCR_RPM_TAG="${SOURCE_RELEASE_TAG}"
  OCR_RPM_NAME="$(basename "${RPM_PATH}")"
else
  STT_DEB_TAG="${SOURCE_RELEASE_TAG}"
  STT_DEB_NAME="$(basename "${DEB_PATH}")"
  STT_RPM_TAG="${SOURCE_RELEASE_TAG}"
  STT_RPM_NAME="$(basename "${RPM_PATH}")"
fi

declare -A COMPONENT_DEB_PATHS
declare -A COMPONENT_DEB_TAGS
declare -A COMPONENT_DEB_NAMES
declare -A COMPONENT_RPM_PATHS
declare -A COMPONENT_RPM_TAGS
declare -A COMPONENT_RPM_NAMES

assets_dir="${RUNNER_TEMP}/squigit-component-assets"
rm -rf "${assets_dir}"
mkdir -p "${assets_dir}"

for component in ocr stt; do
  component_deb_tag=""
  component_deb_name=""
  component_rpm_tag=""
  component_rpm_name=""

  case "${component}" in
    ocr)
      component_deb_tag="${OCR_DEB_TAG}"
      component_deb_name="${OCR_DEB_NAME}"
      component_rpm_tag="${OCR_RPM_TAG}"
      component_rpm_name="${OCR_RPM_NAME}"
      ;;
    stt)
      component_deb_tag="${STT_DEB_TAG}"
      component_deb_name="${STT_DEB_NAME}"
      component_rpm_tag="${STT_RPM_TAG}"
      component_rpm_name="${STT_RPM_NAME}"
      ;;
  esac

  COMPONENT_DEB_TAGS["${component}"]="${component_deb_tag}"
  COMPONENT_DEB_NAMES["${component}"]="${component_deb_name}"
  COMPONENT_RPM_TAGS["${component}"]="${component_rpm_tag}"
  COMPONENT_RPM_NAMES["${component}"]="${component_rpm_name}"

  if [ "${component}" = "${PACKAGE_COMPONENT}" ]; then
    COMPONENT_DEB_PATHS["${component}"]="${DEB_PATH}"
    COMPONENT_RPM_PATHS["${component}"]="${RPM_PATH}"
    continue
  fi

  component_dir="${assets_dir}/${component}"
  mkdir -p "${component_dir}"

  if [ -n "${component_deb_tag}" ] && [ -n "${component_deb_name}" ]; then
    deb_url="${source_release_root}/${component_deb_tag}/${component_deb_name}"
    deb_target="${component_dir}/${component_deb_name}"
    download_asset_with_retry "${deb_url}" "${deb_target}"
    COMPONENT_DEB_PATHS["${component}"]="${deb_target}"
  fi

  if [ -n "${component_rpm_tag}" ] && [ -n "${component_rpm_name}" ]; then
    rpm_url="${source_release_root}/${component_rpm_tag}/${component_rpm_name}"
    rpm_target="${component_dir}/${component_rpm_name}"
    download_asset_with_retry "${rpm_url}" "${rpm_target}"
    COMPONENT_RPM_PATHS["${component}"]="${rpm_target}"
  fi
done

for component in ocr stt; do
  component_dir="apt/dists/${APT_SUITE}/${component}/binary-amd64"
  packages_file="${component_dir}/Packages"
  mkdir -p "${component_dir}"

  deb_path_local="${COMPONENT_DEB_PATHS[${component}]:-}"
  deb_name="${COMPONENT_DEB_NAMES[${component}]:-}"
  deb_tag="${COMPONENT_DEB_TAGS[${component}]:-}"

  if [ -n "${deb_path_local}" ] && [ -f "${deb_path_local}" ] && [ -n "${deb_name}" ] && [ -n "${deb_tag}" ]; then
    apt_scan_root="${RUNNER_TEMP}/apt-scan-${component}"
    rm -rf "${apt_scan_root}"
    mkdir -p "${apt_scan_root}/pool/${component}"
    cp "${deb_path_local}" "${apt_scan_root}/pool/${component}/${deb_name}"

    (
      cd "${apt_scan_root}"
      dpkg-scanpackages --multiversion "pool/${component}" /dev/null > "${repo_dir}/${packages_file}"
    )

    deb_filename="../../../../../${SOURCE_RELEASE_REPO}/releases/download/${deb_tag}/${deb_name}"
    sed -i "s|^Filename: .*|Filename: ${deb_filename}|" "${packages_file}"
  else
    : > "${packages_file}"
  fi

  gzip -9 -c "${packages_file}" > "${packages_file}.gz"
done

apt-ftparchive \
  -o "APT::FTPArchive::Release::Origin=Squigit Org" \
  -o "APT::FTPArchive::Release::Label=Squigit Packages" \
  -o "APT::FTPArchive::Release::Suite=${APT_SUITE}" \
  -o "APT::FTPArchive::Release::Codename=${APT_SUITE}" \
  -o "APT::FTPArchive::Release::Architectures=amd64" \
  -o "APT::FTPArchive::Release::Components=ocr stt" \
  release "apt/dists/${APT_SUITE}" > "apt/dists/${APT_SUITE}/Release"

gpg --batch --yes --pinentry-mode loopback --passphrase "${GPG_PASSPHRASE}" --local-user "${gpg_key_fpr}" \
  --clearsign -o "apt/dists/${APT_SUITE}/InRelease" "apt/dists/${APT_SUITE}/Release"

gpg --batch --yes --pinentry-mode loopback --passphrase "${GPG_PASSPHRASE}" --local-user "${gpg_key_fpr}" \
  --detach-sign --armor -o "apt/dists/${APT_SUITE}/Release.gpg" "apt/dists/${APT_SUITE}/Release"

if [ "${PUBLISH_DNF_REPO}" = "true" ]; then
  for component in ocr stt; do
    component_repo_dir="rpm/${component}"
    temp_repo_dir="${RUNNER_TEMP}/rpm-repo-${component}"
    rm -rf "${temp_repo_dir}"
    mkdir -p "${temp_repo_dir}"

    rpm_path_local="${COMPONENT_RPM_PATHS[${component}]:-}"
    rpm_name="${COMPONENT_RPM_NAMES[${component}]:-}"
    rpm_tag="${COMPONENT_RPM_TAGS[${component}]:-}"

    if [ -n "${rpm_path_local}" ] && [ -f "${rpm_path_local}" ] && [ -n "${rpm_name}" ] && [ -n "${rpm_tag}" ]; then
      cp "${rpm_path_local}" "${temp_repo_dir}/${rpm_name}"
      createrepo_c --simple-md-filenames --baseurl "${source_release_root}/${rpm_tag}/" "${temp_repo_dir}"
    else
      createrepo_c --simple-md-filenames "${temp_repo_dir}"
    fi

    rm -rf "${component_repo_dir}/repodata"
    mkdir -p "${component_repo_dir}"
    cp -a "${temp_repo_dir}/repodata" "${component_repo_dir}/"

    gpg --batch --yes --pinentry-mode loopback --passphrase "${GPG_PASSPHRASE}" --local-user "${gpg_key_fpr}" \
      --detach-sign --armor -o "${component_repo_dir}/repodata/repomd.xml.asc" "${component_repo_dir}/repodata/repomd.xml"
  done
fi

cat > "rpm/squigit.repo" <<EOF_REPO
[squigit-ocr]
name=Squigit OCR Packages
baseurl=${public_base_url}/rpm/ocr
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=${public_base_url}/keys/squigit-packages.asc

[squigit-stt]
name=Squigit STT Packages
baseurl=${public_base_url}/rpm/stt
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=${public_base_url}/keys/squigit-packages.asc
EOF_REPO

gpg --batch --yes --output "keys/squigit-packages.gpg" --export "${gpg_key_fpr}"
gpg --batch --yes --armor --output "keys/squigit-packages.asc" --export "${gpg_key_fpr}"

cat > "${manifest_path}" <<EOF_MANIFEST
# Autogenerated by .github/actions/publish-linux-packages-repo/publish.sh
SOURCE_RELEASE_REPO=${SOURCE_RELEASE_REPO}
OCR_DEB_TAG=${OCR_DEB_TAG}
OCR_DEB_NAME=${OCR_DEB_NAME}
OCR_RPM_TAG=${OCR_RPM_TAG}
OCR_RPM_NAME=${OCR_RPM_NAME}
STT_DEB_TAG=${STT_DEB_TAG}
STT_DEB_NAME=${STT_DEB_NAME}
STT_RPM_TAG=${STT_RPM_TAG}
STT_RPM_NAME=${STT_RPM_NAME}
EOF_MANIFEST

cat > README.md <<'EOF_README'
# Squigit Packages

Signed APT and DNF metadata for Squigit sidecar packages.

- APT metadata root: `apt/`
- DNF metadata root: `rpm/`
- Public key: `keys/squigit-packages.asc`
- Current Debian package filenames/tags are tracked in `metadata/package-assets.env`
- Package binaries are served from `squigit-org/squigit` GitHub Releases.
EOF_README

cat > index.html <<EOF_INDEX
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Squigit Packages</title>
    <style>
      :root {
        color-scheme: light;
      }
      body {
        margin: 0;
        font-family: "Instrument Sans", "Inter", "Segoe UI", sans-serif;
        background: linear-gradient(135deg, #f6f8ff 0%, #f2fff8 100%);
        color: #1f2937;
      }
      main {
        max-width: 900px;
        margin: 0 auto;
        padding: 48px 20px 72px;
      }
      h1 {
        margin-top: 0;
        font-size: 2rem;
      }
      code,
      pre {
        font-family: "JetBrains Mono", "Fira Code", monospace;
      }
      pre {
        background: #0f172a;
        color: #e2e8f0;
        padding: 14px;
        border-radius: 10px;
        overflow-x: auto;
      }
      .card {
        background: #ffffff;
        border: 1px solid #dbe6ff;
        border-radius: 14px;
        padding: 18px;
        margin-top: 18px;
      }
      a {
        color: #0f4fd3;
      }
    </style>
  </head>
  <body>
    <main>
      <h1>Squigit Linux Packages</h1>
      <p>
        Signed APT and DNF metadata for <code>squigit-ocr</code> and
        <code>squigit-stt</code>. Install these sidecars using the standard
        package-manager flow.
      </p>

      <div class="card">
        <h2>Debian/Ubuntu (APT repo)</h2>
        <pre>sudo mkdir -p /etc/apt/keyrings
curl -fsSL ${public_base_url}/keys/squigit-packages.asc | \
  gpg --dearmor | sudo tee /etc/apt/keyrings/squigit-packages.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/squigit-packages.gpg] ${raw_main_base_url}/apt ${APT_SUITE} ocr stt" | \
  sudo tee /etc/apt/sources.list.d/squigit-packages.list >/dev/null
sudo apt update
sudo apt install squigit-ocr squigit-stt</pre>
      </div>

      <div class="card">
        <h2>Fedora/RHEL (DNF repo)</h2>
        <pre>sudo curl -fsSL ${public_base_url}/rpm/squigit.repo -o /etc/yum.repos.d/squigit.repo
sudo dnf makecache
sudo dnf install squigit-ocr squigit-stt</pre>
      </div>

      <p>
        Public key files:
        <a href="keys/squigit-packages.asc">ASCII</a> /
        <a href="keys/squigit-packages.gpg">Binary</a>
      </p>
      <p><a href="metadata/package-assets.env">Latest package asset manifest</a></p>
    </main>
  </body>
</html>
EOF_INDEX

git config user.name "GitHub Actions Bot"
git config user.email "actions@github.com"
git add -A apt rpm keys metadata README.md index.html

if git diff --cached --quiet; then
  echo "No package metadata changes to publish."
else
  git commit -m "Publish ${PACKAGE_NAME} ${PACKAGE_VERSION} package metadata"
  git push origin "${PACKAGES_BRANCH}"
fi

packages_sha="$(git rev-parse HEAD)"

{
  echo "repo_url=https://github.com/${PACKAGES_REPO}"
  echo "apt_source=deb [signed-by=/etc/apt/keyrings/squigit-packages.gpg] ${raw_base_url}/apt ${APT_SUITE} ocr stt"
  echo "dnf_repo_url=${public_base_url}/rpm/squigit.repo"
  echo "packages_sha=${packages_sha}"
} >> "${GITHUB_OUTPUT}"
