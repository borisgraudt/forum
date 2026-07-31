import type { APIRoute } from 'astro';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const name = String(form.get('name') || '').trim();
  const description = String(form.get('description') || '').trim();
  const parent_slug = String(form.get('parent_slug') || '').trim();
  const cookie = request.headers.get('cookie');

  const body: Record<string, string> = { name };
  if (description) body.description = description;
  if (parent_slug) body.parent_slug = parent_slug;

  const res = await fetch(`${API_BASE}/categories`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(cookie ? { Cookie: cookie } : {}),
    },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not create' }));
    const q = parent_slug ? `?parent=${encodeURIComponent(parent_slug)}&` : '?';
    return redirect(
      `/categories/new${q}error=${encodeURIComponent(data.error || 'Could not create')}`,
      303,
    );
  }

  const data = await res.json();
  const slug = data.category?.slug;
  // After creating a subcategory, go to parent; top-level → new community page.
  if (parent_slug) return redirect(`/categories/${parent_slug}`, 303);
  return redirect(slug ? `/categories/${slug}` : '/', 303);
};
