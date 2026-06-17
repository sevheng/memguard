# `memguard` Project Purpose Plan

## 1. Problem

Linux desktop crashes when memory fills up. No graceful degradation. No session protection.

| OS | Behavior |
|----|----------|
| macOS | Slows down, compresses, warns apps, never crashes |
| Windows | Suspends background apps, keeps session alive |
| Linux | OOM killer picks random process, often kills desktop shell |

## 2. Purpose

**Build a user-space daemon that makes Linux desktop memory management as reliable as macOS/Windows — without kernel changes or app cooperation.**

## 3. Core Objectives

| # | Objective | Success Metric |
|---|-----------|--------------|
| 1 | **Protect session** | Desktop shell never killed under pressure |
| 2 | **Graceful degradation** | System slows down before crashing |
| 3 | **Smart decisions** | Background apps frozen first, active apps protected |
| 4 | **Zero config** | Install and run, auto-detects desktop |
| 5 | **Minimal footprint** | <15MB RAM, <1% CPU |

## 4. Scope

**In scope:**
- Memory pressure monitoring (PSI)
- cgroup freeze / throttle / kill
- Desktop shell shield (`oom_score_adj`)
- Active window awareness
- systemd user service

**Out of scope:**
- GUI, tray icon, notifications
- Kernel patches
- Per-app memory optimization
- Browser extension

## 5. Target Users

| User | Pain Point |
|------|------------|
| Low-end hardware (N150, 8GB) | GNOME/KDE crashes daily |
| Developers | Docker + IDE + browser kills session |
| Thin clients / kiosks | Need stable unattended desktop |

## 6. Deliverables

| Phase | Output | Weeks |
|-------|--------|-------|
| 1 | Core daemon: freeze + kill + shell shield | 2 |
| 2 | Smart layer: active window + app classification | 2 |
| 3 | Packaging: RPM, systemd, COPR | 1 |
| 4 | Validation: N150 stress test + docs | 1 |

## 7. Companion: `memguard-system-tune`

One-time setup script for static optimizations:
- I/O scheduler (`bfq`)
- `ananicy-cpp` for CPU interactivity
- Service cleanup
- zram, fstrim, fstab `noatime`

**Separate package.** Install together, run once.

## 8. Success Criteria

- [ ] Open 50 Chrome tabs + VS Code + Docker on 8GB RAM
- [ ] System slows but **does not crash**
- [ ] Desktop shell survives 100% of pressure events
- [ ] Active window never killed
- [ ] Background apps frozen before killed
- [ ] Installable via `dnf install memguard memguard-system-tune`

## 9. Identity

| Element | Value |
|---------|-------|
| Name | `memguard` |
| Type | Systems daemon |
| Language | Rust |
| License | MIT or GPL-3 |

---

**Approve this purpose?** Then I generate the technical spec and Rust scaffold.