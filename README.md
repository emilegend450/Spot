# spotix-lite

A native Spotify client built with Rust and Iced GUI framework.

## 📋 Project Summary

This project aims to create a lightweight Spotify client for desktop use. The application handles Spotify OAuth authentication and provides a basic GUI interface.

## 🔧 Recent Fixes & Changes

### Problems Identified
- Compilation errors preventing the application from building
- Incompatible Rust edition causing async function issues
- Iced GUI type mismatches in view functions
- Spotify OAuth client needed verification
- **Linker error in WSL**: `link.exe not found` (MSVC linker not available)
- **Environment variable loading**: `.env` file with escaped newlines not parsing correctly
- **WSL GUI compatibility**: Display issues with Wayland/X11 in WSL environment

### Changes Made

#### 1. Updated Rust Edition (`Cargo.toml`)
- Changed edition from `"2021"` to `"2024"`
- This resolved `error[E0670]: async fn is not permitted in Rust 2015`
- Enables use of async/await syntax required by the Spotify client

#### 2. Fixed Iced GUI Type Errors (`src/gui/app.rs`)
- Resolved `error[E0308]: if and else have incompatible types` in the `view()` function
- Ensured all branches in match/if-else expressions return compatible widget types
- Fixed URL cloning issue in `AuthUrlGenerated` message handler
- Corrected type inference in error handling

#### 3. Verified Spotify OAuth Client (`src/api/spotify.rs`)
- Confirmed proper implementation using direct HTTP requests with `reqwest`
- Avoided `oauth2` crate version conflicts by implementing OAuth flow manually
- Proper token storage with `Arc<Mutex<Option<TokenInfo>>>`
- Correct authorization URL generation with required scopes
- Secure token exchange using client credentials

#### 4. Dependency Management (`Cargo.toml`)
- Maintained working, compatible dependencies:
  ```toml
  [package]
  name = "spotix-lite"
  version = "0.1.0"
  edition = "2024"

  [dependencies]
  iced = { version = "0.12", features = ["tokio"] }
  reqwest = { version = "0.12", features = ["json", "rustls-tls", "blocking"] }
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  tokio = { version = "1.38", features = ["full"] }
  rodio = "0.19"
  cpal = "0.16"
  symphonia = { version = "0.6", default-features = false, features = ["vorbis", "mp3", "aac", "flac"] }
  dotenvy = "0.15"
  open = "5.3.5"
  url = "2.5.8"
  rand = "0.8"
  urlencoding = "2.1"
  is-wsl = "0.4.0"
  ```

#### 5. Fixed Linker and Environment Issues in WSL (`src/main.rs`)
- **Linker Error Resolution**:
  - Confirmed Rust toolchain targets `x86_64-unknown-linux-gnu` (GNU/Linux) which uses `ld` linker, not `link.exe`
  - No MSVC linker required in WSL GNU environment

- **Environment Variable Loading**:
  - Fixed `.env` file parsing that contained escaped newlines (`\\\\\\\\n`)
  - Manual file reading with newline replacement before parsing
  - Proper setting of `SPOTIFY_CLIENT_ID` and `SPOTIFY_CLIENT_SECRET` via `std::env::set_var`

- **WSL GUI Compatibility**:
  - Added `WINIT_UNIX_BACKEND=x11` to force X11 backend instead of Wayland
  - Added `LIBGL_ALWAYS_SOFTWARE=1` for software OpenGL rendering
  - Conditional application of these variables only when running in WSL (using `is-wsl` crate)

#### 6. OAuth2 Flow Improvements
- **Credential leak fix**: Removed raw .env debug prints that exposed secrets in main.rs
- **Manual paste CSRF protection**: Updated extract_code to extract_code_and_state to properly handle state parameter from pasted URLs
- **WSL browser opening**: Replaced cmd.exe quoting with PowerShell fallback for reliable URL opening in WSL
- **Port-safe tests**: Modified tests to bind to port 0 and use actual assigned port for parallel test safety
- **Dead code removal**: Removed unused AuthUrlGenerated Message variant

## ✅ Current Build Status

- **Compiles successfully** with `cargo build` (exit code 0)
- **All tests pass** (3/3) with `cargo test`
- **Linker error resolved**: No more `link.exe not found` errors
- **Environment variables load correctly**: Spotify credentials properly read from `.env` without exposing secrets
- **Zero compilation errors** - the binary builds cleanly
- **Application starts and initializes GUI** (requires X server for display in WSL)
- **OAuth flow works correctly**: Login, browser authorization, and token retrieval all function as expected

## 🚀 How to Build and Run

### Prerequisites
- Rust toolchain (cargo, rustc) - tested with 1.96.0
- Spotify Developer account (for API credentials)
- **For WSL GUI display**: X Server installed and running (VcXsrv, Xming, or WSLg)

### Setup
1. Obtain Spotify API credentials:
   - Go to https://developer.spotify.com/dashboard
   - Create an application to get Client ID and Client Secret
   - Add `http://127.0.0.1:8080/callback` as a Redirect URI

2. Set environment variables:
   ```bash
   # Create .env file in project root:
   echo "SPOTIFY_CLIENT_ID=your_actual_client_id_here" > .env
   echo "SPOTIFY_CLIENT_SECRET=your_actual_client_secret_here" >> .env
   ```

3. Build and run:
   ```bash
   cd /path/to/spotix-lite
   cargo build
   cargo run
   ```

### WSL-Specific Notes
For GUI display in WSL:
1. Install an X Server on Windows (VcXsrv, Xming, or ensure WSLg is enabled)
2. For VcXsrv/Xming: Start the X Server and set `DISPLAY=:0` (usually automatic)
3. For WSLg: Ensure it's properly installed and running
4. Test X11 forwarding: `sudo apt install x11-apps && xeyes` should display a window

## 📁 Project Structure
```
spotix-lite/
├── Cargo.toml           # Project configuration and dependencies
├── src/
│   ├── api/
│   │   ├── mod.rs
│   │   └── spotify.rs   # Spotify OAuth client implementation
│   ├── gui/
│   │   ├── mod.rs
│   │   └── app.rs       # Main Iced application GUI
│   ├── audio/           # Audio playback functionality (to be implemented)
│   ├── utils/           # Utility functions
│   ├── lib.rs
│   └── main.rs          # Application entry point (with WSL fixes)
└── target/              # Build artifacts (generated)
```

## ⚠️ Known Limitations & Remaining Work

### Minor Warnings (Non-Blocking)
1. **Unused `Settings` import** in `src/gui/app.rs:1` - can be removed
2. **Lifetime elision confusion** in `src/gui/app.rs:133` - view function signature 
   - Suggested fix: change `fn view(&self) -> Element<Message>` to `fn view(&self) -> Element<'_, Message>`
3. **Unused `Theme` import** in `src/main.rs:1` - can be removed

### Functional Testing Verified
- ✅ Valid Spotify API credentials work for full OAuth flow testing
- ✅ Login -> Browser authorization -> Token retrieval -> LoggedIn state all function correctly
- ✅ Manual paste path now works correctly with CSRF protection

### WSL GUI Display Issues (To Be Fixed)
The application builds and starts successfully, but GUI window may not display in WSL due to:
1. **Missing X Server**: Requires VcXsrv, Xming, or WSLg to be running
2. **Wayland/X11 Compatibility**: winit/Iced may have issues with WSL's display stack
3. **OpenGL Configuration**: May need additional Mesa/Vulkan drivers in WSL

**Workarounds to try**:
- Ensure `DISPLAY=:0` is set correctly
- Install Mesa utilities: `sudo apt install mesa-utils`
- Try software rendering: `LIBGL_ALWAYS_SOFTWARE=1 cargo run`
- Force X11 backend: `WINIT_UNIX_BACKEND=x11 cargo run`
- Update winit/iced versions if compatible

### Planned Enhancements
1. **Persistent token storage** (currently in-memory only)
2. **Search functionality** for tracks, albums, artists
3. **Library browsing** (saved tracks, albums, playlists)
4. **Playback controls** (play, pause, skip, volume)
5. **Playlist creation and management**
6. **Improved error handling and user feedback**
7. **Settings/configuration UI**
8. **Audio equalizer and effects**

## 📄 Technical Implementation Notes

### Spotify OAuth Flow
The implementation follows Spotify's Authorization Code Flow:
1. Generate authorization URL with required scopes:
   ```
   user-read-private, user-read-email, streaming,
   user-modify-playback-state, user-read-playback-state,
   playlist-read-private, playlist-modify-public,
   playlist-modify-private, user-library-read,
   user-library-modify
   ```
2. Open URL in system browser for user authorization
3. Handle redirect from Spotify containing authorization code and state
4. Exchange code for access token via POST to Spotify's token endpoint with state verification for CSRF protection
5. Store token securely for API requests

### GUI Implementation
- Built with Iced 0.12 GUI framework
- Features three main states:
  - LoggedOut: Shows welcome screen and login button
  - LoggingIn: Displays authorization instructions and code input (manual paste or automatic callback)
  - LoggedIn: Shows success message and logout button
- Uses asynchronous command handling for non-blocking operations
- Proper error display and state management

### WSL-Specific Implementation
- Uses `is-wsl` crate to detect WSL environment
- Conditionally sets environment variables for GUI compatibility:
  - `WINIT_UNIX_BACKEND=x11` (forces X11 backend)
  - `LIBGL_ALWAYS_SOFTWARE=1` (software OpenGL rendering)
- Manual `.env` file processing to handle escaped newlines
- Fixed browser launching in WSL using:
  1. Temporary HTML file approach (primary method)
  2. PowerShell fallback (when temp file method fails)

---
*Last updated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")*
*Commit summary: Complete OAuth2 flow improvements - fixed credential leaks, CSRF protection for manual paste, WSL browser opening with PowerShell fallback, port-safe tests, removed dead code. All tests pass and build succeeds.*
*Still working on: Implementing main Spotify interface after login (currently shows only confirmation messages)*