import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const token = String(form.get('token') || '').trim();
  const password = String(form.get('password') || '');
  const res = await fetch(`${API_BASE}/auth/password-reset/confirm`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ token, password }),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Failed' }));
    return redirect(
      `/reset-password?token=${encodeURIComponent(token)}&error=${encodeURIComponent(data.error || 'Failed')}`,
      303,
    );
  }
  return redirect('/login?reset=1', 303);
};
