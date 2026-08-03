import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug } = params;
  if (!slug || !threadSlug) return redirect('/', 303);
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const action = String(form.get('action') || 'add');
  const method = action === 'remove' ? 'DELETE' : 'POST';

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/me-too`,
    {
      method,
      headers: { Cookie: headers.Cookie || '', 'X-CSRF-Token': headers['X-CSRF-Token'] || '' },
    },
  );

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Vote failed' }));
    return redirect(`${dest}?error=${encodeURIComponent(data.error || 'Vote failed')}`, 303);
  }
  return redirect(dest, 303);
};
