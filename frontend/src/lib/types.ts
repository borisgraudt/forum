export type UserPublic = {
  id: number;
  username: string;
  display_name: string | null;
  bio?: string | null;
  avatar_url?: string | null;
  email_verified?: boolean;
  role: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
};

export type ProfileThreadItem = {
  id: number;
  title: string;
  slug: string;
  category_slug: string;
  category_name: string;
  created_at: string;
  post_count: number;
};

export type ProfilePostItem = {
  id: number;
  thread_id: number;
  thread_title: string;
  thread_slug: string;
  category_slug: string;
  body_preview: string;
  created_at: string;
};

export type UserProfile = {
  user: UserPublic;
  post_count: number;
  thread_count: number;
  helpful_received: number;
  reputation: number;
  recent_threads?: ProfileThreadItem[];
  recent_posts?: ProfilePostItem[];
};

export type PostEdit = {
  id: number;
  post_id: number;
  editor_id: number;
  body_before: string;
  created_at: string;
  editor_username: string;
  editor_display_name: string | null;
};

export type Category = {
  id: number;
  name: string;
  slug: string;
  description: string | null;
  sort_order: number;
  parent_id: number | null;
  created_at: string;
  updated_at: string;
  topic_count?: number;
};

export type Thread = {
  id: number;
  category_id: number;
  author_id: number;
  title: string;
  slug: string;
  is_pinned: boolean;
  is_locked: boolean;
  post_count: number;
  view_count: number;
  me_too_count?: number;
  last_post_at: string | null;
  created_at: string;
  updated_at: string;
  author_username: string;
  author_display_name: string | null;
  author_avatar_url?: string | null;
};

export type Post = {
  id: number;
  thread_id: number;
  author_id: number;
  body: string;
  body_html?: string;
  reply_to_post_id?: number | null;
  is_deleted?: boolean;
  edited_at?: string | null;
  created_at: string;
  updated_at: string;
  author_username: string;
  author_display_name: string | null;
  helpful_count?: number;
  viewer_marked_helpful?: boolean;
  author_avatar_url?: string | null;
  attachments?: Attachment[];
  embeds?: LinkEmbed[];
};

export type Draft = {
  id: number;
  user_id: number;
  kind: string;
  category_slug: string | null;
  thread_id: number | null;
  title: string | null;
  body: string;
  updated_at: string;
};

export type PageMeta = {
  total: number;
  limit: number;
  offset: number;
};

export type SearchHit = {
  thread: Thread;
  category_slug: string;
  category_name: string;
  post_id: number;
  snippet: string;
};

export type ApiError = {
  error: string;
};

export type Report = {
  id: number;
  reporter_id: number;
  reporter_username?: string;
  target_type: string;
  target_id: number;
  reason: string;
  details?: string | null;
  status: string;
  resolved_by?: number | null;
  resolved_at?: string | null;
  resolution_note?: string | null;
  created_at: string;
  updated_at: string;
};

export type UserSanction = {
  id: number;
  user_id: number;
  username?: string;
  kind: string;
  reason?: string | null;
  created_by: number;
  created_by_username?: string;
  starts_at: string;
  ends_at?: string | null;
  is_active: boolean;
  created_at: string;
};

export type AuditEntry = {
  id: number;
  actor_id?: number | null;
  actor_username?: string | null;
  action: string;
  target_type?: string | null;
  target_id?: number | null;
  meta?: string | null;
  created_at: string;
};

export type Attachment = {
  id: number;
  url: string;
  thumb_url?: string | null;
  mime: string;
  size_bytes: number;
  width?: number | null;
  height?: number | null;
  original_name?: string | null;
};

export type LinkEmbed = {
  url_hash: string;
  url: string;
  title?: string | null;
  description?: string | null;
  image_url?: string | null;
  site_name?: string | null;
  status: string;
};
