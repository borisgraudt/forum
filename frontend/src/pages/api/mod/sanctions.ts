import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const action = String(form.get('action') || 'create');

  if (action === 'lift') {
    const sanctionId = String(form.get('sanction_id') || '').trim();
    const res = await fetch(`${API_BASE}/mod/sanctions/${encodeURIComponent(sanctionId)}`, {
      method: 'DELETE',
      headers: {
        ...(headers.Cookie ? { Cookie: headers.Cookie } : {}),
        ...(headers['X-CSRF-Token'] ? { 'X-CSRF-Token': headers['X-CSRF-Token'] } : {}),
      },
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({ error: 'Could not lift sanction' }));
      return redirect(`/mod?error=${encodeURIComponent(data.error || 'Failed')}#sanctions`, 303);
    }
    return redirect('/mod#sanctions', 303);
  }

  const userId = String(form.get('user_id') || '').trim();
  const kind = String(form.get('kind') || '').trim();
  const reason = String(form.get('reason') || '').trim();
  const ends_at = String(form.get('ends_at') || '').trim();

  const body: Record<string, unknown> = { kind, reason: reason || null };
  if (ends_at) {
    // HTML datetime-local → ISO-ish; append Z if no timezone
    body.ends_at = ends_at.length === 16 ? `${ends_at}:00.000Z` : ends_at;
  }

  const res = await fetch(`${API_BASE}/mod/users/${encodeURIComponent(userId)}/sanctions`, {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not apply sanction' }));
    return redirect(`/mod?error=${encodeURIComponent(data.error || 'Failed')}#sanctions`, 303);
  }
  return redirect('/mod#sanctions', 303);
};
