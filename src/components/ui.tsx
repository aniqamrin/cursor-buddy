import { ShieldCheck, ShieldAlert } from "lucide-react";
import type { PermissionLevel } from "../types";

export function Orb({ thinking, size = 18 }: { thinking: boolean; size?: number }) {
  if (thinking) {
    return (
      <span
        className="cb-orb-ring inline-block rounded-full"
        style={{
          width: size,
          height: size,
          WebkitMask: "radial-gradient(circle, transparent 34%, black 36%)",
          mask: "radial-gradient(circle, transparent 34%, black 36%)",
        }}
      />
    );
  }
  return (
    <span
      className="cb-orb-idle inline-block rounded-full"
      style={{ width: size, height: size }}
    />
  );
}

const LEVEL_META: Record<PermissionLevel, { label: string }> = {
  observe: { label: "Observe" },
  guide: { label: "Guide" },
  assist: { label: "Assist" },
  autopilot: { label: "Autopilot" },
};

export function PermissionChip({ level }: { level: PermissionLevel }) {
  const meta = LEVEL_META[level] ?? { label: level };
  return (
    <span
      title={`Permission level: ${meta.label}`}
      className="inline-flex items-center gap-1 rounded-full border border-white/10 bg-white/[0.04] px-2 py-[3px] text-[10px] font-medium text-zinc-300"
    >
      {level === "autopilot" ? (
        <ShieldAlert className="h-3 w-3 text-amber-400" />
      ) : (
        <ShieldCheck className="h-3 w-3 text-violet-400" />
      )}
      {meta.label}
    </span>
  );
}

export function ContextChip({
  appName,
  title,
}: {
  appName: string | null;
  title?: string | null;
}) {
  if (!appName) return null;
  return (
    <span
      title={title ? `${appName} — ${title}` : appName}
      className="max-w-[140px] truncate rounded-full border border-white/10 bg-white/[0.04] px-2 py-[3px] text-[10px] font-medium text-zinc-300"
    >
      {appName}
    </span>
  );
}

/** Markdown-lite renderer: bold, code ticks, numbered/bulleted lines, paragraphs. */
export function RichText({ text }: { text: string }) {
  const blocks = text.split(/\n\n+/);
  return (
    <div className="space-y-2 leading-relaxed">
      {blocks.map((block, bi) => {
        const lines = block.split("\n");
        const isList = lines.every((l) => /^\s*(\d+[.)]|[-•*])\s+/.test(l));
        if (isList && lines.length > 0) {
          return (
            <ol key={bi} className="ml-4 list-decimal space-y-1">
              {lines.map((l, li) => (
                <li key={li}>{inline(l.replace(/^\s*(\d+[.)]|[-•*])\s+/, ""))}</li>
              ))}
            </ol>
          );
        }
        return (
          <p key={bi} className="whitespace-pre-wrap break-words">
            {lines.map((l, li) => (
              <span key={li}>
                {inline(l)}
                {li < lines.length - 1 ? <br /> : null}
              </span>
            ))}
          </p>
        );
      })}
    </div>
  );
}

function inline(text: string) {
  // Split on **bold** and `code`.
  const parts = text.split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
  return parts.map((part, i) => {
    if (part.startsWith("**") && part.endsWith("**")) {
      return (
        <strong key={i} className="font-semibold">
          {part.slice(2, -2)}
        </strong>
      );
    }
    if (part.startsWith("`") && part.endsWith("`")) {
      return (
        <code
          key={i}
          className="rounded bg-white/10 px-1 py-[1px] font-mono text-[12px]"
        >
          {part.slice(1, -1)}
        </code>
      );
    }
    return <span key={i}>{part}</span>;
  });
}
