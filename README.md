# Matter Controller

A desktop application for commissioning and controlling [Matter](https://csa-iot.org/developer-resource/specifications-download-request/) smart home devices. Runs on macOS, Windows, and Linux.

## What it does

- **Discover** commissionable devices on your network via mDNS or Bluetooth
- **Commission** devices using a pairing code, IP address, or BLE
- **Monitor** all commissioned devices and their reachability status
- **Inspect** the full attribute tree of any device -- every endpoint, cluster, and attribute readable from the device
- **Control** lights, dimmers, and other controllable devices directly from the UI
- **Send commands** to any cluster on any endpoint
- **Write attributes** directly to a device with full control over value type

## Download

Download the latest release from the [Releases](../../releases) page and pick the package for your platform:

| Platform | Installer | Portable |
|---|---|---|
| macOS | `.dmg` | `.zip` (drag-and-drop `.app`) |
| Windows | `.msi` | `.zip` (single `.exe`) |
| Linux | `.deb` or `.AppImage` | `.zip` (single binary) |

## Prerequisites

### macOS: BLE commissioning

To commission devices over Bluetooth, you need the **Bluetooth Central Matter Client Developer Mode** profile installed on your Mac. This is a `.mobileconfig` file available from the Apple Developer portal. Without it, macOS blocks BLE Matter setup.

1. Download the profile from your Apple Developer account
2. Double-click the downloaded `.mobileconfig` file
3. Open **System Settings > Privacy & Security > Profiles** and approve the installation

This step is only needed if you plan to commission devices over Bluetooth. mDNS and pairing-code commissioning work without it.

## First launch

On first launch the app creates a new Matter fabric automatically. No configuration is needed.

Matter state (fabric config, device registry, certificates) is stored in a platform-specific directory:

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/com.matc.ui/matc/` |
| Windows | `%APPDATA%\com.matc.ui\matc\` |
| Linux | `~/.local/share/com.matc.ui/matc/` |

To reset the app to a clean state, delete that directory and relaunch.

## Usage

### Commissioning a device

Open the **Commission** tab. You can:

- Enter a **pairing code** from the device label or QR code
- Scan the local network with **mDNS Scan** and commission a discovered device
- Use **BLE Scan** to find and commission a device over Bluetooth (requires the profile above)

### Controlling devices

The **Control** tab shows all commissioned endpoints that support OnOff, Level, or Color Control. Use the toggle, slider, or color swatch to send commands instantly.

### Inspecting attributes

Open a device from the **Devices** tab, then switch to the **Attributes** tab. The app reads every endpoint, cluster, and attribute directly from the device and displays the full tree with decoded values. Use the **Send Command** tab to send raw cluster commands, or **Write** to set an attribute value.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for build instructions and project internals.
