import type { APIRoute } from 'astro';
import { forwardSetCookies } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const username = String(form.get('username') || '').trim();
  const email = String(form.get('email') || '').trim();
  const password = String(form.get('password') || '');
  const display_name = String(form.get('display_name') || '').trim();

  const body: Record<string, string> = { username, email, password };
  if (display_name) body.display_name = display_name;

  const res = await fetch(`${API_BASE}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Registration failed' }));
    const msg = encodeURIComponent(data.error || 'Registration failed');
    return redirect(`/register?error=${msg}`, 303);
  }

  const response = redirect('/', 303);
  forwardSetCookies(res, response);
  return response;
};
