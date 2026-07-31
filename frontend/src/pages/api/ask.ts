import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const category = String(form.get('category') || '').trim();
  const title = String(form.get('title') || '').trim();
  const body = String(form.get('body') || '').trim();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);

  if (!category || !title || !body) {
    return redirect('/ask?error=' + encodeURIComponent('All fields are required'), 303);
  }

  const res = await fetch(`${API_BASE}/categories/${encodeURIComponent(category)}/threads`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ title, body }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not post question' }));
    return redirect(
      `/ask?category=${encodeURIComponent(category)}&error=${encodeURIComponent(data.error || 'Could not post')}`,
      303,
    );
  }

  const data = await res.json();
  const threadSlug = data.thread?.slug;
  return redirect(
    threadSlug ? `/categories/${category}/threads/${threadSlug}` : `/categories/${category}`,
    303,
  );
};
