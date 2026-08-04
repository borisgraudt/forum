import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

function wantsJson(request: Request) {
  const accept = request.headers.get('accept') || '';
  return accept.includes('application/json') || request.headers.get('x-forum-live') === '1';
}

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const { slug, threadSlug } = params;
  if (!slug || !threadSlug) {
    if (wantsJson(request)) {
      return new Response(JSON.stringify({ error: 'Not found' }), {
        status: 404,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    return redirect('/', 303);
  }

  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const action = String(form.get('action') || 'watch');
  const method = action === 'unwatch' ? 'DELETE' : 'POST';

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/watch`,
    {
      method,
      headers: {
        Cookie: headers.Cookie || '',
        'X-CSRF-Token': headers['X-CSRF-Token'] || '',
      },
    },
  );

  if (wantsJson(request)) {
    const data = await res
      .json()
      .catch(() => (res.ok ? { watching: method === 'POST' } : { error: 'Watch failed' }));
    return new Response(JSON.stringify(data), {
      status: res.status,
      headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
    });
  }

  return redirect(`/categories/${slug}/threads/${threadSlug}`, 303);
};
