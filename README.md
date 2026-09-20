# 🎮 GameWache

<p align="center">
  🌐 <strong>Languages / Języki:</strong>
  <a href="README.md"><strong>🇬🇧 English</strong></a> •
  <a href="README.pl.md">🇵🇱 Polski</a>
</p>

<p align="center">
  <strong>Autonomous GameOps Engine for Game Developers: Crash Sentinel, Deterministic Blackbox Replay, Remote CAS & AI MCP Server in a single Rust binary.</strong><br>
  <em>A fusion of Nachtwache (observability, zero-dependency, AI Auto-Fix) and Blitzrunner (CI optimization, smart prune, bare-metal runners) purpose-built for game engines (Godot 4, Unity, Unreal, Custom).</em>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-amber.svg?style=for-the-badge" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/RAM_Footprint-~25_MB-10b981?style=for-the-badge&logo=ram" alt="RAM ~25MB">
  <img src="https://img.shields.io/badge/Architecture-Single_Binary_Rust-3b82f6?style=for-the-badge&logo=rust" alt="Single Binary">
  <img src="https://img.shields.io/badge/AI_Protocol-Native_MCP_Server-8B5CF6?style=for-the-badge" alt="Native MCP Server">
  <img src="https://img.shields.io/badge/Engines-Godot_4_%7C_Unity-2496ED?style=for-the-badge" alt="Godot & Unity">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Buy_Me_A_Coffee-adrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Buy Me A Coffee"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Support_Author-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

---

## ⚡ Key Highlights

* **Single Binary (~9.4 MB)**: Written in Rust 2024, embedded SQLite WAL database, embedded Web UI dashboard (rust-embed), ~25 MB RAM idle footprint.
* **Deterministic Blackbox Replay**: Circular ring buffer in RAM recording the last 30-60 seconds of player inputs with zero performance penalty (<0.02ms/frame). Replay crashes frame-by-frame in editor or headless CI!
* **Native Model Context Protocol (MCP) Server**: AI agents (Antigravity, Claude Code, Cursor) can query crashes, inspect replays, tune parameters in real-time, audit assets, and run headless tests.
* **Live WebSocket Parameter Tweaker**: Tune physics, weapon heat, recoil, and gameplay balance in the live running game from your browser or AI chat prompt.
* **Shader Stutter Tracker & Pre-warm Generator**: Automatically logs GPU pipeline hitches and generates `shader_prewarm.gd` / `ShaderPrewarm.cs` to eliminate micro-stutters during loading screens.
* **Orphan Asset Hunter & CI Quality Gate**: Scans scene reachability DAGs to prune unreferenced textures, models, and audio before shipping.

---

## 🚀 Quickstart

```bash
# Clone and build
git clone https://github.com/adrian-wulf/gamewache.git
cd gamewache
cargo build --release

# Run GameOps Server (Dashboard at http://localhost:8090)
./target/release/gamewache serve --port 8090

# Run MCP Server for AI Agents
./target/release/gamewache mcp --project-dir /path/to/game
```

---

## 📄 License
MIT License. Created by [Adrian Wulf](https://social-wulf.eu).
