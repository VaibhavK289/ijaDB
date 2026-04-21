# The Complete Desktop App Playbook: Next.js + Tauri + Rust
## From Architecture to Public Release — A Deep Comprehensive Guide

> **Purpose**: This is a reusable, production-grade playbook for building hybrid desktop apps with a modern web UI, Rust desktop shell, installable Windows `.exe`, and shareable public release. Every step explains not just the *how* but the *why*.

---

## Table of Contents

| # | Chapter | What You'll Learn |
|---|---|---|
| 1 | System Design | The hybrid architecture model and why each component exists |
| 2 | Tauri IPC Deep Dive | How JS↔Rust communication works internally via serde |
| 3 | Next.js Static Export | Why `output: 'export'` is mandatory and what you lose |
| 4 | Cargo Workspace | Multi-crate project structure and shared types |
| 5 | Environment Strategy | Handling dev/web/desktop API URLs at build time |
| 6 | Axum Backend | Production Rust API with CORS, health checks, deployment |
| 7 | Tauri Configuration | Every `tauri.conf.json` field explained + NSIS packaging |
| 8 | Branding Assets | Icon requirements and why quality affects trust |
| 9 | Code Signing & Trust | SmartScreen reputation, certificates, timestamps |
| 10 | Release Discipline | Build flow, checksums, git tagging, GitHub Releases |
| 11 | Installation QA | Clean machine testing matrix and validation checklist |
| 12 | CI/CD Pipeline | GitHub Actions workflow for automated signed releases |
| 13 | Auto-Updater | Key generation, manifests, signature verification |
| 14 | Troubleshooting | White screen, CORS errors, missing artifacts |
| 15 | Release Checklist | Copy-paste checklist for every future release |
| 16 | Resource Library | Curated links to official docs, tools, community |

---

## Chapter 1: What You Are Building — The System Design

### 1.1 The Hybrid Desktop App Model

You are building a **hybrid desktop application**. This means your app is not a pure native app (written entirely in C++ or Swift), nor is it a pure web app (running only in a browser). It is a combination:

- **A web-based UI** (built with Next.js) provides the visual layer — buttons, charts, layouts, animations.
- **A native desktop shell** (built with Tauri/Rust) wraps that UI in a real Windows application window, giving it access to the file system, system tray, notifications, and native menus.
- **An optional cloud API** (built with Rust Axum) handles shared data, authentication, or heavy computation that should not run on the user's machine.

#### Why This Model Exists

Historically, building a desktop app meant choosing between:

| Approach | Pros | Cons |
|---|---|---|
| **Pure Native** (C++, C#, Swift) | Maximum performance, smallest binary | Slow to develop, platform-specific code, no code reuse with web |
| **Electron** (Chromium + Node.js) | Fast UI development with web tech, huge ecosystem | Ships a 150MB+ Chromium browser with every app, 300MB+ RAM at idle |
| **Tauri** (OS WebView + Rust) | Fast UI development with web tech, tiny binary (3-10MB), low RAM (30-50MB) | Requires Rust knowledge, slight WebView rendering differences across OS versions |

Tauri exists because developers wanted the speed of web-based UI development without the bloat of shipping an entire browser engine. It achieves this by using the **operating system's built-in web rendering engine** (WebView2 on Windows, WKWebView on macOS) instead of bundling Chromium.

### 1.2 The Four Components in Detail

#### Component 1: Frontend App (Next.js)

**What it does**: Renders all the UI — pages, forms, charts, interactions. It is a standard React/Next.js application.

**How it fits**: During development, you run `npm run dev` and see your app in a browser at `localhost:3000`. For the desktop build, Next.js compiles everything into static HTML/CSS/JS files (the `out/` folder). These static files are then embedded directly into the Tauri binary.

**Why Next.js specifically**: You could use Vite, SvelteKit, or plain HTML. Next.js is chosen because:
- It has a mature ecosystem with excellent developer tooling.
- The App Router provides file-based routing which maps cleanly to desktop "pages."
- Static export (`output: 'export'`) produces optimized, pre-rendered HTML.
- You can reuse the same codebase for a web version of your product later.

#### Component 2: Desktop Shell (Tauri)

**What it does**: Creates a native window on Windows/macOS/Linux, loads your static web files into a WebView, and provides a bridge (IPC) for your JavaScript to call Rust functions.

**How it fits**: The `src-tauri/` directory lives inside your frontend project. When you run `tauri build`, it:
1. Runs your frontend build command (e.g., `npm run build`).
2. Takes the output (`out/` folder) and embeds it into a Rust binary.
3. Compiles the Rust code into a native executable.
4. Wraps that executable in an installer (NSIS `.exe` or `.msi`).

**Why Tauri over Electron**:
- **Binary size**: A "Hello World" Tauri app is ~3MB. Electron is ~150MB.
- **RAM usage**: Tauri idles at ~30-50MB. Electron idles at ~150-400MB.
- **Security**: Tauri follows "least privilege" — the frontend has zero system access by default. You must explicitly expose each capability in Rust. Electron gives the frontend broad Node.js API access by default.
- **Startup time**: Tauri apps launch in <0.5 seconds. Electron apps take 2+ seconds to boot Chromium.

#### Component 3: Backend API (Rust Axum)

**What it does**: A standalone web server deployed to the cloud (Render, Fly.io, Railway). It handles:
- Database operations (user data, shared state).
- Authentication (JWT tokens, OAuth flows).
- Heavy computation (ML inference, data processing).

**How it fits**: Your desktop app makes HTTP requests to this API, just like a mobile app or SPA would. The API URL is injected at build time via environment variables.

**Why Axum**:
- Built on `tokio` (Rust's async runtime) — handles thousands of concurrent connections efficiently.
- Built on `tower` middleware — composable layers for logging, CORS, rate limiting, compression.
- Uses `serde` for automatic JSON serialization — the same library Tauri uses for IPC, meaning your types are consistent across the entire stack.
- Compile-time safety — if your API response type changes, both the backend and any shared consumers fail to compile until fixed.

#### Component 4: Shared Models Crate

**What it does**: A Rust library crate that defines data structures (structs, enums) used by both the backend API and the desktop shell.

**Example**:
```rust
// shared/src/lib.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WeatherResponse {
    pub city: String,
    pub temperature_celsius: f64,
    pub humidity_percent: u8,
    pub description: String,
}
```

Both your Axum backend and your Tauri commands import this crate. If you add a field to `WeatherResponse`, both projects will fail to compile until they handle the new field.

**Why this matters**: Without shared types, your backend might return `{ "temp": 22.5 }` while your frontend expects `{ "temperature": 22.5 }`. This mismatch only shows up at runtime. With a shared crate, it's caught at compile time — before any user ever sees a bug.

### 1.3 How the Pieces Connect at Runtime

```
┌─────────────────────────────────────────────────┐
│                 User's Windows PC                │
│                                                  │
│  ┌──────────────────────────────────────────┐    │
│  │         Tauri Desktop App (.exe)         │    │
│  │                                          │    │
│  │  ┌──────────────┐   ┌────────────────┐   │    │
│  │  │  WebView2     │   │  Rust Core     │   │    │
│  │  │  (Next.js UI) │◄─►│  (IPC Bridge)  │   │    │
│  │  │              │   │               │   │    │
│  │  │  invoke()────►│   │◄─serde JSON──│   │    │
│  │  └──────────────┘   └───────┬────────┘   │    │
│  └─────────────────────────────┼────────────┘    │
│                                │ HTTP requests    │
└────────────────────────────────┼─────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │  Cloud API (Axum)      │
                    │  api.yourapp.com       │
                    │                        │
                    │  Routes + DB + Auth    │
                    └────────────────────────┘
```

**Flow for a typical user action** (e.g., "Get Weather"):
1. User clicks a button in the Next.js UI.
2. JS calls `invoke('get_weather', { city: 'London' })`.
3. Tauri serializes the arguments to JSON via `serde`.
4. The IPC bridge passes the JSON to the Rust function marked `#[tauri::command]`.
5. The Rust function deserializes the JSON into a Rust struct.
6. The Rust function makes an HTTP request to the cloud Axum API.
7. The Axum API queries the database, builds a `WeatherResponse`, serializes it to JSON.
8. The response travels back: Axum → HTTP → Tauri Rust → serde → IPC → JavaScript.
9. The Next.js UI updates with the weather data.

---

## Chapter 2: Deep Dive — Tauri's IPC Mechanism

### 2.1 What IPC Actually Is

IPC stands for **Inter-Process Communication**. In Tauri, your app runs two separate processes:

1. **The WebView process**: This is where your HTML/CSS/JS runs. It is essentially a browser tab.
2. **The Rust Core process**: This is the native binary that manages the window, system tray, file access, etc.

These two processes cannot directly share memory. They communicate by sending **messages** — serialized data packets — back and forth through a secure channel.

### 2.2 The `#[tauri::command]` Macro — What It Actually Does

When you write:

```rust
#[tauri::command]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
```

The `#[tauri::command]` macro transforms this into a much larger function that:

1. **Receives a raw JSON payload** from the IPC channel.
2. **Deserializes** the JSON into the function's parameter types using `serde::Deserialize`. If `name` is missing or not a string, it returns an error immediately — your function code never runs.
3. **Executes your function logic** (the `format!` call).
4. **Serializes the return value** back to JSON using `serde::Serialize`.
5. **Sends the JSON response** back through the IPC channel to JavaScript.

This is why every parameter type and return type in a Tauri command must implement `serde::Serialize` and/or `serde::Deserialize`. Without these traits, the data cannot cross the IPC bridge.

### 2.3 The `invoke_handler` — The Allowlist

On the JavaScript side, you can call `invoke('any_string')`. But Tauri will only execute commands that were explicitly registered during app setup:

```rust
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,           // Only these functions
            get_weather,     // are callable from JS
            save_settings,   // Everything else is rejected
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Why this matters for security**: If an attacker somehow injects JavaScript into your WebView (via XSS), they can only call the specific Rust functions you registered. They cannot access the file system, network, or OS unless one of your registered commands explicitly provides that access. This is the "principle of least privilege" in action.

### 2.4 Async Commands and the UI Thread

If a command does heavy work (network request, file I/O, database query), you should mark it `async`:

```rust
#[tauri::command]
async fn fetch_data(url: String) -> Result<String, String> {
    let response = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?;
    response.text()
        .await
        .map_err(|e| e.to_string())
}
```

**Why**: Synchronous commands block the main thread. If the main thread is blocked, the window becomes unresponsive (the "Not Responding" state). Async commands run on a separate thread managed by `tokio`, keeping the UI smooth.

---

## Chapter 3: Frontend — Next.js Static Export in Depth

### 3.1 Why `output: 'export'` Is Non-Negotiable

A standard Next.js app requires a Node.js server running to:
- Execute `getServerSideProps` on each request.
- Handle API routes (`/api/*`).
- Optimize images on-the-fly via `next/image`.
- Perform Server Actions.

**Tauri does not include Node.js.** There is no server running inside your desktop app. The WebView loads static files directly. Therefore, you must tell Next.js to pre-render everything at build time.

```javascript
// next.config.mjs
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
};
export default nextConfig;
```

This changes the build output from `.next/` (server-dependent) to `out/` (pure static files).

### 3.2 What You Lose with Static Export

| Feature | Available? | Alternative |
|---|---|---|
| `getServerSideProps` | ❌ No | Use `useEffect` + client-side fetch, or Tauri `invoke()` |
| API Routes (`/api/*`) | ❌ No | Use your hosted Axum API, or Tauri commands |
| Server Actions | ❌ No | Use Tauri commands for mutations |
| `next/image` optimization | ❌ No | Set `images: { unoptimized: true }` and optimize images manually |
| Dynamic routes with `generateStaticParams` | ✅ Yes | Must enumerate all paths at build time |
| Client-side routing | ✅ Yes | Works normally |
| React state, hooks, context | ✅ Yes | Works normally |
| CSS Modules, Tailwind | ✅ Yes | Works normally |

### 3.3 Image Optimization Gotcha

Next.js's `<Image>` component normally sends images through a server-side optimizer. Without a server, this fails. You must disable it:

```javascript
const nextConfig = {
  output: 'export',
  images: {
    unoptimized: true,  // Skip server-side image optimization
  },
};
```

Without this setting, your build will fail with an error about the Image Optimization API not being available.

### 3.4 Routing in a Static Export

Next.js App Router creates one HTML file per route. For a static export:
- `/` → `out/index.html`
- `/about` → `out/about/index.html` (if using `trailingSlash: true`)
- `/dashboard` → `out/dashboard/index.html`

**The `trailingSlash` option**: When set to `true`, each route generates its own `index.html` inside a folder. This is more compatible with WebView file loading because the WebView can resolve `/about/` to `/about/index.html` automatically.

```javascript
const nextConfig = {
  output: 'export',
  images: { unoptimized: true },
  trailingSlash: true,  // Recommended for Tauri
};
```

### 3.5 The Complete next.config.mjs for Tauri

```javascript
/** @type {import('next').NextConfig} */
const nextConfig = {
  // MANDATORY: Generate static HTML/CSS/JS files
  output: 'export',
  
  // MANDATORY: Disable server-side image optimization
  images: {
    unoptimized: true,
  },
  
  // RECOMMENDED: Better compatibility with WebView routing
  trailingSlash: true,
  
  // OPTIONAL: If you need to adjust asset paths for WebView loading
  // Usually NOT needed with Tauri's built-in asset protocol
  // assetPrefix: './',
};

export default nextConfig;
```

### 3.6 How Tauri Loads These Files

When your desktop app launches, Tauri does NOT use the `file://` protocol (which has security restrictions). Instead, Tauri uses a **custom protocol** (`tauri://localhost` or `https://tauri.localhost`) to serve your static files. This custom protocol:

- Avoids CORS issues that plague `file://` loading.
- Allows proper path resolution for assets.
- Enables Content Security Policy (CSP) enforcement.
- Makes your app behave as if it were served by a real web server.

This is configured in `tauri.conf.json`:

```json
{
  "build": {
    "frontendDist": "../out",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:3000"
  }
}
```

- `frontendDist`: Points to the Next.js static output folder. Tauri embeds these files into the binary.
- `beforeBuildCommand`: Runs before Tauri compiles. This triggers `npm run build` to generate the `out/` folder.
- `devUrl`: During `tauri dev`, Tauri points the WebView to your live dev server for hot reloading.

---

## Chapter 4: Project Structure — The Cargo Workspace

### 4.1 What a Cargo Workspace Is

A Cargo workspace lets you manage multiple related Rust crates (packages) in a single repository. All crates share:
- A single `Cargo.lock` file (consistent dependency versions).
- A single `target/` directory (shared build artifacts, faster compilation).
- Workspace-level dependency declarations (no version drift between crates).

### 4.2 The Recommended Structure

```
my-app/                          # Root of the repository
├── Cargo.toml                   # Workspace manifest (virtual — no code here)
├── Cargo.lock                   # Shared lock file
│
├── frontend/                    # Next.js + Tauri shell
│   ├── package.json             # Node dependencies + scripts
│   ├── next.config.mjs          # Next.js configuration
│   ├── build-desktop.mjs        # Desktop-specific build script
│   ├── src/                     # Next.js app source
│   │   ├── app/                 # App Router pages
│   │   └── components/          # React components
│   ├── public/                  # Static assets
│   ├── out/                     # Generated: static export output
│   └── src-tauri/               # Tauri shell
│       ├── Cargo.toml           # Tauri crate dependencies
│       ├── tauri.conf.json      # Tauri configuration
│       ├── icons/               # App icons (all sizes)
│       └── src/
│           └── main.rs          # Tauri entry point + commands
│
├── backend/                     # Cloud API
│   ├── Cargo.toml               # Backend crate dependencies
│   ├── render.yaml              # Deployment config (Render.com)
│   └── src/
│       └── main.rs              # Axum server + routes
│
├── shared/                      # Shared types library
│   ├── Cargo.toml               # Shared crate dependencies
│   └── src/
│       └── lib.rs               # Shared structs, enums, types
│
└── target/                      # Generated: all build artifacts go here
    └── release/
        └── bundle/
            └── nsis/
                └── MyApp_1.0.0_x64-setup.exe  # The installer
```

### 4.3 The Root Cargo.toml (Virtual Manifest)

```toml
[workspace]
resolver = "2"
members = [
    "frontend/src-tauri",
    "backend",
    "shared",
]

# Define dependency versions once, use everywhere
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
```

**Why `resolver = "2"`**: Rust has two dependency resolution strategies. Version 2 (default for edition 2021+) handles feature unification more intelligently — it won't accidentally enable features in one crate because another crate in the workspace needs them.

### 4.4 Using Workspace Dependencies

In each child crate's `Cargo.toml`:

```toml
# shared/Cargo.toml
[package]
name = "shared"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }  # Uses version from root
```

```toml
# backend/Cargo.toml
[package]
name = "backend"
version = "0.1.0"
edition = "2021"

[dependencies]
shared = { path = "../shared" }     # Local path dependency
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
```

**Why this matters**: If `serde` is version `1.0.197` in the workspace, every crate uses exactly `1.0.197`. No version conflicts, no duplicate compilations, no subtle bugs from version mismatches.

### 4.5 The Critical Detail: Build Artifact Location

In a workspace, ALL build artifacts go to the ROOT `target/` directory, not to each crate's individual directory. This means:

- ❌ **Wrong**: `frontend/src-tauri/target/release/bundle/nsis/`
- ✅ **Correct**: `target/release/bundle/nsis/`

This catches many developers off guard. After running `tauri build` from the `frontend/` directory, the installer is NOT inside `frontend/`. It's at the workspace root's `target/` folder.

---

## Chapter 5: Environment Variable Strategy

### 5.1 The Three Environments Problem

Your app runs in three completely different environments, and each needs a different API URL:

| Environment | When | API URL | Who runs it |
|---|---|---|---|
| **Local Dev** | `npm run dev` + `tauri dev` | `http://localhost:3001` | You, the developer |
| **Web Production** | Deployed to Vercel/Netlify | `https://api.yourapp.com` | Users in a browser |
| **Desktop Production** | Installed `.exe` on user's PC | `https://api.yourapp.com` | Users on Windows |

The danger: If you hardcode `http://localhost:3001` anywhere in your frontend code, the installed desktop app will try to call your local machine — which obviously doesn't exist on the user's computer.

### 5.2 The Solution: Build-Time Environment Injection

Next.js supports environment variables prefixed with `NEXT_PUBLIC_`. These are embedded into the JavaScript bundle at **build time** (not runtime).

**Step 1**: Reference the variable in your frontend code:
```typescript
// src/lib/api.ts
const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

export async function fetchWeather(city: string) {
  const res = await fetch(`${API_BASE}/api/weather?city=${city}`);
  return res.json();
}
```

**Step 2**: Set the variable differently per environment:

For local dev (`.env.local`):
```
NEXT_PUBLIC_API_URL=http://localhost:3001
```

For web production (Vercel dashboard or `.env.production`):
```
NEXT_PUBLIC_API_URL=https://api.yourapp.com
```

For desktop production, you need a **build script**.

### 5.3 The Desktop Build Script (`build-desktop.mjs`)

This script is the bridge between "npm world" and "Tauri world." It sets the correct environment before triggering the build:

```javascript
// build-desktop.mjs
import { execSync } from 'child_process';

// The production API URL for the desktop app
const DESKTOP_API_URL = process.env.NEXT_PUBLIC_API_URL 
  || 'https://api.yourapp.com';

console.log(`Building desktop app with API URL: ${DESKTOP_API_URL}`);

// Set the env variable and run the Tauri build
execSync('npx tauri build', {
  stdio: 'inherit',
  env: {
    ...process.env,
    NEXT_PUBLIC_API_URL: DESKTOP_API_URL,
  },
});
```

**Why a script instead of a `.env` file**: The desktop build is a special build. It might need different settings than your web production build (different CSP, different feature flags, different analytics keys). A script gives you full programmatic control.

### 5.4 Package.json Scripts

```json
{
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "tauri": "tauri",
    "build:desktop": "node build-desktop.mjs"
  }
}
```

**The workflow**:
- `npm run dev` → Local development with hot reload.
- `npm run build` → Web production build (for Vercel/Netlify).
- `npm run build:desktop` → Desktop installer build with correct API URL.

---

## Chapter 6: The Backend — Rust Axum in Production

### 6.1 When to Use a Cloud API vs. Tauri Commands

| Use Case | Use Tauri Commands (Local) | Use Cloud API (Axum) |
|---|---|---|
| Read/write local files | ✅ | ❌ |
| Access system hardware | ✅ | ❌ |
| Offline functionality | ✅ | ❌ |
| Shared database (multi-user) | ❌ | ✅ |
| User authentication | ❌ | ✅ |
| Heavy ML/AI computation | ❌ | ✅ |
| Rate-limited 3rd party APIs | ❌ (exposes API keys) | ✅ (keys stay server-side) |

**The critical rule**: Never put API keys or secrets in the desktop app binary. Anyone can decompile it and extract them. Secrets belong on the server.

### 6.2 Production Axum Server Structure

```rust
// backend/src/main.rs
use axum::{Router, routing::get, Json};
use tower_http::cors::{CorsLayer, Any};
use shared::WeatherResponse;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::init();

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/weather", get(get_weather))
        .layer(build_cors_layer());

    // Bind to the port (Render.com sets PORT env var)
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ALWAYS have a health endpoint — deployment platforms need this
async fn health_check() -> &'static str {
    "OK"
}

async fn get_weather() -> Json<WeatherResponse> {
    Json(WeatherResponse {
        city: "London".into(),
        temperature_celsius: 15.5,
        humidity_percent: 72,
        description: "Partly cloudy".into(),
    })
}

fn build_cors_layer() -> CorsLayer {
    // In production, restrict to your actual origins
    // In development, allow anything
    if cfg!(debug_assertions) {
        CorsLayer::very_permissive()
    } else {
        CorsLayer::new()
            .allow_origin([
                "https://tauri.localhost".parse().unwrap(),
                "https://yourapp.com".parse().unwrap(),
            ])
            .allow_methods(Any)
            .allow_headers(Any)
    }
}
```

### 6.3 Why the Health Endpoint Matters

Every cloud platform (Render, Fly.io, Railway, AWS) periodically pings your service to check if it's alive. Without a `/health` endpoint:
- The platform thinks your service crashed.
- It restarts your container.
- Your users experience downtime.
- Logs fill with "health check failed" errors.

A health endpoint should:
- Return HTTP 200.
- Respond in <100ms.
- Optionally check database connectivity.
- Never require authentication.

### 6.4 CORS — Why It Exists and How to Handle It

**What CORS is**: Cross-Origin Resource Sharing is a browser security feature. When your desktop app (running at `https://tauri.localhost`) makes a request to your API (at `https://api.yourapp.com`), the browser/WebView blocks it by default because they are different "origins."

**How to fix it**: Your Axum server must include CORS headers that explicitly allow your app's origin. The `tower-http` crate provides `CorsLayer` for this.

**The Tauri-specific detail**: Tauri apps use the custom origin `https://tauri.localhost` (Tauri v2) or `tauri://localhost` (Tauri v1). You must include this in your allowed origins list, or your desktop app will silently fail all API requests.

### 6.5 Deployment Config (render.yaml)

```yaml
services:
  - type: web
    name: myapp-api
    runtime: rust
    buildCommand: cargo build --release -p backend
    startCommand: ./target/release/backend
    envVars:
      - key: PORT
        value: 10000
      - key: DATABASE_URL
        fromDatabase:
          name: myapp-db
          property: connectionString
    healthCheckPath: /health
```

---

## Chapter 7: Tauri Configuration & Packaging

### 7.1 The tauri.conf.json File — Every Field Explained

This is the most important configuration file in your desktop project. Here is a production-ready version with explanations:

```jsonc
{
  // The unique identifier — NEVER CHANGE THIS after first release
  "identifier": "com.yourcompany.yourapp",

  // Human-readable name shown in window title, Start Menu, etc.
  "productName": "Your App Name",

  // Must match the version in src-tauri/Cargo.toml
  "version": "1.0.0",

  "build": {
    // Command to run before building the Tauri binary
    // This generates the static frontend files
    "beforeBuildCommand": "npm run build",
    
    // Command to run before starting dev mode
    "beforeDevCommand": "npm run dev",
    
    // During dev, point to the live dev server
    "devUrl": "http://localhost:3000",
    
    // During production build, embed files from this directory
    "frontendDist": "../out"
  },

  "app": {
    "windows": [
      {
        "title": "Your App Name",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false,
        // Center the window on launch
        "center": true
      }
    ],
    "security": {
      // Content Security Policy — controls what the WebView can load
      "csp": "default-src 'self'; connect-src 'self' https://api.yourapp.com; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' https://fonts.gstatic.com"
    }
  },

  "bundle": {
    // Whether to create installer bundles
    "active": true,
    
    // Which installer format to generate
    "targets": ["nsis"],
    
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],

    "windows": {
      // For code signing (leave empty if unsigned)
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com",
      
      "nsis": {
        // Show a license agreement during install
        "license": null,
        // Allow user to pick install directory
        "installerIcon": "icons/icon.ico",
        // Install for current user only (no admin needed)
        "installMode": "currentUser"
      }
    }
  }
}
```

### 7.2 The Identifier — Why It's Permanent

The `identifier` field (e.g., `com.yourcompany.yourapp`) is used by:
- **Windows Registry**: To store uninstall information.
- **App Data paths**: To create `%APPDATA%/com.yourcompany.yourapp/`.
- **Tauri Updater**: To match updates to the correct installed app.
- **OS shortcuts**: To link Start Menu entries to the correct executable.

If you change this identifier after users have installed your app:
- Their existing installation becomes "orphaned" — Windows doesn't know it's the same app.
- They'll have two entries in "Add/Remove Programs."
- App data from the old identifier is abandoned.
- Auto-update breaks because the updater can't find the old installation.

**Rule**: Pick your identifier before your first public release. Never change it.

### 7.3 NSIS vs MSI — Which Installer Format

| Feature | NSIS (.exe) | MSI |
|---|---|---|
| **User experience** | Familiar "Setup Wizard" | Standard Windows Installer |
| **Best for** | Consumer apps | Enterprise/IT-managed apps |
| **Admin required** | Optional (can install per-user) | Usually requires admin |
| **Customization** | Highly customizable UI | Limited UI customization |
| **Group Policy** | No | Yes (IT can deploy via GPO) |
| **Repair/Modify** | No | Built-in repair functionality |

**Recommendation**: Start with NSIS. It's simpler, produces a single `.exe`, and doesn't require admin privileges when using `installMode: "currentUser"`.

### 7.4 What NSIS Does Behind the Scenes

When a user runs your `MyApp_1.0.0_x64-setup.exe`:

1. **Extraction**: NSIS decompresses all bundled files to a temp directory.
2. **UI Wizard**: Shows the license agreement (if configured), installation directory picker, and progress bar.
3. **File Copy**: Copies your app binary and resources to the chosen directory (default: `C:\Users\<user>\AppData\Local\com.yourcompany.yourapp\`).
4. **Registry Entries**: Writes uninstall information to `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{identifier}`. This is what makes your app appear in "Add/Remove Programs."
5. **Shortcuts**: Creates Start Menu and (optionally) Desktop shortcuts.
6. **Cleanup**: Removes temp files.

**On uninstall**, the reverse happens: shortcuts are deleted, files are removed, and registry entries are cleaned up.

---

## Chapter 8: Branding Assets

### 8.1 What You Need and Why

| Asset | Size | Format | Used For |
|---|---|---|---|
| Master icon | 1024x1024 | PNG | Source for all other sizes |
| icon.ico | Multi-size | ICO | Windows executable icon, taskbar, Alt+Tab |
| icon.icns | Multi-size | ICNS | macOS Dock and Finder |
| 32x32.png | 32x32 | PNG | Small taskbar icon, Windows context menus |
| 128x128.png | 128x128 | PNG | Windows tile, file manager thumbnail |
| 128x128@2x.png | 256x256 | PNG | High-DPI displays |
| StoreLogo.png | 50x50 | PNG | Microsoft Store listing (if applicable) |

### 8.2 Generating Icons from a Single Source

Tauri provides a CLI command that generates all required icon sizes from a single 1024x1024 PNG:

```bash
npx tauri icon path/to/your-master-icon.png
```

This creates the entire `icons/` directory with all required formats and sizes.

### 8.3 Why Icon Quality Matters for Trust

This seems cosmetic, but it directly affects security perception:

1. **User Trust**: A professional icon signals a legitimate product. Default/placeholder icons signal "this might be malware."
2. **Antivirus Heuristics**: Some antivirus engines use icon quality as a heuristic signal. Apps with default Tauri icons or no icons are more likely to be flagged for additional scrutiny.
3. **SmartScreen**: While SmartScreen doesn't directly check icon quality, users are more likely to click "Run Anyway" on a professional-looking app than on one with a generic icon.

### 8.4 Content Security Policy (CSP)

The CSP in your `tauri.conf.json` controls what your WebView can load. A properly configured CSP:

```
default-src 'self';
connect-src 'self' https://api.yourapp.com;
style-src 'self' 'unsafe-inline';
img-src 'self' data: https:;
font-src 'self' https://fonts.gstatic.com;
script-src 'self';
```

**What each directive means**:
- `default-src 'self'` — Only load resources from the app itself by default.
- `connect-src` — Allow API calls to your backend domain.
- `style-src 'unsafe-inline'` — Allow inline styles (needed by many CSS-in-JS solutions).
- `img-src 'self' data: https:` — Allow images from the app, data URIs, and HTTPS sources.
- `font-src` — Allow Google Fonts.
- `script-src 'self'` — Only execute scripts bundled with the app (blocks XSS injection).

---

## Chapter 9: Code Signing — The Trust Layer

### 9.1 What Code Signing Actually Is

Code signing is the process of applying a **digital signature** to your executable file. It uses the same cryptographic principles as HTTPS:

1. You obtain a **certificate** from a trusted Certificate Authority (CA) — like DigiCert, Sectigo, or Microsoft's Trusted Signing service.
2. At build time, you use your **private key** to create a cryptographic hash of your entire binary, then encrypt that hash with your key.
3. The encrypted hash (the "signature") is embedded into the `.exe` file.
4. When a user runs the app, Windows uses the CA's **public key** to decrypt the signature and verify that the hash matches the file's actual contents.

If the file has been modified after signing (by malware, corruption, or tampering), the hash won't match, and Windows will warn the user.

### 9.2 Why Timestamps Are Critical

When you sign a binary, the signature is only valid as long as your certificate is valid. Certificates expire (typically after 1-3 years).

**Without a timestamp**: If your cert expires on 2027-01-01, every binary you signed becomes "invalid" after that date. Users will see warnings even though the software hasn't changed.

**With a timestamp**: The timestamp proves "this file was signed on 2026-06-15, when the cert was valid." The signature remains valid forever, regardless of when the cert expires.

**How to include a timestamp**: The signing tool contacts a timestamp server during the signing operation:
```powershell
signtool sign /tr http://timestamp.digicert.com /td sha256 /fd sha256 /a "MyApp_setup.exe"
```

- `/tr` — Timestamp server URL (use RFC 3161 timestamp)
- `/td sha256` — Timestamp digest algorithm
- `/fd sha256` — File digest algorithm
- `/a` — Auto-select the best certificate from the cert store

### 9.3 Windows SmartScreen — How Reputation Works

SmartScreen is Microsoft's cloud-based reputation system. Here's exactly how it evaluates your app:

**Step 1 — File Download**: When a user downloads your `.exe`, Windows marks it with an "Alternate Data Stream" called `Zone.Identifier`, tagging it as "downloaded from the internet."

**Step 2 — First Launch**: When the user double-clicks the `.exe`, SmartScreen sends the following to Microsoft's servers:
- The file's SHA-256 hash
- The signing certificate details (if signed)
- The file's size
- The download URL

**Step 3 — Reputation Lookup**: Microsoft checks its database:
- **Known Good**: The hash has been seen thousands of times across many different machines without malware reports. → No warning.
- **Known Bad**: The hash matches known malware. → Blocked.
- **Unknown**: The hash has never been seen, or has been seen too few times. → Warning: "Windows protected your PC."

**Step 4 — Building Reputation**: Every time a user runs your app without it being flagged as malware, its reputation improves. After enough "clean" runs across enough unique machines (Microsoft doesn't publish the exact threshold, but estimates suggest thousands of runs), the warning disappears.

**Key insight**: Reputation is tracked **per file hash**. Every time you release a new version, the new binary has a new hash and starts with zero reputation. Signing helps because the certificate itself accumulates reputation over time, carrying some trust to new hashes signed by the same cert.

### 9.4 Certificate Types

| Type | Cost | SmartScreen Effect | Best For |
|---|---|---|---|
| **No certificate** | Free | Always shows warning | Internal tools only |
| **OV (Organization Validation)** | ~$100-400/year | Shows publisher name; warning disappears over time | Small teams, indie developers |
| **EV (Extended Validation)** | ~$300-700/year | Historically instant reputation; now similar to OV but builds faster | Companies distributing widely |
| **Azure Trusted Signing** | ~$10/month | Same as OV; cloud-managed | Modern alternative, no hardware tokens |

### 9.5 Azure Trusted Signing — The Modern Approach

As of 2024, Microsoft offers **Trusted Signing** as a cloud-based alternative to traditional certificates:

1. Create an Azure account and subscription.
2. Create a Trusted Signing account in the Azure portal.
3. Verify your identity via Microsoft Entra Verified ID (government ID + selfie).
4. Configure signing in your CI/CD pipeline using the Azure CLI.

**Advantages over traditional certs**:
- No physical hardware tokens to manage.
- No `.pfx` files to secure.
- Integrates directly with GitHub Actions.
- Predictable, low-cost pricing.

### 9.6 Tauri Configuration for Code Signing

In `tauri.conf.json`:
```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "YOUR_CERT_THUMBPRINT_HERE",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

If `certificateThumbprint` is set but the cert isn't found in the Windows cert store, the build will fail. For CI builds, you import the `.pfx` into the cert store before building (see Chapter 12).

---

## Chapter 10: Release Discipline

### 10.1 The Build Flow — Exact Steps

Open your terminal at the `frontend/` directory and run:

```bash
# Step 1: Install/update dependencies
npm install

# Step 2: Set the production API URL
$env:NEXT_PUBLIC_API_URL = "https://api.yourapp.com"

# Step 3: Build the installer
npx tauri build
```

**What happens internally**:
1. Tauri runs `beforeBuildCommand` → `npm run build` → Next.js generates `out/` folder.
2. Tauri compiles the Rust code in `src-tauri/src/main.rs`.
3. Tauri embeds the `out/` folder into the Rust binary.
4. Tauri runs NSIS to wrap the binary in an installer.
5. Output: `<workspace-root>/target/release/bundle/nsis/YourApp_1.0.0_x64-setup.exe`.

### 10.2 Generating SHA-256 Checksum

Always generate and publish a checksum. This allows users and IT administrators to verify the download wasn't corrupted or tampered with.

**PowerShell**:
```powershell
Get-FileHash "target\release\bundle\nsis\YourApp_1.0.0_x64-setup.exe" -Algorithm SHA256 | Format-List
```

**Output**:
```
Algorithm : SHA256
Hash      : A1B2C3D4E5F6...
Path      : D:\myapp\target\release\bundle\nsis\YourApp_1.0.0_x64-setup.exe
```

Publish this hash in your release notes. Users can verify their download by running the same command and comparing hashes.

### 10.3 Git Tagging Workflow

```bash
# Ensure your working directory is clean
git status

# Commit all release changes
git add .
git commit -m "release: v1.0.0"

# Create an annotated tag
git tag -a v1.0.0 -m "Release v1.0.0 - Initial public release"

# Push the commit and tag
git push origin main
git push origin v1.0.0
```

**Why annotated tags (`-a`) instead of lightweight tags**:
- Annotated tags store the tagger's name, email, and date.
- They can include a message describing the release.
- GitHub/GitLab can trigger CI workflows on annotated tags but not always on lightweight tags.
- `git describe` works more reliably with annotated tags.

**Critical rule**: Always build the release binary from a tagged commit. Never build from uncommitted code. If you discover a bug after tagging, create a new tag (e.g., `v1.0.1`), don't reuse the old one.

### 10.4 GitHub Release Process

1. Go to your repository → Releases → "Draft a new release."
2. Select the tag you just pushed (e.g., `v1.0.0`).
3. Write release notes including:
   - What's new / What changed
   - Known issues
   - System requirements (Windows 10 or later)
   - SHA-256 checksum
   - Installation instructions
4. Upload the installer `.exe` as a release asset.
5. Publish the release.

---

## Chapter 11: Installation QA Checklist

### 11.1 Why Clean Machine Testing Is Mandatory

Your development machine has:
- Node.js installed
- Rust toolchain installed
- Visual C++ redistributables installed
- WebView2 runtime installed (or a recent Windows version)
- Environment variables set
- Previous versions of your app in AppData

A clean machine has NONE of this. If your app works on your dev machine but crashes on a clean install, every user will experience the crash.

### 11.2 The Test Matrix

Test on at minimum:

| Scenario | What You're Validating |
|---|---|
| **Fresh Windows 10** | Minimum OS version compatibility |
| **Fresh Windows 11** | Current OS compatibility |
| **Non-admin user** | App works without elevated privileges |
| **Install** | Installer runs without errors |
| **Launch** | App opens, UI renders, no blank screen |
| **Core workflow** | Primary feature works end-to-end |
| **Network features** | API calls succeed (not hitting localhost) |
| **Close and reopen** | App state persists correctly |
| **Uninstall** | App is removed cleanly from Add/Remove Programs |
| **Reinstall** | Fresh install works after previous uninstall |
| **Upgrade** | Install new version over old version without data loss |

### 11.3 What to Check After Install

- [ ] Start Menu shortcut exists and launches the app.
- [ ] Desktop shortcut works (if created).
- [ ] App data folder created at `%APPDATA%/com.yourcompany.yourapp/`.
- [ ] No leftover files after uninstall.
- [ ] No orphaned registry entries after uninstall.
- [ ] App window renders properly (no white screen, no missing assets).
- [ ] Network requests reach the production API (check browser DevTools via `Ctrl+Shift+I` in the Tauri window).

---

## Chapter 12: CI/CD — Automated Release Pipeline

### 12.1 The Target Workflow

```
Developer pushes a version tag (v1.0.0)
         │
         ▼
GitHub Actions triggers
         │
         ▼
Install Node.js + Rust + Tauri prerequisites
         │
         ▼
Build the frontend (Next.js static export)
         │
         ▼
Build the Tauri installer
         │
         ▼
Sign the installer (using cert from GitHub Secrets)
         │
         ▼
Generate SHA-256 checksum
         │
         ▼
Create GitHub Release + upload assets
         │
         ▼
Done — users can download the new version
```

### 12.2 GitHub Actions Workflow

```yaml
# .github/workflows/release.yml
name: Release Desktop App

on:
  push:
    tags:
      - 'v*'  # Triggers on any tag starting with 'v'

jobs:
  build-windows:
    runs-on: windows-latest
    
    permissions:
      contents: write  # Required to create releases
    
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: frontend/package-lock.json

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache Rust dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install frontend dependencies
        working-directory: frontend
        run: npm ci

      # Import code-signing certificate (if you have one)
      - name: Import Windows Certificate
        if: env.WINDOWS_CERTIFICATE != ''
        env:
          WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        shell: powershell
        run: |
          $pfxPath = Join-Path $env:TEMP "certificate.pfx"
          [IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
          $password = ConvertTo-SecureString $env:WINDOWS_CERTIFICATE_PASSWORD -Force -AsPlainText
          Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\CurrentUser\My -Password $password

      - name: Build Tauri App
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          NEXT_PUBLIC_API_URL: https://api.yourapp.com
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'App ${{ github.ref_name }}'
          releaseBody: 'See the assets to download and install.'
          releaseDraft: true
          prerelease: false
          projectPath: frontend
```

### 12.3 Setting Up GitHub Secrets

Go to your repository → Settings → Secrets and variables → Actions → New repository secret:

| Secret Name | Value | Purpose |
|---|---|---|
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` file | Code signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | The PFX password | Decrypting the certificate |

**To Base64-encode your `.pfx` file**:
```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("path\to\your\cert.pfx")) | Set-Clipboard
```
This copies the Base64 string to your clipboard — paste it into the GitHub Secret.

---

## Chapter 13: Tauri Auto-Updater

### 13.1 How It Works End-to-End

The Tauri updater uses **asymmetric key cryptography** (separate public and private keys) to ensure updates are authentic:

**Setup (once)**:
1. Generate a key pair: `npx tauri signer generate -w ~/.tauri/myapp.key`
2. This creates `myapp.key` (private — NEVER share) and `myapp.key.pub` (public — embed in app).
3. Add the public key to `tauri.conf.json`.

**Build (each release)**:
1. Set `TAURI_SIGNING_PRIVATE_KEY` env var to your private key content.
2. Run `tauri build`.
3. Tauri creates both the installer AND a `.sig` file (the signature).

**Distribution**:
1. Upload the installer and `.sig` to GitHub Releases.
2. Host a `latest.json` manifest (or use GitHub Releases directly as the endpoint).

**Client check**:
1. App launches → checks the update endpoint.
2. Endpoint returns `latest.json` with version, download URL, and signature.
3. App compares versions — if newer, downloads the update.
4. App verifies the downloaded file against the signature using the embedded public key.
5. If verification passes, installs the update and restarts.

### 13.2 The latest.json Manifest

```json
{
  "version": "1.1.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2026-06-15T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "url": "https://github.com/you/app/releases/download/v1.1.0/MyApp_1.1.0_x64-setup.nsis.zip",
      "signature": "CONTENTS_OF_THE_SIG_FILE_HERE"
    }
  }
}
```

### 13.3 Security Implications

- If you **lose your private key**, you cannot sign updates for existing users. They'll need to manually download and reinstall.
- If your private key is **compromised**, an attacker could sign malicious updates. Rotate the key immediately and release a new version with a new public key.
- **Updater signing is separate from OS code signing.** You need BOTH: OS code signing prevents "unknown publisher" warnings; updater signing prevents malicious update injection.

---

## Chapter 14: Troubleshooting Common Pitfalls

### 14.1 White Screen / Blank Window

**Symptom**: The Tauri window opens but shows nothing — just a white or blank screen.

**Causes and Fixes**:
1. **Frontend build failed silently**: Check that `out/index.html` actually exists. Run `npm run build` manually and check for errors.
2. **Wrong `frontendDist` path**: Verify `tauri.conf.json` → `build.frontendDist` points to the correct relative path (e.g., `../out`).
3. **JavaScript error**: Open DevTools in the Tauri window (`Ctrl+Shift+I` or right-click → Inspect). Check the Console tab for errors.
4. **Missing `'use client'` directive**: If you're using Tauri's JS APIs in a component, it must be a Client Component. Add `'use client'` at the top of the file.

### 14.2 API Requests Fail in Production

**Symptom**: Everything works in `tauri dev` but API calls fail in the built `.exe`.

**Causes and Fixes**:
1. **Hardcoded localhost**: Search your codebase for `localhost` or `127.0.0.1`. Replace with the env variable.
2. **Missing env injection**: The build script didn't set `NEXT_PUBLIC_API_URL`. Verify by checking the built JS files for the URL string.
3. **CORS blocking**: Your Axum server doesn't allow the `https://tauri.localhost` origin. Add it to `CorsLayer`.
4. **CSP blocking**: Your `tauri.conf.json` CSP doesn't include your API domain in `connect-src`.

### 14.3 Installer Output Not Found

**Symptom**: `tauri build` succeeds but you can't find the `.exe` file.

**Fix**: In a Cargo workspace, artifacts go to the ROOT `target/` directory:
```
✅ <workspace-root>/target/release/bundle/nsis/
❌ frontend/src-tauri/target/release/bundle/nsis/
```

### 14.4 Version Mismatch

**Symptom**: The installed app shows the wrong version, or updates don't work.

**Fix**: Version must be synchronized in THREE places:
1. `frontend/src-tauri/Cargo.toml` → `version = "1.0.0"`
2. `frontend/src-tauri/tauri.conf.json` → `"version": "1.0.0"`
3. `frontend/package.json` → `"version": "1.0.0"` (optional but recommended)

---

## Chapter 15: Complete Release Checklist

Copy this for every release:

```markdown
## Pre-Release
- [ ] Version bumped in Cargo.toml, tauri.conf.json, and package.json
- [ ] Release notes drafted
- [ ] All tests passing
- [ ] Branding assets finalized (icons, splash screen)

## Build
- [ ] Backend deployed and reachable at production URL
- [ ] `NEXT_PUBLIC_API_URL` set to production URL
- [ ] `npx tauri build` completed without errors
- [ ] Installer file exists at target/release/bundle/nsis/
- [ ] SHA-256 hash generated and recorded

## Validation
- [ ] Clean machine install test passed (Windows 10)
- [ ] Clean machine install test passed (Windows 11)
- [ ] Core user workflow tested end-to-end
- [ ] Network-dependent features working
- [ ] Uninstall and reinstall tested
- [ ] Upgrade from previous version tested (if applicable)

## Trust
- [ ] Installer is code-signed (or documented as unsigned)
- [ ] Signature verified with signtool verify
- [ ] Timestamp is present in signature

## Publish
- [ ] Git commit clean (no uncommitted changes)
- [ ] Git tag created and pushed (e.g., v1.0.0)
- [ ] GitHub Release created
- [ ] Installer uploaded as release asset
- [ ] SHA-256 hash published in release notes
- [ ] Download link tested in incognito browser

## Post-Release
- [ ] Monitor error reporting / crash logs
- [ ] Track user install issues (GitHub Issues, support email)
- [ ] Hotfix path documented if critical bug found
```

---

## Chapter 16: Curated Resource Library

### Official Documentation
| Resource | URL | What You'll Learn |
|---|---|---|
| Tauri v2 Docs | https://v2.tauri.app/ | Complete Tauri v2 API, configuration, plugins |
| Tauri v2 Commands Guide | https://v2.tauri.app/develop/calling-rust/ | Writing and calling IPC commands |
| Next.js Static Exports | https://nextjs.org/docs/app/building-your-application/deploying/static-exports | Configuring `output: 'export'` correctly |
| Axum Framework | https://github.com/tokio-rs/axum | Axum API design, routing, middleware |
| Serde Documentation | https://serde.rs/ | Serialization/deserialization framework |
| The Rust Book | https://doc.rust-lang.org/book/ | Rust fundamentals (ownership, structs, traits) |
| Cargo Workspaces | https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html | Multi-crate project structure |

### Security & Signing
| Resource | URL | What You'll Learn |
|---|---|---|
| Microsoft Trusted Signing | https://learn.microsoft.com/en-us/azure/trusted-signing/ | Cloud-based code signing setup |
| Tauri Code Signing Guide | https://v2.tauri.app/distribute/sign/windows/ | Configuring Tauri for Windows code signing |
| SmartScreen Overview | https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/ | How SmartScreen evaluates files |
| SignTool Documentation | https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool | Windows SDK signing tool reference |

### CI/CD & Automation
| Resource | URL | What You'll Learn |
|---|---|---|
| tauri-apps/tauri-action | https://github.com/tauri-apps/tauri-action | GitHub Actions for Tauri builds |
| GitHub Actions Docs | https://docs.github.com/en/actions | Workflow syntax, secrets, permissions |

### Deep Dives & Community
| Resource | URL | What You'll Learn |
|---|---|---|
| Awesome Tauri | https://github.com/tauri-apps/awesome-tauri | Plugins, examples, community projects |
| Tauri Discord | https://discord.com/invite/tauri | Real-time help from the Tauri community |
| Tauri vs Electron (LogRocket) | https://blog.logrocket.com/tauri-electron-comparison-migration-guide/ | Detailed comparison article |
| NSIS Documentation | https://nsis.sourceforge.io/Main_Page | Installer scripting reference |
| tower-http (CORS) | https://docs.rs/tower-http/latest/tower_http/cors/ | Configuring CORS for Axum |

### Deployment Platforms
| Platform | URL | Best For |
|---|---|---|
| Render | https://render.com | Simple Rust backend deployment |
| Fly.io | https://fly.io | Edge-deployed Rust services |
| Railway | https://railway.app | Quick Rust/Node deployments |
| Shuttle | https://shuttle.dev | Rust-native deployment platform |
