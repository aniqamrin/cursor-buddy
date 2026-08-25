import { useEffect, useState } from "react";
import {
  Brain,
  History,
  MessageSquareText,
  Pause,
  Play,
  Settings as SettingsIcon,
  ShieldCheck,
} from "lucide-react";
import { api } from "../../services/bridge";
import { useBuddy } from "../../stores/buddy";
import { useBuddyEvents } from "../../hooks/useBuddyEvents";
import { Orb } from "../../components/ui";
import type { PermissionLevel, SettingsDto } from "../../types";
import { ChatTab } from "./ChatTab";
import { HistoryTab } from "./HistoryTab";
import { MemoryTab } from "./MemoryTab";
import { PermissionsTab } from "./PermissionsTab";
import { SettingsTab } from "./SettingsTab";

type Tab = "chat" | "history" | "memory" | "permissions" | "settings";

const TABS: { id: Tab; label: string; icon: typeof MessageSquareText }[] = [
  { id: "chat", label: "Chat", icon: MessageSquareText },
  { id: "history", label: "History", icon: History },
  { id: "memory", label: "Memory", icon: Brain },
  { id: "permissions", label: "Permissions", icon: ShieldCheck },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

export function MainShell() {
  useBuddyEvents();
  const paused = useBuddy((s) => s.paused);
  const [tab, setTab] = useState<Tab>("chat");
  const [settings, setSettings] = useState<SettingsDto | null>(null);
  const [showWelcome, setShowWelcome] = useState(false);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setShowWelcome(!s.first_run_completed);
    });
    void seedLatestConversation();
  }, []);

  async function seedLatestConversation() {
    try {
      const convos = await api.listConversations();
      if (convos.length === 0) return;
      const rows = await api.getMessages(convos[0].id);
      useBuddy
        .getState()
        .seedMessages(
          rows.filter((r) => r.role !== "system").map((r) => ({ role: r.role as "user" | "assistant", content: r.content })),
        );
    } catch {
      // Fresh install — no history yet.
    }
  }

  const refreshSettings = () => api.getSettings().then(setSettings);

  return (
    <div className="flex h-full w-full bg-[#0b0c11] text-zinc-100">
      {/* rail */}
      <nav className="flex w-14 shrink-0 flex-col items-center gap-1 border-r border-white/[0.06] bg-black/20 py-3">
        <div className="mb-2">
          <Orb thinking={false} size={20} />
        </div>
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            title={label}
            onClick={() => setTab(id)}
            className={`rounded-lg p-2 transition ${
              tab === id
                ? "bg-violet-500/15 text-violet-300"
                : "text-zinc-500 hover:bg-white/5 hover:text-zinc-200"
            }`}
          >
            <Icon className="h-4.5 w-4.5" />
          </button>
        ))}
        <div className="mt-auto flex flex-col items-center gap-1">
          <span
            className={`rounded-full px-1.5 py-[2px] text-[9px] font-semibold uppercase tracking-wider ${
              paused ? "bg-amber-500/15 text-amber-400" : "bg-emerald-500/15 text-emerald-400"
            }`}
          >
            {paused ? "Paused" : "Active"}
          </span>
          <button
            onClick={() => void api.togglePause()}
            title={paused ? "Resume AI" : "Pause AI"}
            className="rounded-lg p-2 text-zinc-500 transition hover:bg-white/5 hover:text-zinc-200"
          >
            {paused ? <Play className="h-4 w-4" /> : <Pause className="h-4 w-4" />}
          </button>
        </div>
      </nav>

      {/* content */}
      <main className="relative min-w-0 flex-1">
        {tab === "chat" && <ChatTab />}
        {tab === "history" && <HistoryTab />}
        {tab === "memory" && settings && <MemoryTab settings={settings} onChanged={refreshSettings} />}
        {tab === "permissions" && settings && (
          <PermissionsTab level={settings.permission_level} />
        )}
        {tab === "settings" && settings && (
          <SettingsTab settings={settings} onChanged={refreshSettings} />
        )}

        {showWelcome && settings && (
          <Welcome
            onStart={() => {
              setShowWelcome(false);
              setTab("settings");
            }}
            onSkip={async () => {
              setShowWelcome(false);
              await persistFirstRunDone(settings);
              refreshSettings();
            }}
          />
        )}
      </main>
    </div>
  );
}

async function persistFirstRunDone(settings: SettingsDto) {
  if (!settings.first_run_completed) {
    await api.saveSettings({ ...settings, first_run_completed: true });
  }
}

function Welcome({ onStart, onSkip }: { onStart: () => void; onSkip: () => void }) {
  return (
    <div className="absolute inset-0 z-10 flex items-center justify-center bg-[#0b0c11]/95 backdrop-blur-sm">
      <div className="cb-pop w-[480px] rounded-2xl border border-white/10 bg-gradient-to-b from-violet-500/[0.08] to-transparent p-8 text-center shadow-2xl">
        <div className="mb-4 flex justify-center">
          <Orb size={36} thinking={false} />
        </div>
        <h1 className="text-xl font-semibold">Meet Cursor Buddy</h1>
        <p className="mt-1 text-sm italic text-violet-300/90">
          AI help, right where your cursor is.
        </p>
        <p className="mx-auto mt-4 max-w-[360px] text-[13px] leading-relaxed text-zinc-400">
          Press your shortcut anywhere and Buddy appears beside your cursor — aware of the app
          you're using. It can explain what's on screen, teach you things step by step, translate
          language in context, and optionally guide or control the computer with your permission.
        </p>
        <div className="mt-6 flex justify-center gap-2">
          <button
            onClick={onStart}
            className="rounded-lg bg-violet-600 px-5 py-2 text-sm font-medium text-white transition hover:bg-violet-500"
          >
            Get Started
          </button>
          <button
            onClick={onSkip}
            className="rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-400 transition hover:text-zinc-200"
          >
            Skip for now
          </button>
        </div>
        <p className="mt-4 text-[10px] text-zinc-600">
          Nothing is captured until you press the shortcut. You stay in control.
        </p>
      </div>
    </div>
  );
}

/* ---- shared section scaffolding ---- */

export function Section({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mb-8">
      <h2 className="text-sm font-semibold text-zinc-100">{title}</h2>
      {subtitle && <p className="mt-0.5 mb-3 text-xs text-zinc-500">{subtitle}</p>}
      <div className={!subtitle ? "mt-3" : ""}>{children}</div>
    </section>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  description,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  description?: string;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 rounded-xl border border-white/[0.07] bg-white/[0.02] p-3 transition hover:border-white/15">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-0.5 h-4 w-4 accent-violet-500"
      />
      <span>
        <span className="block text-[13px] text-zinc-200">{label}</span>
        {description && <span className="block text-[11px] text-zinc-500">{description}</span>}
      </span>
    </label>
  );
}

export const PERMISSION_INFO: Record<PermissionLevel, { title: string; blurb: string }> = {
  observe: {
    title: "Observe",
    blurb: "Buddy can see allowed context and analyze it, but never touches your computer.",
  },
  guide: {
    title: "Guide",
    blurb: "Buddy can highlight elements and give step-by-step instructions. It cannot click.",
  },
  assist: {
    title: "Assist",
    blurb: "Buddy performs individual actions after you confirm each one.",
  },
  autopilot: {
    title: "Autopilot",
    blurb: "Buddy can complete multi-step tasks on its own. Sensitive actions still require confirmation.",
  },
};
