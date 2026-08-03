import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const login = String(form.get('login') || '').trim();
  const res = await fetch(`${API_BASE}/auth/password-reset`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ login }),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Failed' }));
    return redirect(`/reset-password?error=${encodeURIComponent(data.error || 'Failed')}`, 303);
  }
  return redirect('/reset-password?ok=1', 303);
};
