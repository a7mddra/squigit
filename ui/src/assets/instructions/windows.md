# Windows Configuration & Privacy Policy

SnapLLM operates as a lightweight background service designed for minimal resource usage. Please review the following installation and privacy protocols.

## 1. Installation Verification (SmartScreen)

As an open-source project without a commercial EV certificate, Microsoft Defender SmartScreen may flag this installer as unrecognized during the initial setup.

- **Action:** If you encounter the **"Windows protected your PC"** prompt:
  1. Click **"More info"**.
  2. Select **"Run anyway"**.

## 2. Operational Workflow

- **Global Hotkey:** Press `Win` + `Shift` + `A`.
- **Process:** Trigger the hotkey to freeze the screen, draw a region, and immediately launch the AI analysis window.
- **Display Support:** The engine utilizes native Qt6 rendering to ensure pixel-perfect accuracy on High-DPI (4K) and multi-monitor configurations.

## 3. Zero-Trust Architecture

- **Local-First:** Your Google Gemini API Key is stored encrypted on your local disk. It is never transmitted to us.
- **Direct Connection:** API requests are sent directly to Google (`generativelanguage.googleapis.com`). No intermediate servers are used.
- **Lens Feature:** This optional feature uses ImgBB as a temporary bridge. Do not use "Lens" mode for sensitive personal data.
