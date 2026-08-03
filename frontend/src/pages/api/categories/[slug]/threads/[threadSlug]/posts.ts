import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug } = params;
  if (!slug || !threadSlug) return redirect('/', 303);

  const form = await request.formData();
  const body = String(form.get('body') || '').trim();
  const replyTo = String(form.get('reply_to_post_id') || '').trim();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  const payload: { body: string; reply_to_post_id?: number } = { body };
  if (replyTo) {
    const n = Number(replyTo);
    if (Number.isFinite(n)) payload.reply_to_post_id = n;
  }

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/posts`,
    {
      method: 'POST',
      headers,
      body: JSON.stringify(payload),
    },
  );

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not post reply' }));
    const msg = encodeURIComponent(data.error || 'Could not post reply');
    return redirect(`${dest}?error=${msg}`, 303);
  }

  const draftId = String(form.get('draft_id') || '').trim();
  if (draftId) {
    await fetch(`${API_BASE}/drafts/${encodeURIComponent(draftId)}`, {
      method: 'DELETE',
      headers: {
        ...(headers.Cookie ? { Cookie: headers.Cookie } : {}),
        ...(headers['X-CSRF-Token'] ? { 'X-CSRF-Token': headers['X-CSRF-Token'] } : {}),
      },
    }).catch(() => null);
  }

  return redirect(dest, 303);
};
