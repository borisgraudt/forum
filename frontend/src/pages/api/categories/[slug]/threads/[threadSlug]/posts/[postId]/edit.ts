import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug, postId } = params;
  if (!slug || !threadSlug || !postId) return redirect('/', 303);
  const form = await request.formData();
  const body = String(form.get('body') || '').trim();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!body) {
    return redirect(`${dest}?error=${encodeURIComponent('Body is required')}`, 303);
  }

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/posts/${encodeURIComponent(postId)}`,
    {
      method: 'PATCH',
      headers,
      body: JSON.stringify({ body }),
    },
  );

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not edit' }));
    return redirect(`${dest}?error=${encodeURIComponent(data.error || 'Could not edit')}`, 303);
  }
  return redirect(dest, 303);
};
