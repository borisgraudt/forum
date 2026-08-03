import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const display_name = String(form.get('display_name') || '');
  const bio = String(form.get('bio') || '');
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  const res = await fetch(`${API_BASE}/users/me`, {
    method: 'PATCH',
    headers,
    body: JSON.stringify({ display_name, bio }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not update profile' }));
    return redirect(`/settings/profile?error=${encodeURIComponent(data.error || 'Failed')}`, 303);
  }
  return redirect('/settings/profile?saved=1', 303);
};
