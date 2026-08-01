import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug, postId } = params;
  if (!slug || !threadSlug || !postId) return redirect('/', 303);

  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  // Override method semantics: form POST → API DELETE
  delete (headers as Record<string, string>)['Content-Type'];

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/posts/${encodeURIComponent(postId)}`,
    {
      method: 'DELETE',
      headers: {
        ...(headers.Cookie ? { Cookie: headers.Cookie } : {}),
        ...(headers['X-CSRF-Token'] ? { 'X-CSRF-Token': headers['X-CSRF-Token'] } : {}),
      },
    },
  );

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!res.ok && res.status !== 204) {
    const data = await res.json().catch(() => ({ error: 'Could not delete' }));
    return redirect(`${dest}?error=${encodeURIComponent(data.error || 'Could not delete')}`, 303);
  }
  return redirect(dest, 303);
};
