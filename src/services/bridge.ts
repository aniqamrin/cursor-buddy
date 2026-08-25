import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActivatePayload,
  ApiKeyStatus,
  ConversationSummary,
  MessageRow,
  SettingsDto,
} from "../types";

export const api = {
  send: (text: string) => invoke<void>("chat_send", { text }),
  getSettings: () => invoke<SettingsDto>("get_settings"),
  saveSettings: (settings: SettingsDto) =>
    invoke<SettingsDto>("save_settings", { settings }),
  setApiKey: (key: string) => invoke<ApiKeyStatus>("set_api_key", { key }),
  removeApiKey: () => invoke<ApiKeyStatus>("remove_api_key"),
  apiKeyStatus: () => invoke<ApiKeyStatus>("api_key_status"),
  togglePause: () => invoke<boolean>("toggle_pause"),
  setPermissionLevel: (level: string) =>
    invoke<void>("set_permission_level", { level }),
  hideBubble: () => invoke<void>("hide_bubble"),
  setPinned: (pinned: boolean) => invoke<void>("set_bubble_pinned", { pinned }),
  showMain: () => invoke<void>("show_main"),
  quit: () => invoke<void>("quit_app"),
  clearHistory: () => invoke<number>("clear_history"),
  listConversations: () => invoke<ConversationSummary[]>("list_conversations"),
  getMessages: (conversationId: number) =>
    invoke<MessageRow[]>("get_messages", { conversationId }),
};

type Handler<T> = (payload: T) => void;

export const events = {
  activate: (h: Handler<ActivatePayload>): Promise<UnlistenFn> =>
    listen<ActivatePayload>("cb://activate", (e) => h(e.payload)),
  token: (h: Handler<string>): Promise<UnlistenFn> =>
    listen<{ delta: string }>("cb://token", (e) => h(e.payload.delta)),
  done: (h: Handler<{ conversation_id: number; content: string }>): Promise<UnlistenFn> =>
    listen<{ conversation_id: number; content: string }>("cb://done", (e) => h(e.payload)),
  error: (h: Handler<{ message: string }>): Promise<UnlistenFn> =>
    listen<{ message: string }>("cb://error", (e) => h(e.payload)),
  generationStarted: (h: Handler<void>): Promise<UnlistenFn> =>
    listen("cb://generation-started", () => h()),
  pauseChanged: (h: Handler<boolean>): Promise<UnlistenFn> =>
    listen<{ paused: boolean }>("cb://pause-changed", (e) => h(e.payload.paused)),
  permissionChanged: (h: Handler<string>): Promise<UnlistenFn> =>
    listen<{ level: string }>("cb://permission-changed", (e) => h(e.payload.level)),
};
