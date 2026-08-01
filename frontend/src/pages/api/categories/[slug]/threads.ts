import type { APIRoute } from 'astro';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ params, request, redirect }) => {
  const slug = params.slug;
  if (!slug) return redirect('/', 303);

  const form = await request.formData();
  const title = String(form.get('title') || '').trim();
  const body = String(form.get('body') || '').trim();
  const cookie = request.headers.get('cookie');

  const res = await fetch(`${API_BASE}/categories/${encodeURIComponent(slug)}/threads`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(cookie ? { Cookie: cookie } : {}),
    },
    body: JSON.stringify({ title, body }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not create discussion' }));
    const msg = encodeURIComponent(data.error || 'Could not create discussion');
    return redirect(`/categories/${slug}/new?error=${msg}`, 303);
  }

  const data = await res.json();
  const threadSlug = data.thread?.slug;
  return redirect(
    threadSlug ? `/categories/${slug}/threads/${threadSlug}` : `/categories/${slug}`,
    303,
  );
};
