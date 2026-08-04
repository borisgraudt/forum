import { defineMiddleware } from 'astro:middleware';

/** Long-cache immutable static assets; tight security headers on HTML. */
export const onRequest = defineMiddleware(async (context, next) => {
  const res = await next();
  const path = context.url.pathname;
  const headers = new Headers(res.headers);

  // Static progressive JS / icons — content-addressed by deploy, safe to cache hard.
  if (/\.(?:js|css|svg|ico|webmanifest|woff2?)$/i.test(path) || path.startsWith('/_astro/')) {
    headers.set('Cache-Control', 'public, max-age=31536000, immutable');
  } else if (res.headers.get('content-type')?.includes('text/html')) {
    // SSR pages: revalidate; no intermediate stale personalization.
    if (!headers.has('Cache-Control')) {
      headers.set('Cache-Control', 'private, no-cache');
    }
  }

  headers.set('X-Content-Type-Options', 'nosniff');
  headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
  headers.set('X-Frame-Options', 'SAMEORIGIN');
  headers.set('Permissions-Policy', 'camera=(), microphone=(), geolocation=()');

  return new Response(res.body, {
    status: res.status,
    statusText: res.statusText,
    headers,
  });
});
