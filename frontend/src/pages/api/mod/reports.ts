import type { APIRoute } from 'astro';
import { mutationHeaders } from '../../../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://127.0.0.1:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const form = await request.formData();
  const cookie = request.headers.get('cookie');
  const { headers } = mutationHeaders(cookie, form);
  const reportId = String(form.get('report_id') || '').trim();
  const status = String(form.get('status') || 'resolved').trim();
  const note = String(form.get('note') || '').trim();

  const res = await fetch(`${API_BASE}/mod/reports/${encodeURIComponent(reportId)}`, {
    method: 'PATCH',
    headers,
    body: JSON.stringify({ status, note: note || null }),
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({ error: 'Could not update report' }));
    return redirect(`/mod?error=${encodeURIComponent(data.error || 'Failed')}`, 303);
  }
  return redirect('/mod', 303);
};
