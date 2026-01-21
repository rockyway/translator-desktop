# Deployment and Auto-Updates Guide

This guide covers deployment options for Translator Desktop, with focus on simple distribution and seamless updates.

---

## Quick Start: Setup Checklist

Most of the auto-update infrastructure has been implemented. Complete these manual steps to enable it:

### Step 1: Generate Signing Keys (One-time)

```powershell
# Run in project root - generates key pair for signing updates
npx tauri signer generate -w ~/.tauri/translator-desktop.key
```

Save the password securely - you'll need it for every release.

### Step 2: Update Configuration

Edit `src-tauri/tauri.conf.json` and replace placeholders:

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/YOUR_USERNAME/translator-desktop/releases/latest/download/latest.json"
      ],
      "pubkey": "PASTE_CONTENTS_OF_~/.tauri/translator-desktop.key.pub_HERE"
    }
  }
}
```

### Step 3: Configure GitHub Secrets

Go to your GitHub repo > Settings > Secrets > Actions, and add:

| Secret Name | Value |
|-------------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `~/.tauri/translator-desktop.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password you chose in Step 1 |

### Step 4: Install Dependencies

```bash
npm install
```

### Step 5: Test Build

```powershell
# Set environment variables for local build
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~/.tauri/translator-desktop.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-password"

# Build
npm run build:all
```

### Already Implemented

These components are ready to use:

| Component | File | Status |
|-----------|------|--------|
| Rust updater plugin | `src-tauri/Cargo.toml`, `lib.rs` | Done |
| Updater config | `src-tauri/tauri.conf.json` | Needs pubkey |
| Update check hook | `src/hooks/useUpdater.ts` | Done |
| Update UI component | `src/components/UpdateNotification.tsx` | Done |
| App integration | `src/App.tsx` | Done |
| GitHub Actions | `.github/workflows/release.yml` | Done |
| npm packages | `package.json` | Done |

---

## Table of Contents

1. [Deployment Options Overview](#deployment-options-overview)
2. [Recommended: NSIS + Tauri Updater](#recommended-nsis--tauri-updater)
3. [Alternative: MSI Installer](#alternative-msi-installer)
4. [Setting Up Auto-Updates](#setting-up-auto-updates)
5. [GitHub Releases Workflow](#github-releases-workflow)
6. [GitHub Actions Automation](#github-actions-automation)
7. [Manual Release Process](#manual-release-process)
8. [Troubleshooting](#troubleshooting)

---

## Deployment Options Overview

| Method | Best For | Auto-Updates | Complexity |
|--------|----------|--------------|------------|
| **NSIS + Updater** | Personal/small teams | Yes | Low |
| **MSI Installer** | Enterprise/GPO | Manual | Medium |
| **Portable EXE** | USB/no-install | No | Lowest |

### Recommendation

For Translator Desktop, **NSIS with Tauri Updater** is recommended because:
- Simplest setup and maintenance
- Automatic update prompts for users
- Small installer size (~15-20MB)
- Works seamlessly with GitHub Releases

---

## Recommended: NSIS + Tauri Updater

### Step 1: Install Dependencies

```bash
cd src-tauri
cargo add tauri-plugin-updater
```

### Step 2: Configure Bundle Targets

Update `src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "createUpdaterArtifacts": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico"
    ],
    "externalBin": ["text-monitor"],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  }
}
```

### Step 3: Generate Signing Keys

Tauri requires update packages to be signed. Generate a keypair:

```bash
# Generate keys (save the password securely!)
npx tauri signer generate -w ~/.tauri/translator-desktop.key
```

This creates:
- `~/.tauri/translator-desktop.key` - Private key (keep secret!)
- `~/.tauri/translator-desktop.key.pub` - Public key (embed in app)

**Important:** Back up your private key securely. If lost, users cannot verify updates.

### Step 4: Configure Updater Plugin

Add to `src-tauri/tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/YOUR_USERNAME/translator-desktop/releases/latest/download/latest.json"
      ],
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXkKUldRPT0K..."
    }
  }
}
```

Replace:
- `YOUR_USERNAME` with your GitHub username
- `pubkey` with contents of `~/.tauri/translator-desktop.key.pub`

### Step 5: Register Plugin in Rust

Update `src-tauri/src/lib.rs`:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        // ... other plugins
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Step 6: Add Update Check in Frontend

Create `src/hooks/useUpdater.ts`:

```typescript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useState, useEffect } from 'react';

interface UpdateStatus {
  available: boolean;
  version?: string;
  downloading: boolean;
  progress?: number;
  error?: string;
}

export function useUpdater(checkOnMount = true) {
  const [status, setStatus] = useState<UpdateStatus>({
    available: false,
    downloading: false,
  });

  const checkForUpdates = async () => {
    try {
      const update = await check();

      if (update) {
        setStatus({
          available: true,
          version: update.version,
          downloading: false,
        });
        return update;
      }

      setStatus({ available: false, downloading: false });
      return null;
    } catch (error) {
      setStatus({
        available: false,
        downloading: false,
        error: error instanceof Error ? error.message : 'Update check failed',
      });
      return null;
    }
  };

  const downloadAndInstall = async () => {
    const update = await check();
    if (!update) return;

    setStatus(prev => ({ ...prev, downloading: true, progress: 0 }));

    await update.downloadAndInstall((event) => {
      if (event.event === 'Progress') {
        const progress = (event.data.chunkLength / event.data.contentLength) * 100;
        setStatus(prev => ({ ...prev, progress }));
      }
    });

    // Relaunch the app after update
    await relaunch();
  };

  useEffect(() => {
    if (checkOnMount) {
      checkForUpdates();
    }
  }, [checkOnMount]);

  return {
    ...status,
    checkForUpdates,
    downloadAndInstall,
  };
}
```

### Step 7: Add Update UI Component

Create `src/components/UpdateNotification.tsx`:

```typescript
import { useUpdater } from '../hooks/useUpdater';

export function UpdateNotification() {
  const { available, version, downloading, progress, downloadAndInstall } = useUpdater();

  if (!available) return null;

  return (
    <div className="fixed bottom-4 right-4 bg-amber-500 text-black p-4 rounded-lg shadow-lg max-w-sm">
      <div className="font-semibold">Update Available</div>
      <div className="text-sm mt-1">
        Version {version} is ready to install.
      </div>

      {downloading ? (
        <div className="mt-2">
          <div className="h-2 bg-amber-300 rounded-full overflow-hidden">
            <div
              className="h-full bg-amber-700 transition-all"
              style={{ width: `${progress ?? 0}%` }}
            />
          </div>
          <div className="text-xs mt-1">Downloading... {Math.round(progress ?? 0)}%</div>
        </div>
      ) : (
        <button
          onClick={downloadAndInstall}
          className="mt-2 px-4 py-1 bg-black text-amber-500 rounded hover:bg-gray-900 text-sm font-medium"
        >
          Install & Restart
        </button>
      )}
    </div>
  );
}
```

---

## Alternative: MSI Installer

MSI is useful for enterprise deployment via Group Policy (GPO).

### Configure MSI Target

Update `src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "targets": ["msi"],
    "windows": {
      "wix": {
        "language": "en-US",
        "upgradeCode": "YOUR-UNIQUE-GUID-HERE"
      }
    }
  }
}
```

Generate a unique GUID:
```powershell
[guid]::NewGuid().ToString()
```

### MSI Limitations

- No built-in auto-update (requires separate mechanism like WSUS)
- Larger installer size
- More complex build process (requires WiX Toolset)

---

## GitHub Releases Workflow

### Update Manifest Format

Tauri generates `latest.json` automatically during build:

```json
{
  "version": "0.2.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2024-01-20T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1...",
      "url": "https://github.com/USER/REPO/releases/download/v0.2.0/Translator.Desktop_0.2.0_x64-setup.nsis.zip"
    }
  }
}
```

### Release Artifacts

Each release should include:

| File | Purpose |
|------|---------|
| `Translator.Desktop_x.x.x_x64-setup.exe` | NSIS installer |
| `Translator.Desktop_x.x.x_x64-setup.nsis.zip` | Signed update package |
| `Translator.Desktop_x.x.x_x64-setup.nsis.zip.sig` | Signature file |
| `latest.json` | Update manifest |

---

## GitHub Actions Automation

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}

jobs:
  build:
    runs-on: windows-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'

      - name: Setup Rust
        uses: dtolnay/rust-action@stable

      - name: Setup .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.0.x'

      - name: Install dependencies
        run: npm ci

      - name: Build sidecar
        run: npm run build:sidecar

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Translator Desktop ${{ github.ref_name }}'
          releaseBody: 'See the assets below to download and install.'
          releaseDraft: true
          prerelease: false
          includeUpdaterJson: true

  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Publish release
        uses: actions/github-script@v7
        with:
          script: |
            const { owner, repo } = context.repo;
            const releases = await github.rest.repos.listReleases({ owner, repo });
            const draft = releases.data.find(r => r.draft);
            if (draft) {
              await github.rest.repos.updateRelease({
                owner,
                repo,
                release_id: draft.id,
                draft: false
              });
            }
```

### Setting Up Secrets

Add these secrets in GitHub repo settings:

1. `TAURI_SIGNING_PRIVATE_KEY` - Contents of `~/.tauri/translator-desktop.key`
2. `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - Password used when generating key

### Creating a Release

```bash
# 1. Update version in tauri.conf.json
# 2. Commit changes
git add .
git commit -m "chore: bump version to 0.2.0"

# 3. Create and push tag
git tag v0.2.0
git push origin v0.2.0

# GitHub Actions will automatically build and create release
```

---

## Manual Release Process

If not using GitHub Actions:

### Step 1: Update Version

Edit `src-tauri/tauri.conf.json`:
```json
{
  "version": "0.2.0"
}
```

### Step 2: Build Release

```bash
# Set signing key environment variables
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~/.tauri/translator-desktop.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-password"

# Build
npm run build:all
```

### Step 3: Locate Artifacts

Build outputs are in `src-tauri/target/release/bundle/`:

```
bundle/
├── nsis/
│   ├── TranslatorDesktop_0.2.0_x64-setup.exe
│   ├── TranslatorDesktop_0.2.0_x64-setup.nsis.zip
│   └── TranslatorDesktop_0.2.0_x64-setup.nsis.zip.sig
└── latest.json
```

### Step 4: Create GitHub Release

1. Go to GitHub > Releases > "Create new release"
2. Tag: `v0.2.0`
3. Title: `Translator Desktop v0.2.0`
4. Upload all artifacts from step 3
5. Publish release

---

## Troubleshooting

### Update Check Fails

**Symptom:** `check()` throws error or returns null unexpectedly.

**Solutions:**
1. Verify `endpoints` URL is correct and accessible
2. Check that `latest.json` is uploaded to release
3. Ensure `pubkey` in config matches your signing key

### Signature Verification Failed

**Symptom:** Update downloads but fails to install.

**Solutions:**
1. Ensure `TAURI_SIGNING_PRIVATE_KEY` matches the `pubkey` in config
2. Re-generate keys if mismatch suspected
3. Check password is correct

### Sidecar Not Included in Update

**Symptom:** App updates but .NET monitor doesn't work.

**Solutions:**
1. Verify `externalBin` is configured in `tauri.conf.json`
2. Ensure sidecar is built before Tauri build
3. Check sidecar naming matches configuration

### NSIS Installer Blocked by Windows

**Symptom:** "Windows protected your PC" warning.

**Solutions:**
1. Code sign the installer with an EV certificate (removes warning)
2. Users can click "More info" > "Run anyway"
3. For enterprise: Add to Windows Defender exclusions via GPO

---

## Environment Variables Reference

| Variable | Purpose | Required |
|----------|---------|----------|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key for signing updates | Yes (for updates) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for private key | Yes (if key is encrypted) |

---

## Version Checklist

Before each release:

- [ ] Update `version` in `src-tauri/tauri.conf.json`
- [ ] Update changelog/release notes
- [ ] Test build locally: `npm run build:all`
- [ ] Verify sidecar is bundled correctly
- [ ] Test update from previous version
- [ ] Create git tag: `git tag vX.X.X`
- [ ] Push tag: `git push origin vX.X.X`
- [ ] Verify GitHub Actions completes (if automated)
- [ ] Test download and install from release

---

## Quick Reference

### Commands

```bash
# Generate signing keys
npx tauri signer generate -w ~/.tauri/translator-desktop.key

# Build with signing (PowerShell)
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~/.tauri/translator-desktop.key -Raw
npm run build:all

# Create release tag
git tag v0.2.0 && git push origin v0.2.0
```

### Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/tauri.conf.json` | Add updater config, set targets |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-updater` |
| `src-tauri/src/lib.rs` | Register updater plugin |
| `src/hooks/useUpdater.ts` | Update check hook |
| `src/components/UpdateNotification.tsx` | Update UI |
| `.github/workflows/release.yml` | CI/CD automation |

---

## Creating Your First Release

Once setup is complete, releasing a new version is simple:

### Automated Release (Recommended)

```powershell
# 1. Update version in tauri.conf.json
#    Change "version": "0.1.0" to "version": "0.2.0"

# 2. Commit and tag
git add -A
git commit -m "chore: release v0.2.0"
git tag v0.2.0
git push origin master --tags

# 3. GitHub Actions builds and creates a draft release
# 4. Go to GitHub > Releases, review the draft, and publish
```

### Manual Release

```powershell
# 1. Set signing environment
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~/.tauri/translator-desktop.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-password"

# 2. Build
npm run build:all

# 3. Find artifacts in src-tauri/target/release/bundle/nsis/
# 4. Upload to GitHub Releases manually
```

### What Users See

When an update is available:
1. App shows a notification in the bottom-right corner
2. User clicks "Install & Restart"
3. Update downloads with progress bar
4. App restarts with new version

The update notification automatically appears 3 seconds after app launch if a new version is available on GitHub.
