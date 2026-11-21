# 🎮 GameWache

<p align="center">
  🌐 <strong>Langues / Languages:</strong>
  <a href="README.md">🇬🇧 English</a> •
  <a href="README.pl.md">🇵🇱 Polski</a> •
  <a href="README.de.md">🇩🇪 Deutsch</a> •
  <a href="README.es.md">🇪🇸 Español</a> •
  <a href="README.fr.md"><strong>🇫🇷 Français</strong></a>
</p>

<p align="center">
  <strong>Moteur autonome GameOps pour créateurs de jeux vidéo : sentinelle de crashs, relecture boîte noire (Blackbox Replay), cache distant d'assets (CAS), réglage en direct et serveur MCP natif pour IA dans un binaire Rust unique.</strong><br>
  <em>Une fusion de Nachtwache (observabilité, zéro dépendance, diagnostic) et Blitzrunner (optimisation CI, élagage intelligent de builds, accélération bare-metal) spécialement conçue pour les moteurs de jeux (Godot 4, Unity, Unreal, Custom).</em>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/Licence-MIT-amber.svg?style=for-the-badge" alt="Licence MIT"></a>
  <img src="https://img.shields.io/badge/Empreinte_RAM-~25_Mo-10b981?style=for-the-badge&logo=ram" alt="RAM ~25Mo">
  <img src="https://img.shields.io/badge/Architecture-Binaire_Rust_Unique-3b82f6?style=for-the-badge&logo=rust" alt="Single Binary Rust">
  <img src="https://img.shields.io/badge/Protocole_IA-Serveur_MCP_Natif-8B5CF6?style=for-the-badge" alt="Serveur MCP Natif">
  <img src="https://img.shields.io/badge/Moteurs-Godot_4_%7C_Unity-2496ED?style=for-the-badge" alt="Godot & Unity">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Offrir_un_Café-adrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Offrir un Café"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Soutenir_l'Auteur-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-DEA584?style=flat-square&logo=rust" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/Axum-0.8-black?style=flat-square" alt="Axum 0.8">
  <img src="https://img.shields.io/badge/Stockage-SQLite_WAL-003B57?style=flat-square&logo=sqlite" alt="SQLite WAL">
  <img src="https://img.shields.io/badge/CAS-Blake3_%2B_zstd-EAB308?style=flat-square" alt="CAS Engine">
  <img src="https://img.shields.io/badge/Réglage_Live-WebSocket-06B6D4?style=flat-square" alt="WebSocket Live">
  <img src="https://img.shields.io/badge/Docker-Prêt-2496ED?style=flat-square&logo=docker" alt="Docker Prêt">
  <img src="https://img.shields.io/badge/Philosophie-RobinHood_dev-EAB308?style=flat-square" alt="RobinHood dev">
</p>

---

## ⚡ Pourquoi GameWache ?

Les outils traditionnels du web ne répondent pas aux besoins du jeu vidéo :
1. **Absence de contexte jeu vidéo** : Une simple trace d'erreur ne capture ni le tick physique, ni les entrées manette, ni les chutes de FPS, ni les coordonnées spatiales `(x, y, z)` sur le niveau.
2. **Relecture déterministe (.gwrec)** : Enregistre les 30 à 60 dernières secondes d'entrées joueur (<0.02ms/frame) pour reproduire les crashs image par image dans l'éditeur.
3. **Cache distant d'assets (CAS)** : Accéléré par **Blake3** (>5 Go/s) et **zstd**, réduisant jusqu'à 80% des temps de CI.
4. **Serveur MCP natif pour IA** : Connectez vos agents d'ingénierie (Google Antigravity, Cursor, Claude Code) pour diagnostiquer les incidents et ajuster les constantes de jeu en temps réel.

---

## 🚀 Démarrage Rapide

```bash
# Cloner et compiler
git clone https://github.com/adrian-wulf/gamewache.git
cd gamewache
cargo build --release

# Lancer le serveur GameOps (Tableau de bord sur http://localhost:8090)
./target/release/gamewache serve --port 8090

# Serveur MCP universel pour agents IA (autodétection Godot/Unity)
./target/release/gamewache mcp
```

---

## ☕ Soutenir le Projet & RobinHood dev

<p align="center">
  <a href="https://buymeacoffee.com/adrianwulf"><img src="https://img.shields.io/badge/☕_Offrir_un_Café-buymeacoffee.com%2Fadrianwulf-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Offrir un Café"></a>
  <a href="https://github.com/sponsors/adrian-wulf"><img src="https://img.shields.io/badge/GitHub_Sponsors-Soutenir_l'Auteur-EA4AAA?style=for-the-badge&logo=github-sponsors" alt="GitHub Sponsors"></a>
</p>

### Philosophie RobinHood dev :
> **"Les infrastructures professionnelles de GameOps et de diagnostic de crash doivent être accessibles à tous les créateurs de jeux indépendants, sans abonnements exorbitants."**

GameWache est 100% logiciel libre et open source sous licence MIT.

---

## ⚖️ Mentions Légales & Licence (§ 5 DDG / MIT)

* **Développeur / Auteur :** Adrian Wulf ([https://github.com/adrian-wulf](https://github.com/adrian-wulf), [https://wulf-code.eu](https://wulf-code.eu)).
* **Licence :** MIT License. Consultez [LICENSE](LICENSE) pour plus d'informations.

---

<p align="center">
  <em>Conçu avec passion pour la communauté gamedev indépendante par Adrian Wulf.</em>
</p>
