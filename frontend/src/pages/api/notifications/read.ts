import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

function wantsJson(request: Request) {
  const accept = request.headers.get('accept') || '';
  return accept.includes('application/json') || request.headers.get('x-forum-live') === '1';
}

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const res = await fetch(`${API_BASE}/notifications`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ ids: [] }),
  });

  if (wantsJson(request)) {
    const data = await res.json().catch(() => ({ count: 0 }));
    return new Response(JSON.stringify(data), {
      status: res.status,
      headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
    });
  }

  return redirect('/notifications', 303);
};
