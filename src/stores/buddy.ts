import { create } from "zustand";
import type { ActivatePayload, PermissionLevel } from "../types";

export interface ChatMsg {
  role: "user" | "assistant";
  content: string;
}

export type BuddyStatus = "idle" | "thinking" | "error";

interface BuddyState {
  paused: boolean;
  level: PermissionLevel;
  appName: string | null;
  windowTitle: string | null;
  messages: ChatMsg[];
  streamingText: string;
  status: BuddyStatus;
  error: string | null;
  /** Increments every time the bubble activates at the cursor. */
  activationCount: number;
  /** Pinned bubbles ignore blur-dismissal and keep their position. */
  pinned: boolean;
  /** Character count of OCR'd screen text for this activation (null = pending). */
  screenChars: number | null;

  activate: (p: ActivatePayload) => void;
  setPaused: (paused: boolean) => void;
  setLevel: (level: PermissionLevel) => void;
  setPinned: (pinned: boolean) => void;
  setScreenChars: (chars: number | null) => void;
  seedMessages: (messages: ChatMsg[]) => void;
  pushLocalUserMessage: (content: string) => void;
  generationStarted: () => void;
  appendDelta: (delta: string) => void;
  finishStream: (full: string, conversationId: number) => void;
  fail: (message: string) => void;
}

export const useBuddy = create<BuddyState>((set) => ({
  paused: false,
  level: "assist",
  appName: null,
  windowTitle: null,
  messages: [],
  streamingText: "",
  status: "idle",
  error: null,
  activationCount: 0,
  pinned: false,
  screenChars: null,

  activate: (p) =>
    set((s) => ({
      appName: p.app_name,
      windowTitle: p.window_title,
      paused: p.paused,
      level: p.permission_level,
      error: null,
      activationCount: s.activationCount + 1,
      screenChars: null, // fresh activation: OCR pending again
    })),

  setPaused: (paused) => set({ paused }),
  setLevel: (level) => set({ level }),
  setPinned: (pinned) => set({ pinned }),
  setScreenChars: (chars) => set({ screenChars: chars }),
  seedMessages: (messages) => set({ messages, streamingText: "", status: "idle" }),

  pushLocalUserMessage: (content) =>
    set((s) => ({ messages: [...s.messages, { role: "user", content }] })),

  generationStarted: () => set({ status: "thinking", streamingText: "", error: null }),

  appendDelta: (delta) =>
    set((s) => ({ streamingText: s.streamingText + delta, status: "thinking", error: null })),

  finishStream: (full, _conversationId) =>
    set((s) => ({
      messages: [...s.messages, { role: "assistant", content: full }],
      streamingText: "",
      status: "idle",
    })),

  fail: (message) => set({ error: message, status: "error", streamingText: "" }),
}));
