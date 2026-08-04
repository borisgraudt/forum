/**
 * Tiny progressive composer: markdown chips + paste-image upload.
 * Zero frameworks. ~1KB. Does nothing if form has no [data-composer].
 */
(function () {
  function $(sel, root) {
    return (root || document).querySelector(sel);
  }

  function wrap(ta, before, after) {
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const val = ta.value;
    const selected = val.slice(start, end) || 'text';
    ta.value = val.slice(0, start) + before + selected + after + val.slice(end);
    ta.focus();
    const pos = start + before.length + selected.length;
    ta.setSelectionRange(pos, pos);
    ta.dispatchEvent(new Event('input', { bubbles: true }));
  }

  async function uploadImage(file, csrf) {
    const fd = new FormData();
    fd.append('file', file);
    const res = await fetch('/api/upload?kind=post', {
      method: 'POST',
      headers: csrf ? { 'X-CSRF-Token': csrf } : {},
      body: fd,
      credentials: 'include',
    });
    if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || 'upload failed');
    return res.json();
  }

  function boot(form) {
    const ta = $('textarea[name="body"]', form);
    if (!ta) return;
    const csrf = $('input[name="csrf_token"]', form)?.value || '';
    const bar = document.createElement('div');
    bar.className = 'composer-toolbar';
    bar.setAttribute('role', 'toolbar');
    bar.innerHTML = [
      ['B', '**', '**', 'Bold'],
      ['I', '_', '_', 'Italic'],
      ['`', '`', '`', 'Code'],
      ['Link', '[', '](url)', 'Link'],
      ['• list', '\n- ', '', 'List'],
      ['code', '\n```\n', '\n```\n', 'Code block'],
    ]
      .map(
        ([label, a, b, title]) =>
          `<button type="button" class="composer-toolbar__btn" data-a="${a}" data-b="${b}" title="${title}">${label}</button>`,
      )
      .join('');
    ta.parentNode.insertBefore(bar, ta);

    bar.addEventListener('click', (e) => {
      const btn = e.target.closest('button[data-a]');
      if (!btn) return;
      wrap(ta, btn.getAttribute('data-a') || '', btn.getAttribute('data-b') || '');
    });

    const idsInput = document.createElement('input');
    idsInput.type = 'hidden';
    idsInput.name = 'attachment_ids';
    idsInput.value = '';
    form.appendChild(idsInput);

    const preview = document.createElement('div');
    preview.className = 'composer-previews';
    ta.parentNode.insertBefore(preview, ta.nextSibling);

    const ids = [];
    function addId(id, thumb) {
      ids.push(id);
      idsInput.value = ids.join(',');
      const img = document.createElement('img');
      img.src = thumb;
      img.alt = '';
      img.className = 'composer-previews__img';
      preview.appendChild(img);
    }

    ta.addEventListener('paste', async (e) => {
      const items = e.clipboardData && e.clipboardData.items;
      if (!items) return;
      for (const it of items) {
        if (it.kind === 'file' && it.type.startsWith('image/')) {
          e.preventDefault();
          const file = it.getAsFile();
          if (!file) return;
          try {
            bar.dataset.status = 'uploading';
            const data = await uploadImage(file, csrf);
            const att = data.attachment;
            // Gallery-only: linked via attachment_ids, rendered under the post.
            addId(att.id, mediaSrc(att.thumb_url || att.url));
            bar.dataset.status = 'ok';
          } catch (err) {
            bar.dataset.status = 'err';
            console.warn(err);
          }
        }
      }
    });

    // Optional file picker button
    const fileBtn = document.createElement('button');
    fileBtn.type = 'button';
    fileBtn.className = 'composer-toolbar__btn';
    fileBtn.textContent = 'Image';
    fileBtn.title = 'Upload image';
    bar.appendChild(fileBtn);
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.accept = 'image/jpeg,image/png,image/gif,image/webp';
    fileInput.hidden = true;
    form.appendChild(fileInput);
    fileBtn.addEventListener('click', () => fileInput.click());
    fileInput.addEventListener('change', async () => {
      const file = fileInput.files && fileInput.files[0];
      if (!file) return;
      try {
        const data = await uploadImage(file, csrf);
        const att = data.attachment;
        addId(att.id, mediaSrc(att.thumb_url || att.url));
      } catch (err) {
        console.warn(err);
      }
      fileInput.value = '';
    });
  }

  function mediaSrc(path) {
    if (!path) return '';
    if (/^https?:\/\//i.test(path)) return path;
    var m = document.querySelector('meta[name="api-origin"]');
    var origin = (m && m.content) || '';
    return origin + (path.charAt(0) === '/' ? path : '/' + path);
  }

  document.querySelectorAll('form[data-composer]').forEach(boot);
})();
