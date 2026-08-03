import type {
  ApiError,
  Category,
  Draft,
  PageMeta,
  Post,
  PostEdit,
  SearchHit,
  Thread,
  UserProfile,
  UserPublic,
} from './types';

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
  csrfToken?: string | null;
};

async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const headers = new Headers();
  if (opts.body !== undefined) {
    headers.set('Content-Type', 'application/json');
  }
  if (opts.cookie) {
    headers.set('Cookie', opts.cookie);
  }
  if (opts.csrfToken) {
    headers.set('X-CSRF-Token', opts.csrfToken);
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

export function readCookieValue(
  cookieHeader: string | null | undefined,
  name: string,
): string | null {
  if (!cookieHeader) return null;
  for (const part of cookieHeader.split(';')) {
    const [k, ...rest] = part.trim().split('=');
    if (k === name) return rest.join('=') || null;
  }
  return null;
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

/** Ensure a CSRF cookie/token exists for mutating form posts. */
export async function ensureCsrf(
  cookie?: string | null,
): Promise<{ token: string; setCookies: string[] }> {
  const existing = readCookieValue(cookie, 'csrf');
  if (existing) {
    return { token: existing, setCookies: [] };
  }

  const res = await fetch(`${API_BASE}/auth/csrf`, {
    headers: cookie ? { Cookie: cookie } : {},
  });
  if (!res.ok) {
    throw new ApiRequestError(res.status, 'Could not issue CSRF token');
  }
  const data = (await res.json()) as { csrf_token: string };
  const anyHeaders = res.headers as Headers & { getSetCookie?: () => string[] };
  const setCookies =
    typeof anyHeaders.getSetCookie === 'function'
      ? anyHeaders.getSetCookie()
      : (() => {
          const single = res.headers.get('set-cookie');
          return single ? [single] : [];
        })();
  return { token: data.csrf_token, setCookies };
}

export function applySetCookies(response: { headers: Headers }, setCookies: string[]) {
  for (const c of setCookies) {
    response.headers.append('Set-Cookie', c);
  }
}

/** Build Cookie + X-CSRF-Token headers for backend mutating calls from a form. */
export function mutationHeaders(
  cookie: string | null,
  form: FormData,
): { headers: Record<string, string>; csrf: string | null } {
  const csrf =
    String(form.get('csrf_token') || '').trim() || readCookieValue(cookie, 'csrf') || null;
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (cookie) headers.Cookie = cookie;
  if (csrf) headers['X-CSRF-Token'] = csrf;
  return { headers, csrf };
}

export async function listRootCategories(cookie?: string | null) {
  const data = await request<{ categories: Category[] }>('/categories', { cookie });
  return data.categories;
}

export async function getCategory(slug: string, cookie?: string | null) {
  return request<{ category: Category; children?: Category[]; parent?: Category | null }>(
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

export async function listThreads(
  categorySlug: string,
  cookie?: string | null,
  page: { limit?: number; offset?: number } = {},
) {
  const qs = new URLSearchParams();
  if (page.limit != null) qs.set('limit', String(page.limit));
  if (page.offset != null) qs.set('offset', String(page.offset));
  const q = qs.toString();
  const data = await request<{ threads: Thread[] } & PageMeta>(
    `/categories/${encodeURIComponent(categorySlug)}/threads${q ? `?${q}` : ''}`,
    { cookie },
  );
  return data;
}

export async function getThread(categorySlug: string, threadSlug: string, cookie?: string | null) {
  return request<{ thread: Thread; viewer_me_too?: boolean; first_post?: Post | null }>(
    `/categories/${encodeURIComponent(categorySlug)}/threads/${encodeURIComponent(threadSlug)}`,
    { cookie },
  );
}

export async function listPosts(
  categorySlug: string,
  threadSlug: string,
  cookie?: string | null,
  page: { limit?: number; offset?: number } = {},
) {
  const qs = new URLSearchParams();
  // Thread view loads a generous page of posts by default.
  qs.set('limit', String(page.limit ?? 100));
  if (page.offset != null) qs.set('offset', String(page.offset));
  const data = await request<{ posts: Post[] } & PageMeta>(
    `/categories/${encodeURIComponent(categorySlug)}/threads/${encodeURIComponent(threadSlug)}/posts?${qs}`,
    { cookie },
  );
  return data;
}

export async function searchThreads(
  q: string,
  cookie?: string | null,
  page: { limit?: number; offset?: number } = {},
) {
  const qs = new URLSearchParams({ q });
  if (page.limit != null) qs.set('limit', String(page.limit));
  if (page.offset != null) qs.set('offset', String(page.offset));
  return request<{ results: SearchHit[]; q: string } & PageMeta>(`/search?${qs}`, { cookie });
}

export async function getMe(cookie?: string | null): Promise<UserPublic | null> {
  try {
    const data = await request<{ user: UserPublic }>('/auth/me', { cookie });
    return data.user;
  } catch (err) {
    // Unauthenticated or transient rate-limit: treat as logged-out for SSR shells.
    if (err instanceof ApiRequestError && (err.status === 401 || err.status === 429)) return null;
    throw err;
  }
}

export function isModerator(user: UserPublic | null | undefined): boolean {
  if (!user) return false;
  return user.role === 'moderator' || user.role === 'admin';
}

export function isAdmin(user: UserPublic | null | undefined): boolean {
  return user?.role === 'admin';
}

export async function getUserProfile(username: string, cookie?: string | null) {
  const data = await request<{ profile: UserProfile }>(
    `/users/${encodeURIComponent(username)}`,
    { cookie },
  );
  return data.profile;
}

export async function listAdminUsers(
  cookie?: string | null,
  page: { limit?: number; offset?: number } = {},
) {
  const qs = new URLSearchParams();
  if (page.limit != null) qs.set('limit', String(page.limit));
  if (page.offset != null) qs.set('offset', String(page.offset));
  const q = qs.toString();
  return request<{ users: UserPublic[] } & PageMeta>(`/admin/users${q ? `?${q}` : ''}`, {
    cookie,
  });
}

export async function listAdminCategories(cookie?: string | null) {
  const data = await request<{ categories: Category[] }>('/admin/categories', { cookie });
  return data.categories;
}

export async function listPostEdits(
  categorySlug: string,
  threadSlug: string,
  postId: number,
  cookie?: string | null,
) {
  const data = await request<{ edits: PostEdit[] }>(
    `/categories/${encodeURIComponent(categorySlug)}/threads/${encodeURIComponent(threadSlug)}/posts/${postId}/edits`,
    { cookie },
  );
  return data.edits;
}

export async function listDrafts(cookie?: string | null) {
  const data = await request<{ drafts: Draft[] }>('/drafts', { cookie });
  return data.drafts;
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

/** @deprecated prefer thread.view_count */
export function fakeViews(id: number): number {
  return 12 + ((id * 37) % 480);
}
