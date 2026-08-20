# Windows Desktop Build — v1.7.0

1. Extract the ZIP to a normal local folder. Do not run it from inside the ZIP.
2. Double-click `BUILD_AND_RUN_DEMO.cmd`.
3. The script checks Node, Rust, Cargo, Tauri CLI and Visual Studio C++ Build Tools.
4. It installs npm dependencies if `node_modules` is missing.
5. It builds React, checks Rust, builds the Tauri Windows application and installer.
6. It starts the built desktop EXE automatically in Demo mode.

Demo login is automatic in Demo mode. The seeded account is `admin` / `demo`.

Installer output:
`apps\desktop-host\src-tauri\target\release\bundle`

Important: `tauri.conf.json` is set to version 1.7.0.


## v1.7.1 strict build gate
The build script now runs `npx tsc --noEmit` before Vite/Tauri build. TypeScript errors stop the build immediately. React type packages are included in devDependencies.
