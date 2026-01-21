# Translator Desktop

A Tauri 2.0 desktop application for translation, built with React 18, TypeScript, and Tailwind CSS.

## Prerequisites

- Node.js 18+
- Rust 1.70+
- Windows: Visual Studio Build Tools with C++ workload

## Installation

```bash
# Install frontend dependencies
npm install

# First run will automatically install Rust dependencies
```

## Development

```bash
# Run the desktop app in development mode
npm run tauri dev

# Run only the frontend (Vite)
npm run dev

# Build for production
npm run tauri build
```

## Project Structure

```
translator-desktop/
├── src/                    # React frontend source
│   ├── App.tsx            # Main React component
│   ├── main.tsx           # React entry point
│   └── index.css          # Tailwind CSS
├── src-tauri/             # Tauri backend (Rust)
│   ├── src/
│   │   ├── main.rs        # Rust entry point
│   │   └── lib.rs         # Tauri commands
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── public/                # Static assets
├── package.json           # Frontend dependencies
└── vite.config.ts         # Vite configuration
```

## Tech Stack

- **Frontend**: React 18, TypeScript, Vite, Tailwind CSS
- **Backend**: Tauri 2.0 (Rust)
- **Build**: Vite for frontend, Cargo for Rust

## Ports

- Frontend dev server: http://localhost:1420
