import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const id = Number(form.get('id') || 0);
  const fallback = String(form.get('to') || '/notifications');

  if (Number.isFinite(id) && id > 0) {
    await fetch(`${API_BASE}/notifications`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ ids: [id] }),
    }).catch(() => null);
  }

  // Prefer server-built path from form (SSR already knows slugs).
  const to = fallback.startsWith('/') ? fallback : '/notifications';
  return redirect(to, 303);
};
