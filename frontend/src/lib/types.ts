export type UserPublic = {
  id: number;
  username: string;
  display_name: string | null;
  role: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
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
  last_post_at: string | null;
  created_at: string;
  updated_at: string;
  author_username: string;
  author_display_name: string | null;
};

export type Post = {
  id: number;
  thread_id: number;
  author_id: number;
  body: string;
  body_html?: string;
  created_at: string;
  updated_at: string;
  author_username: string;
  author_display_name: string | null;
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
