import { CheckCircle2 } from "lucide-react";
import { api } from "../../services/bridge";
import { useBuddy } from "../../stores/buddy";
import type { PermissionLevel } from "../../types";
import { PERMISSION_INFO } from "./MainShell";

const ORDER: PermissionLevel[] = ["observe", "guide", "assist", "autopilot"];

export function PermissionsTab({ level }: { level: PermissionLevel }) {
  async function choose(next: PermissionLevel) {
    try {
      await api.setPermissionLevel(next);
    } catch (e) {
      console.error(e);
    }
  }

  return (
    <div className="scroll-thin h-full overflow-y-auto px-6 py-5">
      <h1 className="text-sm font-semibold">Permissions</h1>
      <p className="mb-6 text-xs text-zinc-500">
        Controls what Buddy may do on your computer. The active level is always visible in the
        bubble header.
      </p>

      <div className="grid gap-3 lg:grid-cols-2">
        {ORDER.map((lvl) => {
          const info = PERMISSION_INFO[lvl];
          const active = lvl === useBuddy.getState().level || lvl === level;
          return (
            <button
              key={lvl}
              onClick={() => void choose(lvl)}
              className={`rounded-xl border p-4 text-left transition ${
                active
                  ? "border-violet-400/60 bg-violet-500/10"
                  : "border-white/[0.07] bg-white/[0.02] hover:border-white/20"
              }`}
            >
              <span className="flex items-center gap-2">
                <span className="text-[13px] font-semibold capitalize">{info.title}</span>
                {active && <CheckCircle2 className="h-4 w-4 text-violet-400" />}
              </span>
              <span className="mt-1 block text-[12px] leading-relaxed text-zinc-400">
                {info.blurb}
              </span>
            </button>
          );
        })}
      </div>

      <p className="mt-6 rounded-xl border border-white/[0.07] bg-white/[0.02] p-3 text-[12px] leading-relaxed text-zinc-400">
        Sensitive actions (sending messages, deleting files, purchases, installs, password
        changes) always require explicit confirmation — even in Autopilot. Computer control
        itself arrives with the automation phase; these levels gate it from day one.
      </p>
    </div>
  );
}
