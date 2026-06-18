/** Thin fetch wrapper for the same-origin JSON API (cookies sent automatically). */

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown
): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    credentials: "same-origin",
  });

  const text = await res.text();
  const data = text ? JSON.parse(text) : null;

  if (!res.ok) {
    const msg = data?.error ?? `请求失败 (${res.status})`;
    throw new ApiError(res.status, msg);
  }
  return data as T;
}

export const api = {
  get: <T>(path: string) => request<T>("GET", path),
  post: <T>(path: string, body?: unknown) => request<T>("POST", path, body),
  put: <T>(path: string, body?: unknown) => request<T>("PUT", path, body),
  del: <T>(path: string) => request<T>("DELETE", path),
};

// ---- Types ----

export interface User {
  id: number;
  nickname: string;
  avatar: string;
  email: string | null;
  email_verified: boolean;
  has_password: boolean;
  created_at: string;
}

export interface Word {
  id: number;
  user_id: number;
  term: string;
  phonetic: string;
  definition: string;
  example: string;
  note: string;
  tags: string;
  created_at: string;
  updated_at: string;
}

export interface WordList {
  items: Word[];
  total: number;
  page: number;
  page_size: number;
}

export interface DictionaryResult {
  phonetic: string;
  definition: string;
  example: string;
}

export interface AiTranslationResult {
  phonetic: string;
  definition: string;
  example: string;
  note: string;
  tags: string;
}

export interface SmtpSettings {
  enabled: boolean;
  host: string;
  port: number;
  username: string;
  password: string;
  from_email: string;
  from_name: string;
  use_implicit_tls: boolean;
}

export interface OAuthSettings {
  base_url: string;
  appid: string;
  appkey: string;
}

export interface DictionarySettings {
  enabled: boolean;
  url_template: string;
}

export type AiProvider = "openai_compatible" | "claude" | "gemini";

export interface AiSettings {
  enabled: boolean;
  provider: AiProvider;
  api_endpoint: string;
  api_key: string;
  model: string;
  system_prompt: string;
}

export interface BotSettings {
  enabled: boolean;
  max_bindings_per_user: number;
  webhook_secret: string;
}

export interface AdminSettings {
  smtp: SmtpSettings;
  oauth: OAuthSettings;
  dictionary: DictionarySettings;
  ai: AiSettings;
  bot: BotSettings;
}

export interface WxBinding {
  id: number;
  user_id: number;
  external_user_id: string | null;
  binding_code: string;
  display_name: string;
  avatar: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface BotBindingList {
  items: WxBinding[];
  total: number;
  max_bindings: number;
}

export interface BotQrInfo {
  content: string;
  svg: string;
}

export interface BotConversation {
  id: number;
  user_id: number;
  binding_id: number;
  external_conversation_id: string;
  last_translated_term: string;
  last_translation_json: string;
  created_at: string;
  updated_at: string;
}

export interface BotConversationList {
  items: BotConversation[];
  total: number;
}

export interface BotMessage {
  id: number;
  conversation_id: number;
  direction: "inbound" | "outbound" | string;
  content: string;
  command: string;
  status: string;
  metadata_json: string;
  created_at: string;
}

export interface BotMessageList {
  items: BotMessage[];
  total: number;
  page: number;
  page_size: number;
}
