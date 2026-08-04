/**
 * Instant actions — no full page reload.
 * Me too / Helpful / Watch / Solve: optimistic UI + same-origin FormData
 * (BFF forwards cookies; no CORS preflight). Progressive: without JS forms still POST.
 */
(function () {
  'use strict';

  function wantsJson(headers) {
    headers = headers || {};
    headers.Accept = 'application/json';
    headers['X-Forum-Live'] = '1';
    return headers;
  }

  function setBusy(el, busy) {
    if (!el) return;
    el.disabled = !!busy;
    el.setAttribute('aria-busy', busy ? 'true' : 'false');
  }

  function parseCountLabel(text) {
    // "Me too · 3" / "Helpful · 1"
    var m = String(text || '').match(/·\s*(\d+)/);
    if (m) return parseInt(m[1], 10) || 0;
    return 0;
  }

  async function liveSubmit(form) {
    var fd = new FormData(form);
    var res = await fetch(form.action, {
      method: 'POST',
      body: fd,
      credentials: 'same-origin',
      headers: wantsJson(),
    });
    var data = null;
    try {
      data = await res.json();
    } catch (_) {}
    if (!res.ok) {
      throw new Error((data && data.error) || res.statusText || 'Request failed');
    }
    return data || {};
  }

  // ── Me too ────────────────────────────────────────────────────────────
  document.addEventListener(
    'submit',
    function (e) {
      var form = e.target;
      if (!(form instanceof HTMLFormElement) || !form.hasAttribute('data-live-me-too')) return;
      e.preventDefault();
      if (form.dataset.livePending === '1') return;

      var btn = form.querySelector('button[type="submit"]');
      var hidden = form.querySelector('input[name="action"]');
      var action = (hidden && hidden.value) || 'add';
      var prevVoted = action === 'remove';
      var prevCount = parseCountLabel(btn && btn.textContent);
      // Optimistic flip
      var nextVoted = !prevVoted;
      var nextCount = Math.max(0, prevCount + (nextVoted ? 1 : -1));
      if (hidden) hidden.value = nextVoted ? 'remove' : 'add';
      if (btn) {
        btn.classList.toggle('ad-pill--active', nextVoted);
        btn.textContent = nextCount > 0 ? 'Me too · ' + nextCount : 'Me too';
      }
      var pulse = document.querySelector('[data-me-too-count]');
      if (pulse) pulse.textContent = String(nextCount);

      form.dataset.livePending = '1';
      setBusy(btn, true);
      liveSubmit(form)
        .then(function (data) {
          var count = data.count != null ? data.count : nextCount;
          var voted = data.viewer_voted != null ? !!data.viewer_voted : nextVoted;
          if (hidden) hidden.value = voted ? 'remove' : 'add';
          if (btn) {
            btn.classList.toggle('ad-pill--active', voted);
            btn.textContent = count > 0 ? 'Me too · ' + count : 'Me too';
          }
          if (pulse) pulse.textContent = String(count);
        })
        .catch(function (err) {
          console.warn(err);
          // Revert
          if (hidden) hidden.value = prevVoted ? 'remove' : 'add';
          if (btn) {
            btn.classList.toggle('ad-pill--active', prevVoted);
            btn.textContent = prevCount > 0 ? 'Me too · ' + prevCount : 'Me too';
          }
          if (pulse) pulse.textContent = String(prevCount);
        })
        .finally(function () {
          form.dataset.livePending = '0';
          setBusy(btn, false);
        });
    },
    true,
  );

  // ── Helpful ───────────────────────────────────────────────────────────
  document.addEventListener(
    'submit',
    function (e) {
      var form = e.target;
      if (!(form instanceof HTMLFormElement) || !form.hasAttribute('data-live-helpful')) return;
      e.preventDefault();
      if (form.dataset.livePending === '1') return;

      var btn = form.querySelector('button[type="submit"]');
      var hidden = form.querySelector('input[name="action"]');
      var action = (hidden && hidden.value) || 'add';
      var prevVoted = action === 'remove';
      var prevCount = parseCountLabel(btn && btn.textContent);
      var nextVoted = !prevVoted;
      var nextCount = Math.max(0, prevCount + (nextVoted ? 1 : -1));

      function paint(voted, count) {
        if (hidden) hidden.value = voted ? 'remove' : 'add';
        if (btn) {
          btn.classList.toggle('ad-pill--active', voted);
          btn.innerHTML =
            '<span class="ad-pill__icon" aria-hidden="true">↑</span> Helpful' +
            (count > 0 ? ' · ' + count : '');
        }
      }
      paint(nextVoted, nextCount);

      form.dataset.livePending = '1';
      setBusy(btn, true);
      liveSubmit(form)
        .then(function (data) {
          paint(
            data.viewer_voted != null ? !!data.viewer_voted : nextVoted,
            data.count != null ? data.count : nextCount,
          );
        })
        .catch(function (err) {
          console.warn(err);
          paint(prevVoted, prevCount);
        })
        .finally(function () {
          form.dataset.livePending = '0';
          setBusy(btn, false);
        });
    },
    true,
  );

  // ── Watch ─────────────────────────────────────────────────────────────
  document.addEventListener(
    'submit',
    function (e) {
      var form = e.target;
      if (!(form instanceof HTMLFormElement) || !form.hasAttribute('data-live-watch')) return;
      e.preventDefault();
      if (form.dataset.livePending === '1') return;

      var btn = form.querySelector('button[type="submit"]');
      var act = form.querySelector('input[name="action"]');
      var prev = (act && act.value) === 'unwatch';
      var next = !prev;
      function paint(watching) {
        if (btn) {
          btn.textContent = watching ? 'Watching' : 'Watch';
          btn.classList.toggle('ad-pill--active', watching);
        }
        if (act) act.value = watching ? 'unwatch' : 'watch';
        form.setAttribute('data-api-method', watching ? 'DELETE' : 'POST');
      }
      paint(next);

      form.dataset.livePending = '1';
      setBusy(btn, true);
      liveSubmit(form)
        .then(function (data) {
          paint(data.watching != null ? !!data.watching : next);
        })
        .catch(function (err) {
          console.warn(err);
          paint(prev);
        })
        .finally(function () {
          form.dataset.livePending = '0';
          setBusy(btn, false);
        });
    },
    true,
  );

  // ── Solve ─────────────────────────────────────────────────────────────
  document.addEventListener(
    'submit',
    function (e) {
      var form = e.target;
      if (!(form instanceof HTMLFormElement) || !form.hasAttribute('data-live-solve')) return;
      e.preventDefault();
      if (form.dataset.livePending === '1') return;

      var btn = form.querySelector('button[type="submit"]');
      var solvedInput = form.querySelector('input[name="is_solved"]');
      var prev = solvedInput && solvedInput.value === '1';
      var next = !prev;
      function paint(solved) {
        if (solvedInput) solvedInput.value = solved ? '1' : '0';
        if (btn) {
          btn.textContent = solved ? 'Solved' : 'Mark solved';
          btn.classList.toggle('ad-pill--active', solved);
        }
        var badge = document.querySelector('[data-solved-badge]');
        if (badge) badge.hidden = !solved;
      }
      paint(next);

      form.dataset.livePending = '1';
      setBusy(btn, true);
      liveSubmit(form)
        .then(function (data) {
          var t = data.thread || data;
          paint(t.is_solved != null ? !!t.is_solved : next);
        })
        .catch(function (err) {
          console.warn(err);
          paint(prev);
        })
        .finally(function () {
          form.dataset.livePending = '0';
          setBusy(btn, false);
        });
    },
    true,
  );

  // ── Soft live pulse (tiny JSON, no HTML re-render) ────────────────────
  var root = document.querySelector('[data-thread-live]');
  if (!root) return;
  var cat = root.getAttribute('data-category');
  var thr = root.getAttribute('data-thread');
  if (!cat || !thr) return;

  var apiMeta = document.querySelector('meta[name="api-origin"]');
  var origin = (apiMeta && apiMeta.content) || '';
  var postsEl = document.querySelector('[data-post-count]');
  var lastPosts = postsEl ? parseInt(postsEl.textContent || '0', 10) : 0;
  var pulseUrl =
    origin +
    '/api/v1/categories/' +
    encodeURIComponent(cat) +
    '/threads/' +
    encodeURIComponent(thr) +
    '/pulse';

  function onPulse(p) {
    if (!p) return;
    if (p.me_too_count != null) {
      var m = document.querySelector('[data-me-too-count]');
      if (m) m.textContent = String(p.me_too_count);
      // Sync me-too label count without flipping active state
      var mtForm = document.querySelector('[data-live-me-too]');
      var mtBtn = mtForm && mtForm.querySelector('button');
      if (mtBtn && mtForm.dataset.livePending !== '1') {
        var active = mtBtn.classList.contains('ad-pill--active');
        var base = 'Me too';
        mtBtn.textContent = p.me_too_count > 0 ? base + ' · ' + p.me_too_count : base;
        mtBtn.classList.toggle('ad-pill--active', active);
      }
    }
    if (p.post_count != null && postsEl) {
      postsEl.textContent = String(p.post_count);
      if (p.post_count > lastPosts) {
        lastPosts = p.post_count;
        var banner = document.querySelector('[data-live-banner]');
        if (banner) {
          banner.hidden = false;
          var a = banner.querySelector('a');
          if (a) a.href = location.pathname + location.search;
        }
      }
    }
    if (p.is_solved != null) {
      var badge = document.querySelector('[data-solved-badge]');
      if (badge) badge.hidden = !p.is_solved;
    }
  }

  var timer = null;
  var failStreak = 0;
  function tick() {
    // Pause when tab hidden — zero wasted work
    if (document.hidden) return;
    fetch(pulseUrl, { credentials: 'omit', cache: 'no-store' })
      .then(function (r) {
        if (!r.ok) throw new Error('pulse');
        failStreak = 0;
        return r.json();
      })
      .then(onPulse)
      .catch(function () {
        failStreak++;
      });
  }

  // Adaptive interval: 1.2s active, backoff on errors, pause when hidden
  function schedule() {
    if (timer) clearInterval(timer);
    var ms = failStreak > 3 ? 5000 : 1200;
    timer = setInterval(tick, ms);
  }
  schedule();
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) tick();
  });
  // Immediate first pulse after idle (let first paint win)
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(tick, { timeout: 800 });
  } else {
    setTimeout(tick, 400);
  }
})();
