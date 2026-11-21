# 🎮 GameWache

<p align="center">
  🌐 <strong>Sprachen / Languages:</strong>
  <a href="README.md">🇬🇧 English</a> •
  <a href="README.pl.md">🇵🇱 Polski</a> •
  <a href="README.de.md"><strong>🇩🇪 Deutsch</strong></a> •
  <a href="README.es.md">🇪🇸 Español</a> •
  <a href="README.fr.md">🇫🇷 Français</a>
</p>

<p align="center">
  <strong>Autonome GameOps-Engine für Spieleentwickler: Crash-Sentinel, deterministischer Blackbox-Input-Replay, Remote-Asset-Cache (CAS), Live-Parameter-Tweaker und nativer MCP-Server für KI in einer einzigen Rust-Binärdatei.</strong><br>
  <em>Eine Fusion aus Nachtwache (Observability, Zero-Dependencies, Crash-Diagnose) und Blitzrunner (CI-Optimierung, Smart Pruning, Bare-Metal-Beschleunigung) speziell für Spiele-Engines (Godot 4, Unity, Unreal, Custom).</em>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/Lizenz-MIT-amber.svg?style=for-the-badge" alt="MIT Lizenz"></a>
  <img src="https://img.shields.io/badge/RAM_Verbrauch-~25_MB-10b981?style=for-the-badge&logo=ram" alt="RAM ~25MB">
  <img src="https://img.shields.io/badge/Architektur-Single_Binary_Rust-3b82f6?style=for-the-badge&logo=rust" alt="Single Binary Rust">
  <img src="https://img.shields.io/badge/KI_Protokoll-Nativer_MCP_Server-8B5CF6?style=for-the-badge" alt="Nativer MCP Server">
  <img src="https://img.shields.io/badge/Engines-Godot_4_%7C_Unity-2496ED?style=for-the-badge" alt="Godot & Unity">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Kaffee_Spenden-adrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Kaffee Spenden"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Entwickler_Unterstützen-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-DEA584?style=flat-square&logo=rust" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/Axum-0.8-black?style=flat-square" alt="Axum 0.8">
  <img src="https://img.shields.io/badge/Speicher-SQLite_WAL-003B57?style=flat-square&logo=sqlite" alt="SQLite WAL">
  <img src="https://img.shields.io/badge/CAS-Blake3_%2B_zstd-EAB308?style=flat-square" alt="CAS Engine">
  <img src="https://img.shields.io/badge/Live_Tuning-WebSocket-06B6D4?style=flat-square" alt="WebSocket Live">
  <img src="https://img.shields.io/badge/Docker-Bereit-2496ED?style=flat-square&logo=docker" alt="Docker Bereit">
  <img src="https://img.shields.io/badge/Philosophie-RobinHood_dev-EAB308?style=flat-square" alt="RobinHood dev">
</p>

---

## 📑 Inhaltsverzeichnis
1. [⚡ Warum GameWache?](#-warum-gamewache)
2. [🌐 Adrian Wulf Ökosystem](#-adrian-wulf-ökosystem)
3. [📊 Vergleich: Traditionelle Tools vs GameWache](#-vergleich-traditionelle-tools-vs-gamewache)
4. [🏗️ Systemarchitektur](#️-systemarchitektur)
5. [🚀 Schnellstart](#-schnellstart)
6. [🤖 KI-Agenten-Integration (Nativer MCP-Server)](#-ki-agenten-integration-nativer-mcp-server)
7. [📼 Deterministischer Blackbox-Replay (.gwrec)](#-deterministischer-blackbox-replay-gwrec)
8. [⚡ Remote-Asset-Cache (CAS) & CI Smart Pruning](#-remote-asset-cache-cas--ci-smart-pruning)
9. [🎛️ Live-WebSocket-Parameter-Tweaker](#️-live-websocket-parameter-tweaker)
10. [⚡ Shader-Stutter-Tracker & Pre-Warm-Generator](#-shader-stutter-tracker--pre-warm-generator)
11. [🧹 Verwaiste Assets finden & Speicherbudgets](#-verwaiste-assets-finden--speicherbudgets)
12. [🎮 Engine-Integration (Godot 4 & Unity SDK)](#-engine-integration-godot-4--unity-sdk)
13. [🌐 Privates Playtest-Hosting (WASM / HTML5)](#-privates-playtest-hosting-wasm--html5)
14. [🐳 Produktionsbereitstellung (Docker & Coolify)](#-produktionsbereitstellung-docker--coolify)
15. [☕ Projekt unterstützen & RobinHood dev](#-projekt-unterstützen--robinhood-dev)
16. [⚖️ Impressum & Rechtliche Hinweise (§ 5 DDG / MIT)](#️-impressum--rechtliche-hinweise--5-ddg--mit)

---

## ⚡ Warum GameWache?

Klassische Observability-Tools und Web-CI versagen in der Spieleentwicklung:

1. **Web-Crash-Tracker (Sentry, BugSplat) fehlt der Spielkontext**: Ein Stacktrace wie `NullReferenceException` verrät weder den Physik-Tick, Controller-Eingaben, FPS-Einbrüche noch die Raumkoordinaten `(x, y, z)` auf der Map.
2. **Der Reproduktions-Albtraum**: Meldet ein Spieler *"Crash beim Bosskampf"*, verbringen Entwickler Tage mit Raten. GameWache speichert einen zirkulären **Blackbox-Replay** (<0.02ms/Frame) der letzten 30–60 Sekunden Spielereingaben zur Frame-genauen Wiedergabe im Editor.
3. **Spiele-CI ist extrem langsam und teuer**: Texturkomprimierung (ASTC/BC7) und Asset-Baking dauern 30–90 Minuten auf sauberen CI-Maschinen. GameWache bietet einen blitzschnellen **Remote-CAS-Cache** (Blake3 >5 GB/s + zstd) und **Smart Pruning**, das Asset-Importe überspringt, wenn nur Code geändert wurde.
4. **KI-gestützte Entwicklung**: Mit dem integrierten **Model Context Protocol (MCP)** Server können KI-Agenten (Antigravity, Cursor, Claude Code) Fehler untersuchen, Replays analysieren, Spielparameter live anpassen und Headless-Tests ausführen.

---

## 🌐 Adrian Wulf Ökosystem

GameWache ist Teil einer unabhängigen Suite moderner Entwickler-Werkzeuge unter der **RobinHood dev** Philosophie:

| Service / Projekt | Live URL | Zweck |
| :--- | :---: | :--- |
| 🎮 **GameWache** | [github.com/adrian-wulf/gamewache](https://github.com/adrian-wulf/gamewache) | **Autonome GameOps-Engine:** Crash Sentinel, Replay Blackbox, Remote CAS und KI MCP Server. |
| 🛡️ **Nachtwache** | [adrian-wulf/nachtwache](https://github.com/adrian-wulf/nachtwache) | **Autonomer Error Tracker:** Drop-in Sentry-Ersatz in einer Rust-Binärdatei (~15 MB RAM) mit KI Auto-Fix ([GitHub](https://github.com/adrian-wulf/nachtwache)). |
| ⚡ **BlitzRunner** | [github.com/adrian-wulf/blitzrunner](https://github.com/adrian-wulf/blitzrunner) | **Zero-Server CI-Optimierer:** Git-Diff-Analyse und ephemere Hetzner Bare-Metal-Runner. |
| 🌐 **Zentraler Wulf Hub** | [wulf-code.eu](https://wulf-code.eu) | **Haupt-Ökosystem:** Technologie-Portfolio und Entwickler-Souveränität. |
| 💼 **Wulf Code** | [wulf-code.it](https://wulf-code.it) | **Software House & Beratung:** High-Performance-Infrastruktur und Sicherheitsaudits. |

---

## 🚀 Schnellstart

```bash
# Repository klonen und kompilieren
git clone https://github.com/adrian-wulf/gamewache.git
cd gamewache
cargo build --release

# Server starten (Web Dashboard unter http://localhost:8090)
./target/release/gamewache serve --port 8090

# Universeller MCP-Server für KI-Agenten (erkennt automatisch Godot/Unity)
./target/release/gamewache mcp
```

---

## ☕ Projekt unterstützen & RobinHood dev

<p align="center">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Kaffee_Spenden-buymeacoffee.com%2Fadrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Kaffee Spenden"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Entwickler_Unterstützen-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

### Was ist die RobinHood dev Philosophie?
> **"Professionelle GameOps-, Crash-Diagnose- und CI-Infrastruktur muss jedem Indie-Spieleentwickler frei zugänglich sein, nicht nur Konzernen mit Millionenbudgets."**

GameWache ist zu 100% Freie und Open-Source-Software unter der MIT-Lizenz.

---

## ⚖️ Impressum & Rechtliche Hinweise (§ 5 DDG / MIT)

### Angaben gemäß § 5 deutsches Digitale-Dienste-Gesetz (DDG):
* **Entwickler / Herausgeber:** Adrian Wulf
* **Kontakt:** Erreichbar über GitHub: [https://github.com/adrian-wulf](https://github.com/adrian-wulf) und das Portal [https://wulf-code.eu](https://wulf-code.eu) / [https://wulf-code.it](https://wulf-code.it).

### Haftungsausschluss & MIT-Lizenz:
Die Software wird "WIE BESEHEN" (AS IS) ohne jegliche Gewährleistung bereitgestellt. Details in [LICENSE](LICENSE).

### Markenhinweis:
* **Godot Engine** ist eine eingetragene Marke der Godot Foundation.
* **Unity** ist eine eingetragene Marke von Unity Technologies.
* **Sentry** ist eine eingetragene Marke von Functional Software, Inc.  
GameWache ist ein unabhängiges Open-Source-Projekt und steht in keiner Verbindung zu diesen Unternehmen.

---

<p align="center">
  <em>Mit Leidenschaft für Indie-Game-Entwickler weltweit erschaffen von Adrian Wulf.</em>
</p>
