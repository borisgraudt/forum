import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const action = String(form.get('action') || 'update');
  const categoryId = String(form.get('category_id') || '').trim();
  const back = String(form.get('back') || '/admin#categories');

  if (!categoryId) {
    return redirect(
      `/admin?error=${encodeURIComponent('Missing category')}#categories`,
      303,
    );
  }

  if (action === 'delete') {
    const res = await fetch(`${API_BASE}/admin/categories/${encodeURIComponent(categoryId)}`, {
      method: 'DELETE',
      headers: {
        ...(headers.Cookie ? { Cookie: headers.Cookie } : {}),
        ...(headers['X-CSRF-Token'] ? { 'X-CSRF-Token': headers['X-CSRF-Token'] } : {}),
      },
    });
    if (!res.ok && res.status !== 204) {
      const data = await res.json().catch(() => ({ error: 'Delete failed' }));
      return redirect(
        `/admin?error=${encodeURIComponent(data.error || 'Delete failed')}#categories`,
        303,
      );
    }
    return redirect(back, 303);
  }

  const name = String(form.get('name') || '').trim();
  const description = String(form.get('description') || '');
  const sortRaw = String(form.get('sort_order') || '').trim();
  const body: Record<string, unknown> = {};
  if (name) body.name = name;
  body.description = description;
  if (sortRaw !== '') {
    const n = Number(sortRaw);
    if (Number.isFinite(n)) body.sort_order = n;
  }

  const res = await fetch(`${API_BASE}/admin/categories/${encodeURIComponent(categoryId)}`, {
    method: 'PATCH',
    headers,
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Update failed' }));
    return redirect(
      `/admin?error=${encodeURIComponent(data.error || 'Update failed')}#categories`,
      303,
    );
  }
  return redirect(back, 303);
};
