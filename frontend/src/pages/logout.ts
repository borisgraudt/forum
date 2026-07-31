import type { APIRoute } from 'astro';
import { forwardSetCookies } from '../lib/api';

const API_BASE = import.meta.env.PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export const POST: APIRoute = async ({ request, redirect }) => {
  const cookie = request.headers.get('cookie');
  const res = await fetch(`${API_BASE}/auth/logout`, {
    method: 'POST',
    headers: cookie ? { Cookie: cookie } : undefined,
    credentials: 'include',
  });

  const response = redirect('/', 303);
  forwardSetCookies(res, response);
  return response;
};

export const GET: APIRoute = async ({ redirect }) => redirect('/');
