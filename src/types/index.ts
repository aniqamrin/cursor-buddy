export type PermissionLevel =
  | "observe"
  | "guide"
  | "assist"
  | "autopilot";

export interface SettingsDto {
  hotkey: string;
  model: string;
  autostart: boolean;
  activity_context_enabled: boolean;
  screen_context_enabled: boolean;
  permission_level: PermissionLevel;
  memory_enabled: boolean;
  first_run_completed: boolean;
}

export interface ApiKeyStatus {
  configured: boolean;
  source: "stored" | "env" | "missing";
  masked: string | null;
}

export interface ActivatePayload {
  x: number;
  y: number;
  paused: boolean;
  permission_level: PermissionLevel;
  app_name: string | null;
  window_title: string | null;
}

export interface ConversationSummary {
  id: number;
  title: string;
  created_at: string;
  message_count: number;
}

export interface MessageRow {
  id: number;
  conversation_id: number;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
}
