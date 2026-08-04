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
  const isSolved = String(form.get('is_solved') || '1') === '1';
  const acceptedRaw = form.get('accepted_post_id');
  const accepted_post_id =
    acceptedRaw && String(acceptedRaw).trim() !== '' ? Number(acceptedRaw) : undefined;

  const res = await fetch(
    `${API_BASE}/categories/${encodeURIComponent(slug)}/threads/${encodeURIComponent(threadSlug)}/solve`,
    {
      method: 'POST',
      headers: {
        Cookie: headers.Cookie || '',
        'X-CSRF-Token': headers['X-CSRF-Token'] || '',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        is_solved: isSolved,
        accepted_post_id: Number.isFinite(accepted_post_id) ? accepted_post_id : null,
      }),
    },
  );

  if (wantsJson(request)) {
    const data = await res.json().catch(() => ({ error: 'Solve failed' }));
    return new Response(JSON.stringify(data), {
      status: res.status,
      headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
    });
  }

  const dest = `/categories/${slug}/threads/${threadSlug}`;
  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Solve failed' }));
    return redirect(`${dest}?error=${encodeURIComponent(data.error || 'Solve failed')}`, 303);
  }
  return redirect(dest, 303);
};
