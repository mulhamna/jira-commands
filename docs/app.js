/* jirac landing — interactivity */
(function () {
  const D = window.JIRAC;
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
  const el = (tag, cls, html) => {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (html != null) n.innerHTML = html;
    return n;
  };

  /* ---------------- Theme (light/dark) ---------------- */
  function syncThemeToggle() {
    const dark = document.documentElement.classList.contains("dark");
    const t = $("#theme-toggle");
    if (t) t.setAttribute("aria-pressed", String(dark));
    $$("[data-theme-sun]").forEach((s) => (s.style.display = dark ? "block" : "none"));
    $$("[data-theme-moon]").forEach((s) => (s.style.display = dark ? "none" : "block"));
  }
  function initThemeToggle() {
    syncThemeToggle();
    const t = $("#theme-toggle");
    if (!t) return;
    t.addEventListener("click", () => {
      const root = document.documentElement;
      root.classList.toggle("dark");
      localStorage.setItem("theme", root.classList.contains("dark") ? "dark" : "light");
      syncThemeToggle();
    });
  }

  /* ---------------- Hero direction switcher ---------------- */
  function initHeroSwitch() {
    const saved = localStorage.getItem("jirac-hero") || "spotlight";
    apply(saved);
    $$("[data-hero-pick]").forEach((b) => {
      b.addEventListener("click", () => apply(b.dataset.heroPick));
    });
    function apply(name) {
      document.body.setAttribute("data-hero", name);
      localStorage.setItem("jirac-hero", name);
      $$("[data-hero-pick]").forEach((b) =>
        b.setAttribute("aria-pressed", String(b.dataset.heroPick === name))
      );
      // (re)start typing animation when terminal hero becomes visible
      if (name === "terminal") startTyping();
    }
  }

  /* ---------------- Copy buttons (event delegation) ---------------- */
  function initCopy() {
    document.addEventListener("click", async (e) => {
      const btn = e.target.closest("[data-copy]");
      if (!btn) return;
      let text = btn.getAttribute("data-copy");
      if (text === "@target") {
        const sel = btn.getAttribute("data-copy-target");
        const node = sel ? document.getElementById(sel) : null;
        text = node ? ("value" in node && node.value !== undefined ? node.value : node.textContent) : "";
      }
      try {
        await navigator.clipboard.writeText((text || "").trim());
        flash(btn);
      } catch (_) {
        flash(btn, true);
      }
    });
  }
  function flash(btn, fail) {
    const label = btn.querySelector("[data-copy-label]") || btn;
    const prev = label.textContent;
    btn.classList.add("is-copied");
    label.textContent = fail ? "Failed" : "Copied";
    setTimeout(() => {
      label.textContent = prev;
      btn.classList.remove("is-copied");
    }, 1300);
  }

  /* ---------------- Tabbed installer ---------------- */
  function detectOS() {
    const p = (navigator.platform || "") + " " + (navigator.userAgent || "");
    if (/Win/i.test(p)) return "windows";
    if (/Linux|X11/i.test(p) && !/Android/i.test(p)) return "linux";
    return "macos";
  }
  function initInstaller() {
    const osWrap = $("#install-os");
    const methodWrap = $("#install-methods");
    const out = $("#install-cmd");
    if (!osWrap || !methodWrap || !out) return;
    let os = detectOS();
    let method = null;

    function renderMethods() {
      methodWrap.innerHTML = "";
      const list = D.install[os];
      if (!list.find((m) => m.id === method)) method = (list.find((m) => m.recommended) || list[0]).id;
      list.forEach((m) => {
        const b = el("button", "install-pill", "");
        b.type = "button";
        b.textContent = m.label;
        if (m.recommended) {
          const dot = el("span", "install-pill__star", "★");
          b.appendChild(dot);
        }
        b.setAttribute("aria-pressed", String(m.id === method));
        b.addEventListener("click", () => {
          method = m.id;
          renderMethods();
          renderCmd();
        });
        methodWrap.appendChild(b);
      });
    }
    function renderCmd() {
      const m = D.install[os].find((x) => x.id === method);
      const cmd = m ? m.cmd : "";
      out.querySelector("[data-cmd]").textContent = cmd;
      const mirror = document.getElementById("install-cmd-text");
      if (mirror) mirror.value = cmd;
    }
    $$("[data-os]", osWrap).forEach((b) => {
      b.setAttribute("aria-pressed", String(b.dataset.os === os));
      b.addEventListener("click", () => {
        os = b.dataset.os;
        $$("[data-os]", osWrap).forEach((x) => x.setAttribute("aria-pressed", String(x.dataset.os === os)));
        renderMethods();
        renderCmd();
      });
    });
    renderMethods();
    renderCmd();

    // claw row
    const claw = $("#install-claw");
    if (claw) {
      D.clawhub.forEach((c) => {
        const row = el("div", "claw-row");
        row.innerHTML = `<span class="claw-row__label">${c.label}</span><code>${c.cmd}</code>`;
        const cp = el("button", "copy-mini", '<span data-copy-label>Copy</span>');
        cp.type = "button";
        cp.setAttribute("data-copy", c.cmd);
        row.appendChild(cp);
        claw.appendChild(row);
      });
    }
  }

  /* ---------------- Command explorer ---------------- */
  function initCommands() {
    const listWrap = $("#cmd-list");
    const search = $("#cmd-search");
    const filterWrap = $("#cmd-filters");
    const count = $("#cmd-count");
    if (!listWrap) return;
    const groups = ["All", ...Array.from(new Set(D.commands.map((c) => c.group)))];
    let active = "All";
    let q = "";

    groups.forEach((g) => {
      const b = el("button", "chip", g);
      b.type = "button";
      b.setAttribute("aria-pressed", String(g === active));
      b.addEventListener("click", () => {
        active = g;
        $$("button", filterWrap).forEach((x) => x.setAttribute("aria-pressed", String(x.textContent === active)));
        render();
      });
      filterWrap.appendChild(b);
    });

    function render() {
      const ql = q.toLowerCase();
      const rows = D.commands.filter((c) => {
        const okG = active === "All" || c.group === active;
        const okQ = !ql || (c.use + " " + c.cmd + " " + c.group).toLowerCase().includes(ql);
        return okG && okQ;
      });
      listWrap.innerHTML = "";
      if (!rows.length) {
        listWrap.appendChild(el("p", "cmd-empty", "No commands match — try a different term."));
      }
      rows.forEach((c) => {
        const row = el("div", "cmd-row");
        const left = el("div", "cmd-row__meta");
        left.innerHTML = `<span class="cmd-row__group">${c.group}</span><span class="cmd-row__use">${hl(c.use, ql)}</span>`;
        const right = el("div", "cmd-row__cmd");
        const code = el("code", "", hl(escapeHtml(c.cmd), ql));
        right.appendChild(code);
        const cp = el("button", "copy-mini", '<span data-copy-label>Copy</span>');
        cp.type = "button";
        cp.setAttribute("data-copy", c.cmd);
        right.appendChild(cp);
        row.appendChild(left);
        row.appendChild(right);
        listWrap.appendChild(row);
      });
      if (count) count.textContent = rows.length + " command" + (rows.length === 1 ? "" : "s");
    }
    function hl(str, ql) {
      if (!ql) return str;
      const i = str.toLowerCase().indexOf(ql);
      if (i < 0) return str;
      return str.slice(0, i) + '<mark>' + str.slice(i, i + ql.length) + "</mark>" + str.slice(i + ql.length);
    }
    if (search) {
      search.addEventListener("input", () => {
        q = search.value;
        render();
      });
    }
    render();
  }

  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  /* ---------------- TUI shortcuts ---------------- */
  function initShortcuts() {
    const wrap = $("#sc-grid");
    if (!wrap) return;
    const groups = ["Navigate", "Act", "View"];
    const labels = { Navigate: "Navigate", Act: "Take action", View: "Switch views" };
    groups.forEach((g) => {
      const col = el("div", "sc-col");
      col.appendChild(el("h3", "sc-col__title", labels[g]));
      const ul = el("div", "sc-col__list");
      D.shortcuts.filter((s) => s.group === g).forEach((s) => {
        const row = el("div", "sc-item");
        const keys = el("div", "sc-keys");
        s.keys.forEach((k, i) => {
          if (i) keys.appendChild(el("span", "sc-sep", "/"));
          keys.appendChild(el("kbd", "keycap", k));
        });
        row.appendChild(keys);
        row.appendChild(el("span", "sc-desc", s.desc));
        ul.appendChild(row);
      });
      col.appendChild(ul);
      wrap.appendChild(col);
    });
  }

  /* ---------------- Live theme switcher + faux TUI ---------------- */
  function initTuiTheme() {
    const stage = $("#tui-stage");
    const picker = $("#theme-picker");
    if (!stage || !picker) return;

    // build rows once
    const list = $("#tui-rows", stage);
    D.tuiRows.forEach((r, i) => {
      const row = el("div", "ftui-row" + (i === 0 ? " is-sel" : ""));
      row.innerHTML =
        `<span class="ftui-key">${r.key}</span>` +
        `<span class="ftui-badge ftui-badge--${r.st}">${r.status}</span>` +
        `<span class="ftui-sum">${r.summary}</span>`;
      list.appendChild(row);
    });

    function apply(t) {
      const c = t.c;
      Object.entries(c).forEach(([k, v]) => stage.style.setProperty("--t-" + k, v));
      stage.setAttribute("data-light", t.id === "github-light" ? "1" : "0");
    }
    let activeId = D.themes[0].id;
    D.themes.forEach((t) => {
      const b = el("button", "theme-swatch", "");
      b.type = "button";
      b.innerHTML =
        `<span class="theme-swatch__dots"><i style="background:${t.c.accent}"></i><i style="background:${t.c.green}"></i><i style="background:${t.c.yellow}"></i><i style="background:${t.c.red}"></i></span>` +
        `<span class="theme-swatch__name">${t.name}</span>`;
      b.setAttribute("aria-pressed", String(t.id === activeId));
      b.addEventListener("click", () => {
        activeId = t.id;
        apply(t);
        $$("button", picker).forEach((x, idx) => x.setAttribute("aria-pressed", String(D.themes[idx].id === activeId)));
      });
      picker.appendChild(b);
    });
    apply(D.themes[0]);
  }

  /* ---------------- Typing animation (terminal hero) ---------------- */
  let typingTimer = null;
  function startTyping() {
    const out = $("#type-out");
    if (!out) return;
    clearTimeout(typingTimer);
    const lines = [
      { t: "$ jirac issue list --project CORE", cls: "tl-cmd", speed: 28 },
      { t: "CORE-183  In Progress  Refactor auth error handling", cls: "tl-out" },
      { t: "CORE-177  To Do        Add MCP transition confirmation", cls: "tl-out" },
      { t: "CORE-171  Done         Improve TUI detail renderer", cls: "tl-out" },
      { t: "$ jirac issue transition CORE-183 --to Done", cls: "tl-cmd", speed: 28 },
      { t: "✓ CORE-183 moved to Done", cls: "tl-ok" },
    ];
    out.innerHTML = "";
    let li = 0;
    function nextLine() {
      if (li >= lines.length) {
        typingTimer = setTimeout(() => startTyping(), 4200);
        return;
      }
      const spec = lines[li];
      const lineEl = el("div", "tl " + spec.cls, "");
      out.appendChild(lineEl);
      if (spec.speed) {
        let ci = 0;
        (function type() {
          lineEl.textContent = spec.t.slice(0, ci);
          if (ci <= spec.t.length) {
            ci++;
            typingTimer = setTimeout(type, spec.speed);
          } else {
            li++;
            typingTimer = setTimeout(nextLine, 260);
          }
        })();
      } else {
        lineEl.textContent = spec.t;
        li++;
        typingTimer = setTimeout(nextLine, 90);
      }
    }
    nextLine();
  }

  /* ---------------- Scroll reveal ---------------- */
  function initReveal() {
    const els = $$("[data-reveal]");
    if (!("IntersectionObserver" in window) || matchMedia("(prefers-reduced-motion: reduce)").matches) {
      els.forEach((e) => e.classList.add("is-in"));
      return;
    }
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((en) => {
          if (en.isIntersecting) {
            en.target.classList.add("is-in");
            io.unobserve(en.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: "0px 0px -8% 0px" }
    );
    els.forEach((e) => io.observe(e));
  }

  /* ---------------- Scrollspy nav ---------------- */
  function initSpy() {
    const links = $$("[data-spy]");
    const map = new Map();
    links.forEach((l) => {
      const id = l.getAttribute("href").slice(1);
      const sec = document.getElementById(id);
      if (sec) map.set(sec, l);
    });
    if (!map.size) return;
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((en) => {
          if (en.isIntersecting) {
            links.forEach((l) => l.classList.remove("is-active"));
            const l = map.get(en.target);
            if (l) l.classList.add("is-active");
          }
        });
      },
      { rootMargin: "-45% 0px -50% 0px" }
    );
    map.forEach((_, sec) => io.observe(sec));
  }

  /* ---------------- Mobile nav ---------------- */
  function initMobileNav() {
    const btn = $("#nav-toggle");
    const panel = $("#mobile-nav");
    if (!btn || !panel) return;
    btn.addEventListener("click", () => {
      const open = panel.classList.toggle("is-open");
      btn.setAttribute("aria-expanded", String(open));
    });
    $$("a", panel).forEach((a) =>
      a.addEventListener("click", () => {
        panel.classList.remove("is-open");
        btn.setAttribute("aria-expanded", "false");
      })
    );
  }

  document.addEventListener("DOMContentLoaded", () => {
    initThemeToggle();
    initHeroSwitch();
    initCopy();
    initInstaller();
    initCommands();
    initShortcuts();
    initTuiTheme();
    initReveal();
    initSpy();
    initMobileNav();
    startTyping();
  });
})();
