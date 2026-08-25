import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Pause, Pin, PinOff, Play, X } from "lucide-react";
import { api } from "../../services/bridge";
import { useBuddy } from "../../stores/buddy";
import { useBuddyEvents } from "../../hooks/useBuddyEvents";
import { Composer, MessageList } from "../../components/chat";
import { ContextChip, Orb, PermissionChip } from "../../components/ui";

const QUICK_PROMPTS = [
  "What am I looking at?",
  "How do I do this faster?",
  "Explain this simply",
];

export function Bubble() {
  useBuddyEvents();
  const paused = useBuddy((s) => s.paused);
  const level = useBuddy((s) => s.level);
  const pinned = useBuddy((s) => s.pinned);
  const appName = useBuddy((s) => s.appName);
  const windowTitle = useBuddy((s) => s.windowTitle);
  const status = useBuddy((s) => s.status);
  const messages = useBuddy((s) => s.messages);
  const activationCount = useBuddy((s) => s.activationCount);

  const [hotkeyLabel, setHotkeyLabel] = useState("Ctrl+Shift+Space");

  useEffect(() => {
    api.getSettings().then((s) => {
      setHotkeyLabel(
        s.hotkey
          .replace(/control\+/i, "Ctrl+")
          .replace(/shift\+/i, "Shift+")
          .replace(/alt\+/i, "Alt+"),
      );
    });
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void api.hideBubble();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Dismiss when the user clicks elsewhere — unless pinned.
  useEffect(() => {
    const onBlur = () => {
      if (!useBuddy.getState().pinned) void api.hideBubble();
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, []);

  const togglePinned = useCallback(() => {
    const next = !useBuddy.getState().pinned;
    useBuddy.getState().setPinned(next);
    void api.setPinned(next);
  }, []);

  // Manual drag: any header press outside a button starts a native move.
  const onHeaderMouseDown = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button")) return;
    void getCurrentWindow().startDragging();
  }, []);

  const submit = useCallback(
    (text: string) => {
      useBuddy.getState().pushLocalUserMessage(text);
      api.send(text).catch((err) => useBuddy.getState().fail(String(err)));
    },
    [],
  );

  const resume = () => api.togglePause();

  return (
    <div className="cb-pop h-full w-full p-[6px]">
      <div className="flex h-full w-full flex-col overflow-hidden rounded-2xl border border-white/10 bg-[#0b0c11]/95 text-zinc-100 shadow-[0_18px_60px_-12px_rgba(0,0,0,0.85)] ring-1 ring-black/50">
        {/* header — drag to move; buttons stay clickable */}
        <div
          onMouseDown={onHeaderMouseDown}
          className="flex h-11 shrink-0 items-center gap-2 border-b border-white/[0.06] px-3 cursor-move select-none"
        >
          <Orb thinking={status === "thinking"} size={16} />
          <span className="text-[13px] font-semibold tracking-wide">
            Cursor Buddy
          </span>
          <span className="ml-auto flex items-center gap-1.5">
            <ContextChip appName={appName} title={windowTitle} />
            <PermissionChip level={level} />
            <button
              onClick={togglePinned}
              className={
                "rounded-md p-1 transition " +
                (pinned
                  ? "bg-violet-500/20 text-violet-300 hover:bg-violet-500/30"
                  : "text-zinc-400 hover:bg-white/10 hover:text-zinc-100")
              }
              title={pinned ? "Unpin (returns to cursor)" : "Pin in place"}
            >
              {pinned ? <PinOff className="h-3.5 w-3.5" /> : <Pin className="h-3.5 w-3.5" />}
            </button>
            <button
              onClick={() => void api.togglePause()}
              className="rounded-md p-1 text-zinc-400 transition hover:bg-white/10 hover:text-zinc-100"
              title={paused ? "Resume AI" : "Pause AI"}
            >
              {paused ? <Play className="h-3.5 w-3.5" /> : <Pause className="h-3.5 w-3.5" />}
            </button>
            <button
              onClick={() => void api.hideBubble()}
              className="rounded-md p-1 text-zinc-400 transition hover:bg-white/10 hover:text-zinc-100"
              title="Dismiss (Esc)"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </span>
        </div>

        {/* body */}
        {paused ? (
          <PausedPanel onResume={resume} />
        ) : messages.length === 0 ? (
          <EmptyBody onPick={submit} />
        ) : (
          <MessageList compact />
        )}

        {/* composer */}
        {!paused && (
          <Composer
            onSubmit={submit}
            autoFocusKey={activationCount}
            hint={`Enter ↵ send · Esc dismiss · ${hotkeyLabel} anywhere`}
          />
        )}
      </div>
    </div>
  );
}

function EmptyBody({ onPick }: { onPick: (t: string) => void }) {
  return (
    <div className="scroll-thin flex min-h-0 flex-1 flex-col items-start justify-center gap-2 px-4 py-2">
      <p className="text-[13px] text-zinc-300">I see what you're working on.</p>
      <div className="flex flex-wrap gap-1.5">
        {QUICK_PROMPTS.map((q) => (
          <button
            key={q}
            onClick={() => onPick(q)}
            className="rounded-full border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[11px] text-zinc-300 transition hover:border-violet-400/50 hover:bg-violet-500/10 hover:text-violet-200"
          >
            {q}
          </button>
        ))}
      </div>
    </div>
  );
}

function PausedPanel({ onResume }: { onResume: () => void }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center">
      <Pause className="h-6 w-6 text-zinc-400" />
      <p className="text-[13px] font-medium text-zinc-200">Cursor Buddy is paused.</p>
      <ul className="space-y-0.5 text-[11px] text-zinc-500">
        <li>Screen capture: OFF</li>
        <li>Context capture: OFF</li>
        <li>Automation: OFF</li>
      </ul>
      <button
        onClick={onResume}
        className="mt-1 rounded-lg bg-violet-600 px-3 py-1.5 text-[12px] font-medium text-white transition hover:bg-violet-500"
      >
        Resume
      </button>
    </div>
  );
}
