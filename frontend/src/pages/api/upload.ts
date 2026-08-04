import type { APIRoute } from 'astro';
import { readCookieValue } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

/** Proxy multipart upload to backend (avatar or post image). */
export const POST: APIRoute = async ({ request }) => {
  const cookie = request.headers.get('cookie');
  const csrf = request.headers.get('x-csrf-token') || readCookieValue(cookie, 'csrf') || '';
  const url = new URL(request.url);
  const kind = url.searchParams.get('kind') || 'post';
  const target = kind === 'avatar' ? `${API_BASE}/me/avatar` : `${API_BASE}/uploads`;

  const res = await fetch(target, {
    method: 'POST',
    headers: {
      ...(cookie ? { Cookie: cookie } : {}),
      ...(csrf ? { 'X-CSRF-Token': csrf } : {}),
    },
    body: await request.formData(),
  });

  const text = await res.text();
  return new Response(text, {
    status: res.status,
    headers: { 'Content-Type': res.headers.get('Content-Type') || 'application/json' },
  });
};
