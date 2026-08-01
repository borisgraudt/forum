import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug } = params;
  if (!slug || !threadSlug) return redirect('/', 303);

  const form = await request.formData();
  const body = String(form.get('body') || '').trim();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/posts`,
    {
      method: 'POST',
      headers,
      body: JSON.stringify({ body }),
    },
  );

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not post reply' }));
    const msg = encodeURIComponent(data.error || 'Could not post reply');
    return redirect(`${dest}?error=${msg}`, 303);
  }

  return redirect(dest, 303);
};
