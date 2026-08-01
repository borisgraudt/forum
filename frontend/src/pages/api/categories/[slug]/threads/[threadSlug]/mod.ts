import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug } = params;
  if (!slug || !threadSlug) return redirect('/', 303);

  const form = await request.formData();
  const action = String(form.get('action') || '');
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  const body: Record<string, boolean> = {};
  if (action === 'lock') body.is_locked = true;
  else if (action === 'unlock') body.is_locked = false;
  else if (action === 'pin') body.is_pinned = true;
  else if (action === 'unpin') body.is_pinned = false;
  else {
    return redirect(`/categories/${slug}/threads/${threadSlug}?error=Unknown+action`, 303);
  }

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}`,
    {
      method: 'PATCH',
      headers,
      body: JSON.stringify(body),
    },
  );

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Moderation failed' }));
    return redirect(`${dest}?error=${encodeURIComponent(data.error || 'Moderation failed')}`, 303);
  }
  return redirect(dest, 303);
};
