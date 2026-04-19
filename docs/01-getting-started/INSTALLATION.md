# Installation Guide

Squigit is distributed via the official GitHub releases page. The installation process and required dependencies vary depending on the host operating system.

## Windows

1. Download the latest Windows executable installer (`.exe`) from the GitHub releases page.
2. Execute the downloaded installer.
3. **Microsoft Defender SmartScreen Configuration:** As an open-source application without a commercial Extended Validation (EV) certificate, Microsoft Defender SmartScreen may flag the installer as an unrecognized application.
   - **Resolution:** Upon encountering the "Windows protected your PC" prompt, select **More info**, followed by **Run anyway** to proceed with the installation.

---

## macOS (Apple Silicon / ARM Exclusive)

_Note: The local AI engines utilized by Squigit are strictly optimized for Apple Silicon (M1/M2/M3/M4) architectures. Intel-based Mac systems are currently unsupported._

1. Download the latest macOS `.dmg` disk image from the GitHub releases page.
2. Mount the disk image and transfer the **Squigit** application to the system's `/Applications` directory.
3. **Gatekeeper Security Configuration:** Due to the use of an ad-hoc open-source signature, macOS Gatekeeper may restrict the application from launching or erroneously report the file as damaged.
   - **Terminal Resolution:** Execute the following command in the macOS Terminal to forcefully remove the Apple quarantine attribute:
     ```bash
     sudo xattr -rd com.apple.quarantine /Applications/Squigit.app
     ```
   - **Alternative GUI Resolution:** Navigate to **System Settings -> Privacy & Security**, locate the security prompt near the bottom of the interface, and select **Open Anyway**.

---

## Linux (AppImage)

_Note: Squigit for Linux is distributed exclusively as an AppImage to maximize cross-distribution compatibility. Due to Continuous Integration (CI) size constraints, the Linux AppImage does not bundle the necessary Optical Character Recognition (OCR) and Speech-to-Text (STT/Whisper) dependencies. These components must be installed manually via the system's package manager. The Tauri backend will automatically locate their system paths upon execution._

### 1. Dependency Installation

The required external dependencies must be installed prior to running the application.

**For Debian/Ubuntu-based distributions (APT):**

1. Add the Squigit package repository:

```bash
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://squigit-org.github.io/squigit-packages/keys/squigit-packages.asc | \
  gpg --dearmor | sudo tee /etc/apt/keyrings/squigit-packages.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/squigit-packages.gpg] https://github.com/squigit-org/squigit-packages/raw/main/apt stable ocr stt" | \
  sudo tee /etc/apt/sources.list.d/squigit-packages.list >/dev/null
```

2. Update the package cache:

```bash
sudo apt update
```

3. Install dependencies:

```bash
sudo apt install squigit-ocr squigit-stt
```

**For Fedora/RHEL-based distributions (DNF):**

1. Add the Squigit repository file:

```bash
sudo curl -fsSL https://squigit-org.github.io/squigit-packages/rpm/squigit.repo \
  -o /etc/yum.repos.d/squigit.repo
```

2. Update the package cache:

```bash
sudo dnf makecache
```

3. Install dependencies:

```bash
sudo dnf install squigit-ocr squigit-stt
```

### 2. Application Execution

1. Download the latest `.AppImage` release from the GitHub repository.
2. Grant execution permissions to the downloaded AppImage file via the terminal:
   ```bash
   chmod +x Squigit-*.AppImage
   ```
3. Execute the file directly by invoking it in the terminal or double-clicking it via a file manager. Alternatively, it can be integrated into the desktop environment utilizing third-party utilities such as `AppImageLauncher`.
