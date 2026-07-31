import type { APIRoute } from 'astro';
import { forwardSetCookies } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const login = String(form.get('login') || '').trim();
  const password = String(form.get('password') || '');

  const res = await fetch(`${API_BASE}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ login, password }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Sign in failed' }));
    const msg = encodeURIComponent(data.error || 'Sign in failed');
    return redirect(`/login?error=${msg}`, 303);
  }

  const response = redirect('/', 303);
  forwardSetCookies(res, response);
  return response;
};
