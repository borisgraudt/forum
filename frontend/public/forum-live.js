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

  /** Snapshot form fields *before* optimistic UI mutates hidden inputs. */
  async function liveSubmit(form, fd) {
    var body = fd || new FormData(form);
    var res = await fetch(form.action, {
      method: 'POST',
      body: body,
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
      // Capture payload before optimistic flip (otherwise action is already inverted).
      var fd = new FormData(form);

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
      liveSubmit(form, fd)
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
      var prevCount = parseCountLabel(btn && (btn.textContent || ''));
      var fd = new FormData(form); // before optimistic flip
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
      liveSubmit(form, fd)
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
      var fd = new FormData(form);
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
      liveSubmit(form, fd)
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
      // Form holds the *next* action: "1" = mark solved, "0" = unsolve.
      var fd = new FormData(form);
      var currentlySolved = solvedInput && solvedInput.value === '0';
      var nextSolved = !currentlySolved;
      function paint(solved) {
        // Store the *next* action in the form (toggle)
        if (solvedInput) solvedInput.value = solved ? '0' : '1';
        if (btn) {
          btn.textContent = solved ? 'Solved' : 'Mark solved';
          btn.classList.toggle('ad-pill--active', solved);
        }
        var badge = document.querySelector('[data-solved-badge]');
        if (badge) badge.hidden = !solved;
      }
      paint(nextSolved);

      form.dataset.livePending = '1';
      setBusy(btn, true);
      liveSubmit(form, fd)
        .then(function (data) {
          var t = data.thread || data;
          paint(t.is_solved != null ? !!t.is_solved : nextSolved);
        })
        .catch(function (err) {
          console.warn(err);
          paint(currentlySolved);
        })
        .finally(function () {
          form.dataset.livePending = '0';
          setBusy(btn, false);
        });
    },
    true,
  );

  // ── Lightbox (gallery only, full-res, keyboard) ───────────────────────
  (function lightbox() {
    var items = [];
    var idx = 0;
    var root = null;
    var img = null;
    var meta = null;

    function ensure() {
      if (root) return root;
      root = document.createElement('div');
      root.className = 'lb';
      root.hidden = true;
      root.setAttribute('role', 'dialog');
      root.setAttribute('aria-modal', 'true');
      root.setAttribute('aria-label', 'Image viewer');
      root.innerHTML =
        '<button type="button" class="lb__btn lb__close" aria-label="Close">×</button>' +
        '<button type="button" class="lb__btn lb__prev" aria-label="Previous">‹</button>' +
        '<img class="lb__img" alt="" />' +
        '<button type="button" class="lb__btn lb__next" aria-label="Next">›</button>' +
        '<div class="lb__meta" aria-live="polite"></div>';
      document.body.appendChild(root);
      img = root.querySelector('.lb__img');
      meta = root.querySelector('.lb__meta');
      root.querySelector('.lb__close').addEventListener('click', close);
      root.querySelector('.lb__prev').addEventListener('click', function (e) {
        e.stopPropagation();
        show(idx - 1);
      });
      root.querySelector('.lb__next').addEventListener('click', function (e) {
        e.stopPropagation();
        show(idx + 1);
      });
      root.addEventListener('click', function (e) {
        if (e.target === root) close();
      });
      return root;
    }

    function show(i) {
      if (!items.length) return;
      idx = (i + items.length) % items.length;
      ensure();
      var it = items[idx];
      // Swap src only when changed — avoids flash
      if (img.getAttribute('src') !== it.full) {
        img.removeAttribute('src');
        img.src = it.full;
      }
      img.alt = it.alt || '';
      meta.textContent = items.length > 1 ? idx + 1 + ' / ' + items.length : '';
      root.hidden = false;
      document.body.classList.add('lb-open');
      var prev = root.querySelector('.lb__prev');
      var next = root.querySelector('.lb__next');
      var multi = items.length > 1;
      prev.hidden = !multi;
      next.hidden = !multi;
    }

    function close() {
      if (!root || root.hidden) return;
      root.hidden = true;
      document.body.classList.remove('lb-open');
      if (img) img.removeAttribute('src');
    }

    document.addEventListener('click', function (e) {
      var btn = e.target && e.target.closest && e.target.closest('[data-lightbox]');
      if (!btn) return;
      e.preventDefault();
      var gallery = btn.closest('[data-lightbox-gallery]') || document;
      items = Array.prototype.map.call(gallery.querySelectorAll('[data-lightbox]'), function (el) {
        return {
          full: el.getAttribute('data-full') || (el.querySelector('img') && el.querySelector('img').src) || '',
          alt: (el.querySelector('img') && el.querySelector('img').alt) || '',
        };
      }).filter(function (x) {
        return !!x.full;
      });
      var start = parseInt(btn.getAttribute('data-index') || '0', 10) || 0;
      // Prefer index of clicked button among filtered set
      var full = btn.getAttribute('data-full');
      for (var i = 0; i < items.length; i++) {
        if (items[i].full === full) {
          start = i;
          break;
        }
      }
      show(start);
    });

    document.addEventListener('keydown', function (e) {
      if (!root || root.hidden) return;
      if (e.key === 'Escape') close();
      else if (e.key === 'ArrowLeft') show(idx - 1);
      else if (e.key === 'ArrowRight') show(idx + 1);
    });

    // Touch swipe
    var touchX = null;
    document.addEventListener(
      'touchstart',
      function (e) {
        if (!root || root.hidden) return;
        touchX = e.changedTouches[0].clientX;
      },
      { passive: true },
    );
    document.addEventListener(
      'touchend',
      function (e) {
        if (touchX == null || !root || root.hidden) return;
        var dx = e.changedTouches[0].clientX - touchX;
        touchX = null;
        if (Math.abs(dx) < 40) return;
        if (dx > 0) show(idx - 1);
        else show(idx + 1);
      },
      { passive: true },
    );
  })();

  // ── Soft live pulse (tiny JSON, no HTML re-render) ────────────────────
  var liveRoot = document.querySelector('[data-thread-live]');
  if (!liveRoot) return;
  var cat = liveRoot.getAttribute('data-category');
  var thr = liveRoot.getAttribute('data-thread');
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

  function schedule() {
    if (timer) clearInterval(timer);
    var ms = failStreak > 3 ? 5000 : 1200;
    timer = setInterval(tick, ms);
  }
  schedule();
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) tick();
  });
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(tick, { timeout: 800 });
  } else {
    setTimeout(tick, 400);
  }
})();
