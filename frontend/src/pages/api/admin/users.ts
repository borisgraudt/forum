import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const userId = String(form.get('user_id') || '');
  const role = String(form.get('role') || '').trim();
  const isActive = form.get('is_active');
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  const body: Record<string, unknown> = {};
  if (role) body.role = role;
  if (isActive === '0' || isActive === '1') body.is_active = isActive === '1';

  const res = await fetch(`${API_BASE}/admin/users/${encodeURIComponent(userId)}`, {
    method: 'PATCH',
    headers,
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Update failed' }));
    return redirect(`/admin?error=${encodeURIComponent(data.error || 'Update failed')}`, 303);
  }
  return redirect('/admin', 303);
};
