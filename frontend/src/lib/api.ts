import type { ApiError, Category, Post, Thread, UserPublic } from './types';

// Prefer 127.0.0.1 over localhost: Node's fetch resolves localhost to ::1 first,
// and the API often only listens on IPv4 — each SSR hop paid ~10ms+ of delay.
const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export class ApiRequestError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

type RequestOptions = {
  method?: string;
  body?: unknown;
  cookie?: string | null;
};

async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const headers = new Headers();
  if (opts.body !== undefined) {
    headers.set('Content-Type', 'application/json');
  }
  if (opts.cookie) {
    headers.set('Cookie', opts.cookie);
  }

  const res = await fetch(`${API_BASE}${path}`, {
    method: opts.method ?? (opts.body !== undefined ? 'POST' : 'GET'),
    headers,
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
    credentials: 'include',
  });

  if (res.status === 204) {
    return undefined as T;
  }

  const text = await res.text();
  const data = text ? JSON.parse(text) : null;

  if (!res.ok) {
    const message = (data as ApiError | null)?.error || res.statusText || 'Request failed';
    throw new ApiRequestError(res.status, message);
  }

  return data as T;
}

export function getCookieHeader(request: Request): string | null {
  return request.headers.get('cookie');
}

export function forwardSetCookies(from: Response, to: { headers: Headers }): void {
  const anyHeaders = from.headers as Headers & { getSetCookie?: () => string[] };
  const cookies =
    typeof anyHeaders.getSetCookie === 'function'
      ? anyHeaders.getSetCookie()
      : (() => {
          const single = from.headers.get('set-cookie');
          return single ? [single] : [];
        })();

  for (const c of cookies) {
    to.headers.append('Set-Cookie', c);
  }
}

export async function listRootCategories(cookie?: string | null) {
  const data = await request<{ categories: Category[] }>('/categories', { cookie });
  return data.categories;
}

export async function getCategory(slug: string, cookie?: string | null) {
  return request<{ category: Category; children?: Category[] }>(
    `/categories/${encodeURIComponent(slug)}`,
    { cookie },
  );
}

export async function listChildren(slug: string, cookie?: string | null) {
  const data = await request<{ categories: Category[] }>(
    `/categories/${encodeURIComponent(slug)}/children`,
    { cookie },
  );
  return data.categories;
}

export async function createCategory(
  body: {
    name: string;
    description?: string;
    slug?: string;
    parent_slug?: string;
  },
  cookie?: string | null,
) {
  return request<{ category: Category }>('/categories', {
    method: 'POST',
    body,
    cookie,
  });
}

export async function listThreads(categorySlug: string, cookie?: string | null) {
  const data = await request<{ threads: Thread[] }>(
    `/categories/${encodeURIComponent(categorySlug)}/threads`,
    { cookie },
  );
  return data.threads;
}

export async function getThread(categorySlug: string, threadSlug: string, cookie?: string | null) {
  return request<{ thread: Thread; first_post?: Post | null }>(
    `/categories/${encodeURIComponent(categorySlug)}/threads/${encodeURIComponent(threadSlug)}`,
    { cookie },
  );
}

export async function createThread(
  categorySlug: string,
  body: { title: string; body: string; slug?: string },
  cookie?: string | null,
) {
  return request<{ thread: Thread; first_post: Post }>(
    `/categories/${encodeURIComponent(categorySlug)}/threads`,
    { method: 'POST', body, cookie },
  );
}

export async function listPosts(categorySlug: string, threadSlug: string, cookie?: string | null) {
  const data = await request<{ posts: Post[] }>(
    `/categories/${encodeURIComponent(categorySlug)}/threads/${encodeURIComponent(threadSlug)}/posts`,
    { cookie },
  );
  return data.posts;
}

export async function getMe(cookie?: string | null): Promise<UserPublic | null> {
  try {
    const data = await request<{ user: UserPublic }>('/auth/me', { cookie });
    return data.user;
  } catch (err) {
    if (err instanceof ApiRequestError && err.status === 401) return null;
    throw err;
  }
}

export function formatWhen(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function formatRelative(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const sec = Math.max(0, Math.round((Date.now() - d.getTime()) / 1000));
  if (sec < 60) return 'less than a minute ago';
  const min = Math.round(sec / 60);
  if (min === 1) return '1 minute ago';
  if (min < 60) return `${min} minutes ago`;
  const hr = Math.round(min / 60);
  if (hr === 1) return '1 hour ago';
  if (hr < 24) return `${hr} hours ago`;
  const day = Math.round(hr / 24);
  if (day === 1) return '1 day ago';
  if (day < 30) return `${day} days ago`;
  return formatWhen(iso);
}

export function authorLabel(item: {
  author_username?: string;
  author_display_name?: string | null;
}): string {
  return item.author_display_name || item.author_username || 'Member';
}

export function initials(name: string): string {
  const p = name.trim().split(/\s+/).filter(Boolean);
  if (!p.length) return '?';
  if (p.length === 1) return p[0].slice(0, 2).toUpperCase();
  return (p[0][0] + p[1][0]).toUpperCase();
}

/** Deterministic placeholder views until we track real counters. */
export function fakeViews(id: number): number {
  return 12 + ((id * 37) % 480);
}
