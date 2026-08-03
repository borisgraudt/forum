import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const action = String(form.get('action') || 'save');
  const back = String(form.get('back') || '/ask');

  if (action === 'delete') {
    const draftId = String(form.get('draft_id') || '').trim();
    if (!draftId) {
      return redirect(
        `${back}${back.includes('?') ? '&' : '?'}error=${encodeURIComponent('No draft to discard')}`,
        303,
      );
    }
    const res = await fetch(`${API_BASE}/drafts/${encodeURIComponent(draftId)}`, {
      method: 'DELETE',
      headers: {
        ...(headers.Cookie ? { Cookie: headers.Cookie } : {}),
        ...(headers['X-CSRF-Token'] ? { 'X-CSRF-Token': headers['X-CSRF-Token'] } : {}),
      },
    });
    if (!res.ok && res.status !== 204) {
      const data = await res.json().catch(() => ({ error: 'Could not discard draft' }));
      return redirect(
        `${back}${back.includes('?') ? '&' : '?'}error=${encodeURIComponent(data.error || 'Discard failed')}`,
        303,
      );
    }
    return redirect(back, 303);
  }

  const kind = String(form.get('kind') || 'thread');
  const category_slug =
    String(form.get('category_slug') || form.get('category') || '').trim() || null;
  const thread_id_raw = String(form.get('thread_id') || '').trim();
  const thread_id = thread_id_raw ? Number(thread_id_raw) : null;
  const title = String(form.get('title') || '').trim() || null;
  const body = String(form.get('body') || '');

  const res = await fetch(`${API_BASE}/drafts`, {
    method: 'PUT',
    headers,
    body: JSON.stringify({
      kind,
      category_slug,
      thread_id: Number.isFinite(thread_id) ? thread_id : null,
      title,
      body,
    }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not save draft' }));
    return redirect(
      `${back}${back.includes('?') ? '&' : '?'}error=${encodeURIComponent(data.error || 'Draft failed')}`,
      303,
    );
  }
  return redirect(`${back}${back.includes('?') ? '&' : '?'}draft=saved`, 303);
};
