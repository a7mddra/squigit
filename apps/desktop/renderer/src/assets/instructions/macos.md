# macOS Configuration & Privacy Policy

SnapLLM requires specific system permissions to capture screen data and process global hotkeys. Please review the following configuration steps and privacy protocols.

## 1. Installation Verification

Due to Apple's security policies for open-source software, macOS may flag the application as "damaged" upon first launch.

- **Action:** If launch fails, open **Terminal** and execute:

```bash
    `xattr -cr /Applications/SnapLLM.app`
```

## 2. Essential Permissions

To prevent "black screen" captures and ensure hotkey responsiveness, you must grant the following in **System Settings > Privacy & Security**:

1. **Input Monitoring** (Required)
   - _Enable for:_ **SnapLLM**
   - _Function:_ Allows the background daemon to detect the `Cmd+Shift+A` trigger.
2. **Screen Recording** (Critical)
   - _Enable for:_ **SnapLLM**
   - _Function:_ Grants the capture engine access to display pixels. **Failure to enable this will result in black/empty screenshots.**
3. **Automation** (Optional)
   - _Enable for:_ **SnapLLM > System Events**
   - _Function:_ Allows the application to silence the system shutter sound during capture.

## 3. Usage

- **Trigger:** Press `Cmd ⌘` + `Shift ⇧` + `A`.
- **Analysis:** Draw a region to instantly capture and analyze.

## 4. Zero-Trust Architecture

- **Local-First:** Application logic and encryption occur locally on your device.
- **Direct Connection:** API requests are sent directly to Google (`generativelanguage.googleapis.com`). No intermediate servers are used.
- **Lens Feature:** This optional feature uses ImgBB as a temporary bridge. Do not use "Lens" mode for sensitive personal data.
