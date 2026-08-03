import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const target_type = String(form.get('target_type') || '').trim();
  const target_id = Number(form.get('target_id') || 0);
  const reason = String(form.get('reason') || '').trim();
  const details = String(form.get('details') || '').trim();
  const back = String(form.get('back') || '/');

  const res = await fetch(`${API_BASE}/mod/reports`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      target_type,
      target_id,
      reason,
      details: details || null,
    }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Report failed' }));
    return redirect(
      `${back}${back.includes('?') ? '&' : '?'}error=${encodeURIComponent(data.error || 'Report failed')}`,
      303,
    );
  }
  return redirect(`${back}${back.includes('?') ? '&' : '?'}reported=1`, 303);
};
