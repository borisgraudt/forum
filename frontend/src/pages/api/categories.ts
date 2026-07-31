import type { APIRoute } from 'astro';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const name = String(form.get('name') || '').trim();
  const description = String(form.get('description') || '').trim();
  const cookie = request.headers.get('cookie');

  const body: Record<string, string> = { name };
  if (description) body.description = description;

  const res = await fetch(`${API_BASE}/categories`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(cookie ? { Cookie: cookie } : {}),
    },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not create topic' }));
    const msg = encodeURIComponent(data.error || 'Could not create topic');
    return redirect(`/categories/new?error=${msg}`, 303);
  }

  const data = await res.json();
  const slug = data.category?.slug;
  return redirect(slug ? `/categories/${slug}` : '/', 303);
};
