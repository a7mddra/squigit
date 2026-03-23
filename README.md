# <img src="docs/data/header.svg" alt="logo">

[![Squigit CI](https://github.com/a7mddra/squigit/actions/workflows/distribute.yml/badge.svg?branch=v1.0.0)](https://github.com/a7mddra/squigit/actions/workflows/distribute.yml)
[![squigit](https://img.shields.io/github/package-json/v/a7mddra/squigit?filename=package.json&label=squigit&color=orange)](https://github.com/a7mddra/squigit/blob/main/package.json)
[![License](https://img.shields.io/github/license/a7mddra/squigit)](https://github.com/a7mddra/squigit/blob/main/LICENSE)

![Squigit demo](docs/data/demo.gif)

Squigit is an open-source "Circle to Search" desktop utility that brings the power of AI vision directly to your screen. It provides seamless screen capture, analysis, and visual search capabilities with a single hotkey.

Explore detailed guides and architecture in our [documentation](https://github.com/a7mddra/squigit/tree/main/docs).

## 🚀 Why Squigit?

- **🎯 Native Performance**: Polyglot architecture with C++ capture, Rust orchestration, and Tauri UI.
- **🔗 Bring Your Own Key (BYOK)**: No middleman. Plug your own Gemini and ImgBB API keys for direct, cost-effective access.
- **⚡ Instant Global Hotkey**: `Super/Cmd/Win + Shift + A` freezes your desktop for instant capture from anywhere.
- **🧠 Multi-Model Support**: Switch between Gemini 3.1 Flash (efficiency) and Gemini 3.1 Pro (power).
- **🪄 Google Lens Integration**: One-click visual search by automatically hosting images and launching Google Lens.
- **🔒 Local-First Security**: Your API keys, images, and conversations never leave your machine.

## 📦 Download

### Pre-requisites before installation

- Windows 10+, macOS 12+, or Linux (X11/Wayland)
- 64-bit system

### Quick Install

Download the appropriate installer for your operating system:

#### Windows

Download [`Squigit_Installer.exe`](https://github.com/a7mddra/squigit/releases/download/installers/squigit-installer-win-x64.zip).

#### macOS

Download [`squigit-installer.dmg`](https://github.com/a7mddra/squigit/releases/download/installers/squigit-installer-mac-x64.zip).

#### Linux

Download [`squigit-installer`](https://github.com/a7mddra/squigit/releases/download/installers/squigit-installer-linux-x64.zip).

## Installation

Grab the latest version of Squigit for your operating system from our [Latest Releases](https://github.com/a7mddra/squigit/releases/latest) page.

### 🪟 Windows

1. Download the latest Windows setup executable.
2. Run the installer.
3. **SmartScreen Warning:** Because Squigit is an open-source project without a commercial EV certificate, Microsoft Defender SmartScreen may initially flag the installer as unrecognized.
   - **Fix:** If you see the "Windows protected your PC" screen, simply click **More info**, and then select **Run anyway**.

### 🍎 macOS (Apple Silicon / ARM only)

_Note: Squigit's local AI engines are currently optimized strictly for Apple Silicon (M1/M2/M3/M4). Intel Macs are not supported._

1. Download the latest `.dmg` file.
2. Open it and drag **Squigit** into your `Applications` folder.
3. **Gatekeeper / "App is damaged" Warning:** Because we use an ad-hoc open-source signature, macOS Gatekeeper may block the app from launching or claim the file is damaged.
   - **Fix:** Open your **Terminal** and run this command to clear the Apple quarantine flag:
     ```bash
     sudo xattr -rd com.apple.quarantine /Applications/Squigit.app
     ```
   - _(Alternatively: Go to System Settings -> Privacy & Security, scroll to the bottom, and click "Open Anyway")._

### 🐧 Linux (AppImage only)

_Note: Squigit is distributed as an AppImage to ensure maximum compatibility across different Linux distributions._

1. Download the latest `.AppImage` file.
2. **Make it Executable:** By default, Linux prevents new downloads from running as programs. You must grant the file execution permissions before launching it.
   - **Fix:** Open your terminal in the directory where you downloaded the file and run:
     ```bash
     chmod +x Squigit-*.AppImage
     ```
   - You can now double-click the file to run it, or integrate it into your desktop environment using tools like AppImageLauncher.

## ⭐ Key Features

### Instant Screen Capture & Analysis

- Press `Super/Cmd/Win + Shift + A` anywhere to freeze your screen
- Draw any shape around content with intelligent cropping
- Get immediate AI analysis powered by Google Gemini models
- Continue conversations with follow-up questions

### Multi-Modal AI Integration

- **Direct Gemini API Access**: Your prompts go straight to Google's models with zero hops
- **Visual Context Injection**: Screenshots are automatically included in chat context
- **Model Hot-Swapping**: Instantly switch between Gemini models for different tasks

### Cross-Platform Visual Search

- **One-Click Google Lens**: Upload screenshots to ImgBB and open in Google Lens
- **Secure Image Handling**: Local encryption with AES-256 for API keys
- **Privacy-First**: No central server, no data collection

### Advanced Desktop Integration

- **Global Hotkey Registration**: Works across all applications
- **Multi-Monitor Support**: Captures and manages all displays simultaneously
- **Platform-Optimized Capture**: Different strategies for X11, Wayland, Windows, and macOS

## 🔐 Authentication: Bring Your Own Key ([BYOK](docs/06-policies/BYOK.md))

Squigit operates on a zero-trust, local-first model where you provide your own API keys.

### Setup Process

1. **Launch Squigit** and signup
2. **Generate API Keys** on the official provider sites:
   - **Gemini**: [Google AI Studio](https://aistudio.google.com/app/apikey)
   - **ImgBB**: [ImgBB API](https://api.imgbb.com/)
3. **Copy to Clipboard** - Squigit automatically detects and securely stores your keys

### Security Guarantees

- **Local Storage**: Keys encrypted with AES-256 and stored in your user directory
- **Direct Communication**: No proxy servers - requests go directly to `generativelanguage.googleapis.com` and `api.imgbb.com`
- **Clipboard Safety**: Keys are never manually pasted; automatic detection prevents keylogging exposure
- **Revocable Access**: You control and can revoke keys anytime via provider dashboards

## 🚀 Getting Started

### Basic Usage

#### Instant Capture (Hotkey Method)

1. Press `Super/Cmd/Win + Shift + A`
2. Draw a shape around the content you want to analyze
3. Release to capture and open Squigit with AI analysis

#### Direct Application Launch

1. Open Squigit from your applications menu
2. Drag & drop an image or use the file dialog
3. Chat with the AI about your uploaded image

#### Google Lens Integration

1. Capture or upload an image
2. Click the "Google Lens" button in the chat interface
3. Visual search opens in your default browser

### Quick Examples

#### Analyze Code from Screen

```bash
# 1. Press Super+Shift+A
# 2. Draw around the code snippet
# 3. Ask: "Explain this function and suggest improvements"
```

#### Research Web Content

```bash
# 1. Press Super+Shift+A on an article
# 2. Draw around the text
# 3. Ask: "Summarize the key points and find related sources"
```

#### Visual Product Search

```bash
# 1. Press Super+Shift+A on a product image
# 2. Draw around the product
# 3. Click "Google Lens" to find it online
```

## 📚 Documentation

### Getting Started

- [**Quick Start Guide**](docs/01-getting-started/QUICKSTART.md) - First-time setup and basic usage
- [**Installation Guide**](docs/01-getting-started/INSTALLATION.md) - Detailed installation instructions
- [**BYOK Model**](docs/06-policies/BYOK.md) - Understanding Bring Your Own Key

### Architecture & Development

- [**System Architecture**](docs/02-architecture/ARCHITECTURE.md) - High-level overview with diagrams
- [**Build System**](docs/02-architecture/BUILD.md) - Compilation and packaging
- [**CaptureKit**](docs/02-architecture/CAPTUREKIT.md) - Screen capture engine (C++/Qt6)
- [**Orchestrator**](docs/02-architecture/ORCHESTRATOR.md) - Core lifecycle manager (Rust)
- [**Squigit App**](docs/02-architecture/SQUIGIT.md) - Electron/React application

### Development & Contribution

- [**Development Guide**](docs/03-development/DEVELOPMENT.md) - Setting up your dev environment
- [**Contributing Guide**](docs/03-development/CONTRIBUTING.md) - How to contribute
- [**Debugging Guide**](docs/03-development/DEBUGGING.md) - Troubleshooting and diagnostics

### Deployment & Operations

- [**CI/CD Architecture**](docs/04-deployment/CI_ARCHITECTURE.md) - Automated build pipelines
- [**Installer Architecture**](docs/04-deployment/INSTALLERS.md) - Cross-platform deployment
- [**Release Strategy**](docs/04-deployment/RELEASE_STRATEGY.md) - Version control and updates

### API Reference

- [**Configuration Guide**](docs/05-api-reference/CONFIGURATION.md) - Runtime configuration files
- [**IPC Protocol**](docs/05-api-reference/IPC_PROTOCOL.md) - Inter-process communication

### Policies

- [**Code of Conduct**](docs/06-policies/CODE_OF_CONDUCT.md) - Community guidelines
- [**Security Policy**](docs/06-policies/SECURITY.md) - Vulnerability reporting and practices

## 🤝 Contributing

We welcome contributions! Squigit is fully open source, and we encourage the community to:

- Report bugs and suggest features
- Improve documentation
- Submit code improvements
- Share platform-specific optimizations

See our [Contributing Guide](docs/03-development/CONTRIBUTING.md) for development setup, coding standards, and how to submit pull requests.

Check our [TODO List](TODO.md) for planned features and priorities.

## 📖 Resources

- **[Documentation](https://github.com/a7mddra/squigit/tree/main/docs)** - Complete documentation hub
- **[GitHub Issues](https://github.com/a7mddra/squigit/issues)** - Report bugs or request features
- **[Security Policy](docs/06-policies/SECURITY.md)** - Security updates and reporting

## 📄 Legal

- **License**: [Apache License 2.0](LICENSE)
- **Security**: [Security Policy](docs/06-policies/SECURITY.md)
- **Privacy**: No data collection, local-only operation

---

<p align="center">
  Built with ❤️ by the open source community
</p>
