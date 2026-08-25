# Cursor Buddy

> AI help, right where your cursor is.

Cursor Buddy is a Windows-native AI companion. Press **Ctrl+Shift+Space** anywhere and a
small bubble appears beside your mouse cursor — already aware of which application you're
using, so you can ask about what's in front of you without explaining it.

## Current status: Phases 1–2 (working shell + screen context)

- System tray with Pause AI, Permission Level, Settings, Quit
- Global activation shortcut (**Ctrl+Shift+Space**, remappable in Settings)
- Cursor detection + multi-monitor/DPI-aware popup placement (never off-screen)
- Draggable, pinnable bubble UX
- Active-window/process detection captured *before* the popup takes focus
- Streaming chat via OpenAI **and** Gemini (provider auto-detected from API key;
  credentials never enter the web bundle)
- Context-aware system prompt (active app, window title, cursor position)
- **Screen context (Phase 2)**: on-demand GDI capture of the active window +
  Windows.Media.Ocr text extraction (zh/en), in-memory only with a 5-minute TTL,
  fed into answers — visible as a `screen ✓` chip in the bubble
- Chinese/translation-aware response mode when your message contains Chinese text
- SQLite persistence for settings, conversations, and messages
- Permission levels (Observe / Guide / Assist / Autopilot) wired through a Safety Layer
  that gates all future computer-control actions
- First-run welcome, settings UI, conversation history

Roadmap: screen context & vision ✅ → UI Automation & Explain This → guidance overlay →
computer control → teaching & memory. See the architecture notes at the bottom.

## Prerequisites

- Windows 10 or 11
- [Rust](https://rustup.rs) (MSVC toolchain) + Visual Studio C++ Build Tools
- Node.js 18+
- WebView2 Runtime (preinstalled on Windows 11)

## Development

```powershell
npm install
npm run tauri dev
```

First launch opens the welcome window. Either paste an OpenAI API key into
Settings → AI, or set `OPENAI_API_KEY` before launching (see `.env.example`).

Then, from any app: press **Ctrl+Shift+Space**, and Buddy pops up next to your cursor.

## Building an installer

```powershell
npm run tauri build
```

Produces NSIS/MSI installers under `src-tauri/target/release/bundle`.

## Architecture

```
cursor-buddy/
├── src/                  # React + TypeScript UI
│   ├── app/              # window routing (bubble vs main)
│   ├── components/       # shared UI primitives
│   ├── features/
│   │   ├── bubble/       # cursor-anchored popup
│   │   └── main/         # expanded panel (chat/history/memory/permissions/settings)
│   ├── hooks/            # event wiring
│   ├── services/         # typed Tauri bridge (invoke/listen)
│   ├── stores/           # zustand state
│   └── types/
└── src-tauri/
    └── src/
        ├── lib.rs        # builder, plugins, tray, hotkey wiring
        ├── windows/      # native Win32: cursor, monitors/DPI, active window
        ├── placement.rs  # edge-aware bubble placement math (+ unit tests)
        ├── context.rs    # activation-time context snapshot
        ├── ai/           # provider trait, OpenAI SSE client, agent turn loop
        ├── safety/       # permission levels + action gate
        ├── storage/      # SQLite (settings/conversations/messages)
        ├── commands/     # chat / settings / misc IPC surface
        ├── hotkey.rs     # global shortcut registration
        ├── tray.rs       # tray menu + live state sync
        ├── events.rs     # typed event payloads (cb://*)
        └── state.rs      # AppState, settings model, key resolution
```

### Design decisions worth knowing

1. **Context is snapshotted before the bubble takes focus** — otherwise the "active app"
   would be Cursor Buddy itself.
2. **All screen math is physical pixels** internally, converted to logical only at the UI
   boundary; monitors are queried per-cursor-point for correct multi-DPI behavior.
3. **The agent/tool loop runs in Rust.** Provider credentials stay out of the webview;
   tools will execute as direct function calls with zero IPC round-trips.
4. **The Safety Layer exists from day one.** Even though computer control ships later,
   every planned action already flows through permission checks — automation can never
   bypass it by accident.

## Privacy

- No continuous capture of anything. Screen/app context is read only at activation.
- Screenshots never touch disk (vision caching will be in-memory with short TTL).
- The API key is stored locally (or via env var) and used only server-side in Rust.
- Pause AI stops context collection and automation instantly from tray or bubble.
