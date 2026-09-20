# 🎮 GameWache

<p align="center">
  🌐 <strong>Idiomas / Languages:</strong>
  <a href="README.md">🇬🇧 English</a> •
  <a href="README.pl.md">🇵🇱 Polski</a> •
  <a href="README.de.md">🇩🇪 Deutsch</a> •
  <a href="README.es.md"><strong>🇪🇸 Español</strong></a> •
  <a href="README.fr.md">🇫🇷 Français</a>
</p>

<p align="center">
  <strong>Motor GameOps autónomo para desarrolladores de videojuegos: monitor de fallos, repetición de caja negra (Blackbox Replay), caché remota de assets (CAS), ajuste en vivo de variables y servidor MCP nativo para IA en un único binario Rust.</strong><br>
  <em>Una fusión de Nachtwache (observabilidad, cero dependencias, diagnóstico) y Blitzrunner (optimización CI, poda inteligente de builds, aceleración bare-metal) diseñada especialmente para motores de juegos (Godot 4, Unity, Unreal, Custom).</em>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/Licencia-MIT-amber.svg?style=for-the-badge" alt="Licencia MIT"></a>
  <img src="https://img.shields.io/badge/Consumo_RAM-~25_MB-10b981?style=for-the-badge&logo=ram" alt="RAM ~25MB">
  <img src="https://img.shields.io/badge/Arquitectura-Binario_Rust_Único-3b82f6?style=for-the-badge&logo=rust" alt="Single Binary Rust">
  <img src="https://img.shields.io/badge/Protocolo_IA-Servidor_MCP_Nativo-8B5CF6?style=for-the-badge" alt="Servidor MCP Nativo">
  <img src="https://img.shields.io/badge/Motores-Godot_4_%7C_Unity-2496ED?style=for-the-badge" alt="Godot & Unity">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Invítame_a_un_Café-adrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Invítame a un Café"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Apoyar_al_Autor-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-DEA584?style=flat-square&logo=rust" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/Axum-0.8-black?style=flat-square" alt="Axum 0.8">
  <img src="https://img.shields.io/badge/Almacenamiento-SQLite_WAL-003B57?style=flat-square&logo=sqlite" alt="SQLite WAL">
  <img src="https://img.shields.io/badge/CAS-Blake3_%2B_zstd-EAB308?style=flat-square" alt="CAS Engine">
  <img src="https://img.shields.io/badge/Ajuste_en_Vivo-WebSocket-06B6D4?style=flat-square" alt="WebSocket Live">
  <img src="https://img.shields.io/badge/Docker-Listo-2496ED?style=flat-square&logo=docker" alt="Docker Listo">
  <img src="https://img.shields.io/badge/Filosofía-RobinHood_dev-EAB308?style=flat-square" alt="RobinHood dev">
</p>

---

## ⚡ ¿Por qué GameWache?

Las herramientas de observabilidad estándar de la web no comprenden los desafíos del desarrollo de videojuegos:
1. **Falta de contexto de juego**: Una excepción clásica no registra los fotogramas de física, secuencias del mando, caídas de FPS ni las coordenadas espaciales `(x, y, z)` en el nivel.
2. **Repetición determinista de caja negra (.gwrec)**: Almacena en memoria RAM los últimos 30–60 segundos de entradas del jugador (<0.02ms/fotograma) para reproducir caídas paso a paso en el editor.
3. **Caché remota de assets (CAS) ultrarrápida**: Acelerada con **Blake3** (>5 GB/s) y compresión **zstd**, ahorrando hasta el 80% del tiempo de compilación en CI.
4. **Servidor MCP nativo para IA**: Conecta asistentes como Google Antigravity, Cursor y Claude Code para investigar errores, balancear física en tiempo real y ejecutar pruebas headless.

---

## 🚀 Inicio Rápido

```bash
# Clonar y compilar
git clone https://github.com/adrian-wulf/gamewache.git
cd gamewache
cargo build --release

# Iniciar servidor GameOps (Panel web en http://localhost:8090)
./target/release/gamewache serve --port 8090

# Servidor universal MCP para agentes IA (autodetecta proyectos Godot/Unity)
./target/release/gamewache mcp
```

---

## ☕ Apoyo al Proyecto & RobinHood dev

<p align="center">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Invítame_a_un_Café-buymeacoffee.com%2Fadrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Invítame a un Café"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Apoyar_al_Autor-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

### Filosofía RobinHood dev:
> **"La infraestructura profesional de GameOps y diagnóstico debe estar al alcance de todos los desarrolladores independientes, sin barreras ni suscripciones mensuales abusivas."**

GameWache es 100% software libre de código abierto bajo la Licencia MIT.

---

## ⚖️ Aviso Legal & Licencia (§ 5 DDG / MIT)

* **Desarrollador / Autor:** Adrian Wulf ([https://github.com/adrian-wulf](https://github.com/adrian-wulf), [https://social-wulf.eu](https://social-wulf.eu)).
* **Licencia:** MIT License. Consulte [LICENSE](LICENSE) para más detalles.

---

<p align="center">
  <em>Creado con pasión para la comunidad gamedev independiente por Adrian Wulf.</em>
</p>
