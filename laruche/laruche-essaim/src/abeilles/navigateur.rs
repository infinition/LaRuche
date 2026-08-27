//! Persistent browser control, over the LaRuche extension or over CDP.
//!
//! The older `browser_navigate` spawns a throwaway `chrome --dump-dom` per call,
//! so nothing survives between two calls: no click, no typing, no logged-in
//! session. This tool keeps one connection open for the whole run, which is what
//! makes multi-step work on a real site possible.
//!
//! Two transports, one behaviour. [`Canal`] hides which one is in use, so every
//! action below is written once:
//!
//!   - `extension`: the user's own Chrome, through the LaRuche extension. The
//!     only way to reach an already-open browser with its live sessions, since
//!     Chrome 136 refuses `--remote-debugging-port` on the default profile.
//!   - `launch`: a Chrome started by LaRuche on its own persistent profile,
//!     driven over CDP. No extension needed; sign in once and the profile keeps
//!     the cookies for later runs.
//!   - `attach`: an already running Chrome that was started with
//!     `--remote-debugging-port`, also over CDP.
//!   - `auto` (default): the extension if it is connected, otherwise a debugging
//!     port that answers, otherwise launch.
//!
//! Navigation without vision is the primary path: `read` returns a numbered map
//! of the interactive elements, and `click`/`fill` act on those numbers. The
//! screenshot is there when looking is genuinely required, and it rides back to
//! the model through `ResultatAbeille::images`.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::pont_navigateur::PontNavigateur;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

const DEFAULT_PORT: u16 = 9222;
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_CHARS: usize = 6000;

/// The page-mapping script. It tags every visible interactive element with a
/// stable `data-lr-ref` so that a later `click` or `fill` can find it again,
/// and returns one line per element. Kept as one expression so it can be handed
/// straight to Runtime.evaluate.
const SCRIPT_READ: &str = r#"
(() => {
  const SEL = 'a[href], button, input, select, textarea, summary, [role="button"], [role="link"], [role="tab"], [role="checkbox"], [role="menuitem"], [contenteditable="true"], [onclick]';
  document.querySelectorAll('[data-lr-ref]').forEach(e => e.removeAttribute('data-lr-ref'));
  const visible = el => {
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return false;
    const s = getComputedStyle(el);
    return s.visibility !== 'hidden' && s.display !== 'none' && s.opacity !== '0';
  };
  const label = el => {
    const bits = [
      el.getAttribute('aria-label'),
      el.getAttribute('placeholder'),
      el.getAttribute('title'),
      el.getAttribute('name'),
      el.value && el.type !== 'password' ? String(el.value) : '',
      (el.innerText || '').replace(/\s+/g, ' ').trim()
    ].filter(Boolean);
    return (bits[0] || '').slice(0, 80);
  };
  let n = 0;
  const rows = [];
  for (const el of document.querySelectorAll(SEL)) {
    if (!visible(el)) continue;
    n += 1;
    el.setAttribute('data-lr-ref', String(n));
    const role = el.getAttribute('role') || el.tagName.toLowerCase();
    const kind = el.type ? `${role}:${el.type}` : role;
    const dis = el.disabled ? ' [disabled]' : '';
    rows.push(`ref_${n} <${kind}>${dis} ${label(el)}`.trimEnd());
  }
  return JSON.stringify({
    url: location.href,
    title: document.title,
    count: n,
    elements: rows,
    text: (document.body ? document.body.innerText : '').replace(/\n{3,}/g, '\n\n')
  });
})()
"#;

/// The on-page indicator. A page driven by an agent should say so: this draws a
/// breathing amber frame and a small badge, and flashes whatever gets clicked
/// or filled. It lives in a shadow root with `pointer-events: none`, so it
/// cannot restyle the host page nor swallow a click meant for it.
const SCRIPT_GLOW: &str = r##"
(() => {
  const ID = '__laruche_glow__';
  const sleep = (ms) => new Promise((r) => setTimeout(r, Math.max(0, ms)));
  // Time multiplier the agent can dial with the action's `speed` param. 1 is the
  // pleasant default; higher slows everything down so it is easier to watch.
  const sp = () => (window.__lrSpeed || 1);
  // The overlay auto-hides when the agent stops touching the tab. Each run of
  // this script (one per browser action) rearms the timer; when actions stop,
  // nothing rearms it and the frame, badge and cursor fade out on their own.
  // This is how "LaRuche is no longer working here" shows without the tool
  // needing to know when the chat turn ended.
  const IDLE_MS = 12000;
  // Cursor position survives across runs (each action re-runs this IIFE), so the
  // cursor does not jump back to centre between two actions.
  let cursorX = window.__lrcx ?? window.innerWidth / 2;
  let cursorY = window.__lrcy ?? window.innerHeight / 2;

  const install = () => {
    if (document.getElementById(ID)) return;
    const host = document.createElement('div');
    host.id = ID;
    host.style.cssText = 'position:fixed;inset:0;z-index:2147483647;pointer-events:none;';
    const root = host.attachShadow({ mode: 'open' });
    root.innerHTML = `<style>
      .frame{position:fixed;inset:0;pointer-events:none;
        box-shadow:inset 0 0 0 2px rgba(245,166,35,.85), inset 0 0 26px 5px rgba(245,166,35,.30);
        animation:lr-breathe 2.4s ease-in-out infinite;}
      .badge{position:fixed;right:12px;bottom:12px;display:flex;align-items:center;gap:7px;
        padding:6px 11px;border-radius:999px;background:rgba(18,16,12,.82);color:#F5D18B;
        font:600 12px/1 ui-monospace,SFMono-Regular,Menlo,monospace;letter-spacing:.09em;
        box-shadow:0 2px 14px rgba(0,0,0,.45), 0 0 0 1px rgba(245,166,35,.35);}
      .dot{width:7px;height:7px;border-radius:50%;background:#F5A623;
        box-shadow:0 0 8px 2px rgba(245,166,35,.9);animation:lr-blink 1.2s ease-in-out infinite;}
      .pulse{position:fixed;border-radius:6px;pointer-events:none;
        box-shadow:0 0 0 2px rgba(245,166,35,.95), 0 0 18px 6px rgba(245,166,35,.55);
        animation:lr-pop .65s ease-out forwards;}
      .cursor{position:fixed;left:0;top:0;width:36px;height:36px;pointer-events:none;
        transform:translate(-200px,-200px);transition:transform 0s linear;
        filter:drop-shadow(0 2px 4px rgba(0,0,0,.65));will-change:transform;}
      .cursor svg{display:block}
      .cursor .ring{position:absolute;left:-13px;top:-13px;width:56px;height:56px;border-radius:50%;
        border:3px solid rgba(245,166,35,.9);opacity:0;transform:scale(.4);}
      .cursor.down .ring{animation:lr-press .5s ease-out;}
      .flash{position:fixed;inset:0;background:#fff;opacity:0;pointer-events:none;}
      .flash.go{animation:lr-flash .45s ease-out;}
      .hud{position:fixed;left:14px;bottom:14px;width:270px;max-width:44vw;
        background:rgba(14,12,9,.84);backdrop-filter:blur(10px);-webkit-backdrop-filter:blur(10px);
        border:1px solid rgba(245,166,35,.35);border-radius:12px;pointer-events:auto;
        box-shadow:0 8px 30px rgba(0,0,0,.45);color:#f0e6d2;overflow:hidden;
        font:12px/1.45 ui-monospace,SFMono-Regular,Menlo,monospace;opacity:.94;}
      .hud .hd{display:flex;align-items:center;gap:7px;padding:7px 10px;cursor:grab;
        background:rgba(245,166,35,.12);border-bottom:1px solid rgba(245,166,35,.2);
        user-select:none;font-weight:600;letter-spacing:.06em;color:#F5D18B;}
      .hud .hd:active{cursor:grabbing;}
      .hud .hd .d{width:7px;height:7px;border-radius:50%;background:#F5A623;
        box-shadow:0 0 7px 2px rgba(245,166,35,.9);animation:lr-blink 1.2s ease-in-out infinite;}
      .hud .hd .sp{flex:1;}
      .hud .hd .mn{cursor:pointer;opacity:.7;padding:0 4px;}
      .hud .bd{max-height:78px;overflow:auto;padding:6px 10px 4px;}
      .hud.min .bd,.hud.min .na,.hud.min .io{display:none;}
      .hud .na{max-height:118px;overflow:auto;padding:6px 10px 8px;color:#e8e0d0;
        border-top:1px solid rgba(245,166,35,.16);white-space:pre-wrap;
        font:12px/1.5 ui-sans-serif,system-ui,-apple-system,Segoe UI,sans-serif;}
      .hud .na:empty{display:none;}
      .hud .na .moi{color:#F5D18B;}
      .hud .io{display:flex;gap:6px;padding:7px 8px;border-top:1px solid rgba(245,166,35,.2);
        background:rgba(245,166,35,.06);}
      .hud .io textarea{flex:1;resize:none;height:34px;border-radius:7px;padding:7px 8px;
        border:1px solid rgba(245,166,35,.3);background:rgba(0,0,0,.35);color:#f0e6d2;
        font:12px/1.35 ui-sans-serif,system-ui,-apple-system,Segoe UI,sans-serif;outline:none;}
      .hud .io textarea:focus{border-color:rgba(245,166,35,.65);}
      .hud .io button{border:0;border-radius:7px;padding:0 11px;cursor:pointer;
        background:rgba(245,166,35,.85);color:#20180a;font:600 12px/1 ui-sans-serif,system-ui,sans-serif;}
      .hud .io button:hover{background:#F5A623;}
      .hud .ln{padding:2px 0;color:#ded6c6;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}
      .hud .ln:first-child{color:#F5D18B;}
      .hud .ln .t{color:#a2957a;margin-right:6px;}
      /* Les ascenseurs par defaut sont enormes et clairs: sur un panneau de
         270px poses sur une page quelconque, ils sautent aux yeux plus que le
         contenu. */
      .hud .bd::-webkit-scrollbar,.hud .na::-webkit-scrollbar{width:6px;}
      .hud .bd::-webkit-scrollbar-thumb,.hud .na::-webkit-scrollbar-thumb{
        background:rgba(245,166,35,.35);border-radius:3px;}
      .hud .bd::-webkit-scrollbar-track,.hud .na::-webkit-scrollbar-track{background:transparent;}
      @keyframes lr-flash{0%{opacity:0}18%{opacity:.7}100%{opacity:0}}
      @keyframes lr-breathe{0%,100%{opacity:.5}50%{opacity:1}}
      @keyframes lr-blink{0%,100%{opacity:1}50%{opacity:.25}}
      @keyframes lr-pop{0%{opacity:1;transform:scale(.97)}100%{opacity:0;transform:scale(1.05)}}
      @keyframes lr-press{0%{opacity:.9;transform:scale(.4)}100%{opacity:0;transform:scale(1.25)}}
      @media (prefers-reduced-motion:reduce){.frame,.dot,.pulse,.cursor .ring{animation:none}}
    </style>
    <div class="frame"></div>
    <div class="flash" id="lr-flash"></div>
    <div class="badge"><span class="dot"></span>LaRuche</div>
    <div class="hud" id="lr-hud">
      <div class="hd" id="lr-hud-hd"><span class="d"></span><span class="sp">LaRuche</span><span class="mn" id="lr-hud-mn">–</span></div>
      <div class="bd" id="lr-hud-bd"></div>
      <div class="na" id="lr-hud-na"></div>
      <div class="io">
        <textarea id="lr-hud-in" rows="1" placeholder="repondre a LaRuche..."></textarea>
        <button id="lr-hud-go">envoyer</button>
      </div>
    </div>
    <div class="cursor" id="lr-cur"><span class="ring"></span>
      <svg width="36" height="36" viewBox="0 0 22 22"><path d="M2 2 L2 17 L6.5 12.7 L9.4 19 L12 17.8 L9.1 11.7 L15 11.5 Z"
        fill="#F5A623" stroke="#3a2a08" stroke-width="1"/></svg></div>`;
    (document.body || document.documentElement).appendChild(host);
    const cur = root.getElementById('lr-cur');
    if (cur) cur.style.transform = `translate(${cursorX}px, ${cursorY}px)`;
    installHud(root);
  };

  // The floating panel. It says what the agent is doing, in the page itself, so
  // the human does not have to watch the chat window to follow along. It is the
  // one part of the overlay that takes pointer events (it can be dragged and
  // folded), which is why it sits inside the shadow root: `read` never sees it,
  // so the agent cannot end up reading its own commentary back.
  const installHud = (root) => {
    const hud = root.getElementById('lr-hud');
    if (!hud) return;
    // Position, folded state and log all survive across runs: this whole script
    // re-executes on every action, and a panel that jumped home each time would
    // be unusable.
    const pos = window.__lrHudPos;
    if (pos) { hud.style.left = pos.x + 'px'; hud.style.top = pos.y + 'px'; hud.style.right = 'auto'; hud.style.bottom = 'auto'; }
    if (window.__lrHudMin) hud.classList.add('min');
    renderHud(root);
    installChat(root);

    const hd = root.getElementById('lr-hud-hd');
    const mn = root.getElementById('lr-hud-mn');
    if (mn) mn.addEventListener('click', (e) => {
      e.stopPropagation();
      window.__lrHudMin = !window.__lrHudMin;
      hud.classList.toggle('min', !!window.__lrHudMin);
      mn.textContent = window.__lrHudMin ? '+' : '–';
    });
    if (hd) hd.addEventListener('pointerdown', (e) => {
      if (e.target === mn) return;
      const r = hud.getBoundingClientRect();
      const dx = e.clientX - r.left, dy = e.clientY - r.top;
      const move = (ev) => {
        const x = Math.max(0, Math.min(window.innerWidth - r.width, ev.clientX - dx));
        const y = Math.max(0, Math.min(window.innerHeight - r.height, ev.clientY - dy));
        hud.style.left = x + 'px'; hud.style.top = y + 'px';
        hud.style.right = 'auto'; hud.style.bottom = 'auto';
        window.__lrHudPos = { x, y };
      };
      const up = () => {
        window.removeEventListener('pointermove', move);
        window.removeEventListener('pointerup', up);
      };
      window.addEventListener('pointermove', move);
      window.addEventListener('pointerup', up);
      e.preventDefault();
    });
  };

  const renderHud = (root) => {
    const bd = (root || shadow()) && (root || shadow()).getElementById('lr-hud-bd');
    if (!bd) return;
    const log = window.__lrHudLog || [];
    bd.innerHTML = '';
    for (const l of log) {
      const d = document.createElement('div');
      d.className = 'ln';
      const t = document.createElement('span');
      t.className = 't';
      t.textContent = l.t;
      d.appendChild(t);
      d.appendChild(document.createTextNode(l.m));
      bd.appendChild(d);
    }
  };

  // La narration du modele, poussee par le noeud toutes les demi-secondes. On
  // n'affiche que la fin: le panneau est un coin d'oeil, pas un transcript, et
  // le chat complet est a deux metres de la dans sa propre fenetre.
  window.__larucheChat = (texte, fini) => {
    window.__lrChat = String(texte || '');
    window.__lrChatFini = !!fini;
    touch();
    renderChat(shadow());
  };

  const renderChat = (root) => {
    const r = root || shadow();
    const na = r && r.getElementById('lr-hud-na');
    if (!na) return;
    const dit = window.__lrDit || [];
    na.textContent = '';
    if (window.__lrChat) na.appendChild(document.createTextNode(window.__lrChat));
    for (const m of dit) {
      const d = document.createElement('div');
      d.className = 'moi';
      d.textContent = 'vous: ' + m;
      na.appendChild(d);
    }
    // Coller au bas: ce qui vient d'arriver est ce qu'on veut lire.
    na.scrollTop = na.scrollHeight;
  };

  // La reponse tapee dans la page. Elle est DEPOSEE ici, et le noeud la releve
  // au passage suivant: pas de canal a ouvrir, pas d'evenement a router, et le
  // meme aller-retour sert dans les deux sens.
  const envoyer = (root) => {
    const zone = (root || shadow()).getElementById('lr-hud-in');
    if (!zone) return;
    const texte = zone.value.trim();
    if (!texte) return;
    zone.value = '';
    window.__lrSorties = [...(window.__lrSorties || []), texte];
    window.__lrDit = [...(window.__lrDit || []), texte].slice(-4);
    touch();
    renderChat(root);
  };

  const installChat = (root) => {
    const zone = root.getElementById('lr-hud-in');
    const bouton = root.getElementById('lr-hud-go');
    if (bouton) bouton.addEventListener('click', () => envoyer(root));
    if (zone) {
      zone.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          envoyer(root);
        }
        // Le shadow DOM ne cloisonne pas les evenements: sans cela, taper ici
        // declencherait les raccourcis clavier de la page hote (Gmail, Notion,
        // GitHub en sont pleins).
        e.stopPropagation();
      });
      for (const t of ['keypress', 'keyup', 'input']) {
        zone.addEventListener(t, (e) => e.stopPropagation());
      }
    }
    renderChat(root);
  };

  // Called by the tool after each action. Newest first, capped, so the panel
  // stays a glance and never grows into a scrollback.
  window.__larucheHud = (msg) => {
    const now = new Date();
    const t = String(now.getHours()).padStart(2, '0') + ':' + String(now.getMinutes()).padStart(2, '0') + ':' + String(now.getSeconds()).padStart(2, '0');
    window.__lrHudLog = [{ t, m: String(msg) }, ...(window.__lrHudLog || [])].slice(0, 40);
    touch();
    renderHud(shadow());
  };

  const shadow = () => {
    const host = document.getElementById(ID);
    return host && host.shadowRoot;
  };
  const cursor = () => { const s = shadow(); return s && s.getElementById('lr-cur'); };

  // Glide the virtual cursor to a point, using a CSS transition so the motion is
  // smooth without a rAF loop. Resolves when the transition is done.
  const moveTo = async (x, y, ms) => {
    const cur = cursor();
    cursorX = x; cursorY = y;
    window.__lrcx = x; window.__lrcy = y;
    if (!cur) return;
    ms = ms * sp();
    cur.style.transitionDuration = ms + 'ms';
    cur.style.transitionTimingFunction = 'cubic-bezier(.22,.61,.36,1)';
    cur.style.transform = `translate(${x}px, ${y}px)`;
    await sleep(ms);
  };
  const centreOf = (el) => {
    const r = el.getBoundingClientRect();
    return [r.left + r.width / 2, r.top + r.height / 2];
  };

  window.__laruchePulse = (el) => {
    const s = shadow();
    if (!s || !el || !el.getBoundingClientRect) return;
    const r = el.getBoundingClientRect();
    const d = document.createElement('div');
    d.className = 'pulse';
    d.style.cssText = `left:${r.left - 3}px;top:${r.top - 3}px;width:${r.width + 6}px;height:${r.height + 6}px;`;
    s.appendChild(d);
    setTimeout(() => d.remove(), 700);
  };

  // Move the cursor onto an element and play a press, without clicking: the
  // caller fires the real click so event semantics stay exact.
  window.__larucheClickAnim = async (el) => {
    if (!el) return;
    const [x, y] = centreOf(el);
    await moveTo(x, y, 620);
    const cur = cursor();
    if (cur) { cur.classList.add('down'); setTimeout(() => cur.classList.remove('down'), 520 * sp()); }
    window.__laruchePulse(el);
    await sleep(260 * sp());
  };

  // Glide the cursor onto an element without pressing: what `hover` shows.
  window.__larucheHoverAnim = async (el) => {
    if (!el) return;
    const [x, y] = centreOf(el);
    await moveTo(x, y, 620);
    window.__laruchePulse(el);
  };

  // Type text character by character with the cursor parked on the field, using
  // the native value setter so frameworks (React, Vue) actually register it.
  window.__larucheTypeAnim = async (el, text) => {
    if (!el) return;
    const r = el.getBoundingClientRect();
    await moveTo(r.left + Math.min(24, r.width / 2), r.top + r.height / 2, 520);
    el.focus();
    // Long text should not take forever: shrink the per-char delay past 60 chars.
    const per = Math.max(16, (text.length > 60 ? 30 : 55) * sp());
    if (el.isContentEditable) {
      el.textContent = '';
      for (const ch of text) {
        el.textContent += ch;
        el.dispatchEvent(new InputEvent('input', { bubbles: true, data: ch, inputType: 'insertText' }));
        await sleep(per);
      }
    } else {
      const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      const desc = Object.getOwnPropertyDescriptor(proto, 'value');
      const setter = desc && desc.set;
      let acc = '';
      for (const ch of text) {
        acc += ch;
        if (setter) setter.call(el, acc); else el.value = acc;
        el.dispatchEvent(new Event('input', { bubbles: true }));
        await sleep(per);
      }
      el.dispatchEvent(new Event('change', { bubbles: true }));
    }
  };

  // Smoothly scroll the window and let the cursor drift with the content, so the
  // motion is legible rather than an instant jump.
  window.__larucheScrollAnim = async (targetY) => {
    const y = Math.max(0, Math.min(targetY, (document.documentElement.scrollHeight - window.innerHeight)));
    window.scrollTo({ top: y, behavior: 'smooth' });
    const cur = cursor();
    if (cur) await moveTo(cursorX, Math.min(window.innerHeight * 0.6, cursorY + 40), 400);
    await sleep(620 * sp());
  };

  // A brief white flash, played AFTER a screenshot is captured so it signals the
  // capture to the human without appearing in the image itself.
  window.__larucheFlash = () => {
    const s = shadow();
    const f = s && s.getElementById('lr-flash');
    if (!f) return;
    f.classList.remove('go');
    void f.offsetWidth; // restart the animation
    f.classList.add('go');
  };
  window.__larucheGlowOff = () => {
    clearTimeout(window.__lrIdle);
    const host = document.getElementById(ID);
    if (host) host.remove();
  };

  // Install-or-revive the overlay and (re)arm the idle fade. Called on every run.
  const touch = () => {
    install();
    const host = document.getElementById(ID);
    if (host) {
      host.style.transition = 'opacity .35s ease';
      host.style.opacity = '1';
    }
    clearTimeout(window.__lrIdle);
    window.__lrIdle = setTimeout(() => {
      const h = document.getElementById(ID);
      if (!h) return;
      h.style.transition = 'opacity .6s ease';
      h.style.opacity = '0';
      setTimeout(() => { if (h && h.parentNode) h.remove(); }, 650);
    }, IDLE_MS);
  };

  if (document.body) touch();
  else document.addEventListener('DOMContentLoaded', touch, { once: true });
})()
"##;

/// The console and network tap.
///
/// Both are read from inside the page rather than from CDP events, and that is
/// a deliberate choice: an event subscription would have to be plumbed twice,
/// once per transport, and buffered somewhere in the node between two tool
/// calls. Patching `console` and `fetch`/`XHR` into two ring buffers on the
/// page costs a few lines, behaves identically on both transports, and the
/// buffer lives exactly where the data is produced.
///
/// The trade is honest: it only sees what happened after injection, and only
/// requests the page made through fetch or XHR. Resource Timing fills the
/// second gap for everything else (images, scripts, beacons), so the network
/// view is complete even though the detail is richer for fetch and XHR.
const SCRIPT_TAP: &str = r##"
(() => {
  if (window.__lrTap) return;
  window.__lrTap = 1;
  const CAP = 200;
  const logs = window.__lrLogs = [];
  const net = window.__lrNet = [];
  const now = () => new Date().toISOString().slice(11, 23);
  const brief = (v) => {
    try {
      if (typeof v === 'string') return v;
      if (v instanceof Error) return v.stack || (v.name + ': ' + v.message);
      if (v && v.nodeName) return '<' + v.nodeName.toLowerCase() + '>';
      return JSON.stringify(v);
    } catch { return String(v); }
  };
  const push = (arr, item) => { arr.push(item); if (arr.length > CAP) arr.shift(); };

  for (const level of ['log', 'info', 'warn', 'error', 'debug']) {
    const original = console[level].bind(console);
    console[level] = (...args) => {
      push(logs, { t: now(), level, text: args.map(brief).join(' ').slice(0, 500) });
      original(...args);
    };
  }
  // Uncaught errors never reach console.error through the patch above, and they
  // are the ones worth having.
  addEventListener('error', (e) => push(logs, {
    t: now(), level: 'error',
    text: (e.message || 'error') + (e.filename ? ' @ ' + e.filename + ':' + e.lineno : '')
  }));
  addEventListener('unhandledrejection', (e) => push(logs, {
    t: now(), level: 'error', text: 'unhandled rejection: ' + brief(e.reason)
  }));

  const origFetch = window.fetch;
  if (origFetch) {
    window.fetch = async (...args) => {
      const t0 = performance.now();
      const url = String((args[0] && args[0].url) || args[0] || '');
      const method = ((args[1] && args[1].method) || (args[0] && args[0].method) || 'GET').toUpperCase();
      try {
        const res = await origFetch(...args);
        push(net, { t: now(), method, url, status: res.status, ms: Math.round(performance.now() - t0), via: 'fetch' });
        return res;
      } catch (e) {
        push(net, { t: now(), method, url, status: 0, ms: Math.round(performance.now() - t0), via: 'fetch', error: brief(e).slice(0, 120) });
        throw e;
      }
    };
  }
  const openXhr = XMLHttpRequest.prototype.open;
  const sendXhr = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (m, u, ...rest) {
    this.__lr = { method: String(m || 'GET').toUpperCase(), url: String(u || '') };
    return openXhr.call(this, m, u, ...rest);
  };
  XMLHttpRequest.prototype.send = function (...rest) {
    const meta = this.__lr;
    if (meta) {
      const t0 = performance.now();
      this.addEventListener('loadend', () => push(net, {
        t: now(), method: meta.method, url: meta.url, status: this.status,
        ms: Math.round(performance.now() - t0), via: 'xhr'
      }));
    }
    return sendXhr.call(this, ...rest);
  };
})()
"##;

/// One open CDP connection to one page.
struct Cdp {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    port: u16,
    /// Identifier returned by Page.addScriptToEvaluateOnNewDocument, kept so the
    /// indicator can be uninstalled again on `close`.
    glow_script: Option<String>,
    /// Whether the console/network tap is already registered for new documents.
    /// Registering it twice would not break anything, but it would stack a fresh
    /// copy on every action for the whole life of the session.
    tap_registre: bool,
}

impl Cdp {
    async fn connect(ws_url: &str, port: u16) -> Result<Self> {
        let (ws, _) = connect_async(ws_url).await?;
        Ok(Self {
            ws,
            next_id: 0,
            port,
            glow_script: None,
            tap_registre: false,
        })
    }

    /// Install the indicator on the current document and on every future one.
    /// Registering it for new documents is what keeps it alive across the
    /// page's own navigations, which a one-off eval would not survive.
    async fn glow_on(&mut self) {
        if self.glow_script.is_none() {
            if let Ok(res) = self
                .call(
                    "Page.addScriptToEvaluateOnNewDocument",
                    json!({ "source": SCRIPT_GLOW }),
                )
                .await
            {
                self.glow_script = res
                    .get("identifier")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
        }
        self.eval(SCRIPT_GLOW, false).await.ok();
    }

    async fn glow_off(&mut self) {
        if let Some(id) = self.glow_script.take() {
            self.call(
                "Page.removeScriptToEvaluateOnNewDocument",
                json!({ "identifier": id }),
            )
            .await
            .ok();
        }
        self.eval("window.__larucheGlowOff && window.__larucheGlowOff()", false)
            .await
            .ok();
    }

    /// Send one command and wait for the matching reply. CDP interleaves events
    /// with replies on the same socket, so anything without our id is dropped.
    async fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let payload = json!({ "id": id, "method": method, "params": params }).to_string();
        self.ws.send(Message::text(payload)).await?;

        let deadline = tokio::time::Instant::now() + CALL_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(anyhow!("CDP timeout on {method}"));
            }
            let msg = match tokio::time::timeout(remaining, self.ws.next()).await {
                Err(_) => return Err(anyhow!("CDP timeout on {method}")),
                Ok(None) => return Err(anyhow!("CDP connection closed")),
                Ok(Some(m)) => m?,
            };
            let Message::Text(txt) = msg else { continue };
            let v: Value = match serde_json::from_str(txt.as_str()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("id").and_then(Value::as_u64) != Some(id) {
                continue; // an event, or a reply to someone else
            }
            if let Some(err) = v.get("error") {
                let m = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                return Err(anyhow!("CDP error on {method}: {m}"));
            }
            return Ok(v.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    /// Evaluate JavaScript in the page and return its value.
    ///
    /// The script is wrapped in an async function, so `await` works and a value
    /// must be produced with `return`. Wrapping is explicit rather than clever:
    /// the REPL-style "last expression wins" rule surprises people often enough.
    async fn eval(&mut self, script: &str, wrap: bool) -> Result<Value> {
        let expression = if wrap {
            format!("(async () => {{ {script} }})()")
        } else {
            script.to_string()
        };
        let res = self
            .call(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "awaitPromise": true,
                    "returnByValue": true,
                    "userGesture": true
                }),
            )
            .await?;

        if let Some(ex) = res.get("exceptionDetails") {
            let text = ex
                .get("exception")
                .and_then(|e| e.get("description"))
                .and_then(Value::as_str)
                .or_else(|| ex.get("text").and_then(Value::as_str))
                .unwrap_or("script error");
            return Err(anyhow!("{text}"));
        }
        Ok(res
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    /// Poll until the document finishes loading, or give up quietly.
    async fn wait_ready(&mut self, seconds: u64) -> Result<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds.max(1));
        loop {
            if let Ok(v) = self.eval("return document.readyState", true).await {
                if v.as_str() == Some("complete") {
                    return Ok(());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

/// The live connection, whichever transport carries it.
///
/// Both arms answer the same three questions: run this script, go to this URL,
/// give me a picture. Everything else in this file is written against `Canal`
/// and therefore works identically on the user's Chrome and on a Chrome that
/// LaRuche started itself.
enum Canal {
    // Boxee: la variante CDP porte une connexion websocket entiere, l'autre
    // rien du tout, et l'ecart de taille se paie sur chaque valeur du type.
    Cdp(Box<Cdp>),
    /// The user's browser, reached through the extension. Stateless here: the
    /// bridge owns the socket, so nothing to hold on to.
    Extension,
}

impl Canal {
    fn nom(&self) -> &'static str {
        match self {
            Canal::Cdp(_) => "CDP",
            Canal::Extension => "extension",
        }
    }

    async fn eval(&mut self, script: &str, wrap: bool) -> Result<Value> {
        match self {
            Canal::Cdp(cdp) => cdp.eval(script, wrap).await,
            Canal::Extension => {
                let expression = if wrap {
                    format!("(async () => {{ {script} }})()")
                } else {
                    script.to_string()
                };
                let res = PontNavigateur::global()
                    .appeler("eval", json!({ "script": expression }))
                    .await?;
                Ok(res.get("value").cloned().unwrap_or(Value::Null))
            }
        }
    }

    async fn navigate(&mut self, url: &str) -> Result<()> {
        match self {
            Canal::Cdp(cdp) => {
                cdp.call("Page.navigate", json!({ "url": url })).await?;
                cdp.wait_ready(15).await.ok();
                Ok(())
            }
            Canal::Extension => {
                PontNavigateur::global()
                    .appeler("navigate", json!({ "url": url }))
                    .await?;
                Ok(())
            }
        }
    }

    /// Returns the capture as base64 PNG.
    async fn screenshot(&mut self) -> Result<String> {
        match self {
            Canal::Cdp(cdp) => {
                let res = cdp
                    .call(
                        "Page.captureScreenshot",
                        json!({ "format": "png", "captureBeyondViewport": false }),
                    )
                    .await?;
                res.get("data")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("Chrome returned no image data"))
            }
            Canal::Extension => {
                let res = PontNavigateur::global()
                    .appeler("screenshot", json!({}))
                    .await?;
                res.get("data")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("The extension returned no image data"))
            }
        }
    }

    async fn glow_on(&mut self) {
        match self {
            Canal::Cdp(cdp) => cdp.glow_on().await,
            Canal::Extension => {
                // The extension keeps the script and re-injects it after each
                // navigation, the same job Page.addScriptToEvaluateOnNewDocument
                // does on the CDP side.
                PontNavigateur::global()
                    .appeler("glow", json!({ "on": true, "script": SCRIPT_GLOW }))
                    .await
                    .ok();
            }
        }
    }

    async fn glow_off(&mut self) {
        match self {
            Canal::Cdp(cdp) => cdp.glow_off().await,
            Canal::Extension => {
                PontNavigateur::global()
                    .appeler("glow", json!({ "on": false }))
                    .await
                    .ok();
            }
        }
    }

    /// Install the console and network tap on this document and on every future
    /// one. Registering it for new documents matters more here than it does for
    /// the indicator: a page that navigates would otherwise come back mute, and
    /// the agent would read an empty console and conclude there was no error.
    async fn tap_on(&mut self) {
        match self {
            Canal::Cdp(cdp) if !cdp.tap_registre => {
                cdp.call(
                    "Page.addScriptToEvaluateOnNewDocument",
                    json!({ "source": SCRIPT_TAP }),
                )
                .await
                .ok();
                cdp.tap_registre = true;
            }
            Canal::Cdp(_) => {}
            Canal::Extension => {
                PontNavigateur::global()
                    .appeler("tap", json!({ "script": SCRIPT_TAP }))
                    .await
                    .ok();
            }
        }
        self.eval(SCRIPT_TAP, false).await.ok();
    }

    /// A raw DevTools command. This exists for the one thing `eval` genuinely
    /// cannot do: synthesise a real key press or mouse move. Events dispatched
    /// from page script carry `isTrusted: false`, which native controls and a
    /// good number of sites ignore outright, so a typed Enter would silently do
    /// nothing. `Input.*` events are indistinguishable from a human's.
    async fn input(&mut self, method: &str, params: Value) -> Result<Value> {
        match self {
            Canal::Cdp(cdp) => cdp.call(method, params).await,
            Canal::Extension => {
                PontNavigateur::global()
                    .appeler("cdp", json!({ "methode": method, "params": params }))
                    .await
            }
        }
    }

    /// Push one line into the on-page panel. Best effort throughout: the panel
    /// is a comfort for the human, never a reason to fail an action.
    async fn hud(&mut self, message: &str) {
        let js = format!(
            "window.__larucheHud && window.__larucheHud({})",
            serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into())
        );
        self.eval(&js, false).await.ok();
    }

    /// Every open tab, so the agent can look beyond its own.
    async fn list_tabs(&mut self) -> Result<Value> {
        match self {
            Canal::Cdp(cdp) => {
                let list: Vec<Value> = reqwest::Client::new()
                    .get(format!("http://127.0.0.1:{}/json/list", cdp.port))
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await?
                    .json()
                    .await?;
                let tabs: Vec<Value> = list
                    .into_iter()
                    .filter(|t| t.get("type").and_then(Value::as_str) == Some("page"))
                    .map(|t| {
                        json!({
                            "tabId": t.get("id").and_then(Value::as_str).unwrap_or(""),
                            "title": t.get("title").and_then(Value::as_str).unwrap_or(""),
                            "url": t.get("url").and_then(Value::as_str).unwrap_or(""),
                        })
                    })
                    .collect();
                Ok(json!({ "tabs": tabs }))
            }
            Canal::Extension => PontNavigateur::global().appeler("tabs", json!({})).await,
        }
    }

    /// Adopt an existing tab as the one being driven.
    async fn select_tab(&mut self, id: &Value) -> Result<Value> {
        match self {
            Canal::Cdp(cdp) => {
                let want = id.as_str().map(str::to_string).unwrap_or_else(|| id.to_string());
                let list: Vec<Value> = reqwest::Client::new()
                    .get(format!("http://127.0.0.1:{}/json/list", cdp.port))
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await?
                    .json()
                    .await?;
                let target = list.iter().find(|t| {
                    t.get("id").and_then(Value::as_str) == Some(want.as_str())
                });
                let Some(target) = target else {
                    return Err(anyhow!("no tab {want} (list them with the tabs action)"));
                };
                let ws = target
                    .get("webSocketDebuggerUrl")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("that tab exposes no debugger URL"))?;
                let mut fresh = Cdp::connect(ws, cdp.port).await?;
                fresh.call("Page.enable", json!({})).await.ok();
                fresh.call("Runtime.enable", json!({})).await.ok();
                let url = target.get("url").and_then(Value::as_str).unwrap_or("").to_string();
                **cdp = fresh;
                Ok(json!({ "tabId": want, "url": url }))
            }
            Canal::Extension => {
                PontNavigateur::global()
                    .appeler("select", json!({ "tabId": id }))
                    .await
            }
        }
    }

    /// Cheap liveness probe: a socket looks fine until it is actually used.
    async fn est_vivant(&mut self) -> bool {
        match self {
            Canal::Cdp(_) => self.eval("return 1", true).await.is_ok(),
            Canal::Extension => PontNavigateur::global().est_connecte().await,
        }
    }
}

// --------------------------- le compagnon de page ---------------------------
//
// Le panneau de la page ne se contente plus de nommer les actions: il porte la
// narration du modele et une zone pour lui repondre. Deux choix expliquent la
// forme de ce qui suit.
//
// Un seul aller-retour sert les deux sens. Toutes les demi-secondes, le noeud
// evalue un script qui POUSSE la narration et REMONTE ce que l'utilisateur a
// tape. L'alternative, `Runtime.addBinding` plus un evenement route jusqu'au
// noeud, demandait une plomberie par transport et une pompe a evenements du
// cote CDP, ou rien ne lit la socket entre deux appels.
//
// La reponse devient un STEER, pas un message. Le mecanisme existe deja, il est
// fait pour l'intervention humaine en cours de route, et il atterrit dans la
// session. Rien de nouveau a router.

/// Ce que le modele est en train de dire, et ce qu'il faut en pousser.
#[derive(Default)]
struct Narration {
    texte: String,
    sale: bool,
    fini: bool,
}

static NARRATION: OnceLock<std::sync::Mutex<Narration>> = OnceLock::new();
/// Le canal de pilotage de la session en cours. Une seule, la plus recente: le
/// panneau vit dans UNE page, et faire remonter sa reponse vers deux sessions
/// simultanees n'aurait pas de sens.
static STEER: OnceLock<std::sync::Mutex<Option<tokio::sync::mpsc::Sender<String>>>> =
    OnceLock::new();
static GLOW_ACTIF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static RELEVE_LANCEE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Longueur de narration gardee. Le panneau montre la fin d'une phrase en
/// cours, pas un transcript: le chat complet est dans sa propre fenetre.
const NARRATION_MAX: usize = 700;

fn narration() -> &'static std::sync::Mutex<Narration> {
    NARRATION.get_or_init(|| std::sync::Mutex::new(Narration::default()))
}

fn steer() -> &'static std::sync::Mutex<Option<tokio::sync::mpsc::Sender<String>>> {
    STEER.get_or_init(|| std::sync::Mutex::new(None))
}

/// Le noeud declare le canal de pilotage du tour qui commence.
pub fn brancher_pilotage(tx: tokio::sync::mpsc::Sender<String>) {
    if let Ok(mut g) = steer().lock() {
        *g = Some(tx);
    }
    if let Ok(mut n) = narration().lock() {
        *n = Narration::default();
    }
}

/// Fin du tour: la page garde ce qui est affiche, mais une reponse tapee apres
/// coup ne part plus dans le vide, elle est signalee comme non transmise.
pub fn debrancher_pilotage() {
    if let Ok(mut g) = steer().lock() {
        *g = None;
    }
}

/// Un evenement du tour en cours, pour le panneau de la page.
///
/// Appele a CHAQUE jeton, donc volontairement minuscule: on empile dans un
/// tampon, et c'est la releve qui parle au navigateur, deux fois par seconde au
/// plus. Un eval par jeton serait un aller-retour websocket par jeton.
pub fn narrer(evenement: &crate::evenements::ChatEvent) {
    use crate::evenements::ChatEvent;
    if !GLOW_ACTIF.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Ok(mut n) = narration().lock() else {
        return;
    };
    match evenement {
        ChatEvent::Token { text } => {
            n.texte.push_str(text);
            let compte = n.texte.chars().count();
            if compte > NARRATION_MAX * 2 {
                n.texte = n.texte.chars().skip(compte - NARRATION_MAX).collect();
            }
            n.sale = true;
            n.fini = false;
        }
        ChatEvent::Done { full_response } => {
            let compte = full_response.chars().count();
            n.texte = full_response
                .chars()
                .skip(compte.saturating_sub(NARRATION_MAX))
                .collect();
            n.sale = true;
            n.fini = true;
        }
        ChatEvent::Error { message } => {
            n.texte = format!("erreur: {message}");
            n.sale = true;
            n.fini = true;
        }
        _ => {}
    }
    drop(n);
    lancer_releve();
}

/// Le script d'un passage: il pousse la narration ET rapporte ce qui a ete tape.
fn script_releve(texte: &str, fini: bool, pousser: bool) -> String {
    format!(
        r#"(() => {{
             if ({pousser} && window.__larucheChat) window.__larucheChat({texte}, {fini});
             const r = window.__lrSorties || [];
             window.__lrSorties = [];
             return JSON.stringify(r);
           }})()"#,
        texte = serde_json::to_string(texte).unwrap_or_else(|_| String::from("\"\"")),
    )
}

/// Demarre la releve, une seule fois pour la vie du processus.
///
/// Sans runtime sous la main, on ne lance rien ET on ne consomme pas le drapeau:
/// `narrer` est appele depuis la boucle d'evenements du chat, mais rien
/// n'empeche un test ou un chemin futur de l'appeler ailleurs, et un
/// `tokio::spawn` hors runtime panique.
fn lancer_releve() {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    if RELEVE_LANCEE.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if !GLOW_ACTIF.load(std::sync::atomic::Ordering::Relaxed) {
                continue;
            }
            let (texte, fini, sale) = match narration().lock() {
                Ok(mut n) => {
                    let sale = n.sale;
                    n.sale = false;
                    (n.texte.clone(), n.fini, sale)
                }
                Err(_) => continue,
            };
            // `try_lock`: pendant qu'une action est en cours, le panneau montre
            // deja cette action, et attendre le verrou ne ferait que retarder
            // l'action elle-meme.
            let recues: Vec<String> = {
                let Ok(mut garde) = session().try_lock() else {
                    continue;
                };
                let Some(canal) = garde.as_mut() else {
                    continue;
                };
                match canal.eval(&script_releve(&texte, fini, sale), false).await {
                    Ok(v) => serde_json::from_str(v.as_str().unwrap_or("[]")).unwrap_or_default(),
                    Err(_) => continue,
                }
            };
            if recues.is_empty() {
                continue;
            }
            let tx = steer().lock().ok().and_then(|g| g.clone());
            for message in recues {
                match &tx {
                    Some(tx) => {
                        let _ = tx.try_send(message);
                    }
                    // Hors tour, il n'y a rien a piloter. Le dire dans le
                    // panneau vaut mieux que d'avaler la phrase en silence.
                    None => {
                        let apercu: String = message.chars().take(40).collect();
                        let js = format!(
                            "window.__larucheHud && window.__larucheHud({})",
                            serde_json::to_string(&format!("hors tour, non transmis: {apercu}"))
                                .unwrap_or_else(|_| String::from("\"\""))
                        );
                        if let Ok(mut g) = session().try_lock() {
                            if let Some(c) = g.as_mut() {
                                c.eval(&js, false).await.ok();
                            }
                        }
                    }
                }
            }
        }
    });
}

/// The process-wide session. One browser, reused across tool calls.
static SESSION: OnceLock<Mutex<Option<Canal>>> = OnceLock::new();

fn session() -> &'static Mutex<Option<Canal>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

fn chrome_paths() -> Vec<&'static str> {
    if cfg!(windows) {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ]
    } else {
        vec![
            "google-chrome",
            "chromium-browser",
            "chromium",
            "microsoft-edge",
        ]
    }
}

/// Where the LaRuche-owned browser keeps its profile. Under the user's config
/// directory so cookies survive reboots, with a temp fallback for the rare
/// system that exposes no home.
fn profil_navigateur(port: u16) -> std::path::PathBuf {
    let base = std::env::var_os("LARUCHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs_config().map(|d| d.join("laruche")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("navigateur").join(format!("profil-{port}"))
}

fn dirs_config() -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(std::path::PathBuf::from)
                    .map(|h| h.join(".config"))
            })
    }
}

fn find_chrome() -> Option<String> {
    chrome_paths()
        .into_iter()
        .find(|p| std::path::Path::new(p).exists() || which::which(p).is_ok())
        .map(str::to_string)
}

/// Ask the debugging endpoint for a page target, creating one if needed.
async fn discover_page(port: u16, url: Option<&str>) -> Result<String> {
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");

    let list: Vec<Value> = client
        .get(format!("{base}/json/list"))
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .json()
        .await?;

    let existing = list
        .iter()
        .find(|t| t.get("type").and_then(Value::as_str) == Some("page"))
        .and_then(|t| t.get("webSocketDebuggerUrl").and_then(Value::as_str))
        .map(str::to_string);

    if let Some(ws) = existing {
        return Ok(ws);
    }

    // No page open: ask Chrome for a fresh tab.
    let target = url.unwrap_or("about:blank");
    let created: Value = client
        .put(format!("{base}/json/new?{}", urlencoding::encode(target)))
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .json()
        .await?;

    created
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Chrome did not return a debugger URL for the new tab"))
}

async fn port_alive(port: u16) -> bool {
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/json/version"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Start a private Chrome and wait for its debugging port to answer.
async fn launch_chrome(port: u16, headless: bool) -> Result<()> {
    let Some(bin) = find_chrome() else {
        return Err(anyhow!(
            "No Chrome, Chromium or Edge found. Install one, or start your own with \
             --remote-debugging-port={port} and use mode \"attach\"."
        ));
    };

    // A persistent profile, not a temporary one: signing in to a site once must
    // still count on the next run. Chrome 136 and later ignore the debugging
    // port on the default profile, so a separate directory is required anyway.
    let profile = profil_navigateur(port);
    let mut cmd = Command::new(bin);
    cmd.arg(format!("--remote-debugging-port={port}"))
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .arg("about:blank")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if headless {
        cmd.arg("--headless=new");
    }
    // Detached on purpose: the browser must outlive this single tool call.
    cmd.kill_on_drop(false);
    cmd.spawn()?;

    for _ in 0..40 {
        if port_alive(port).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(anyhow!(
        "Chrome was started but port {port} never answered."
    ))
}

/// Make sure a live session exists, opening one if this is the first call.
async fn ensure_session(
    guard: &mut Option<Canal>,
    mode: &str,
    port: u16,
    headless: bool,
    glow: bool,
    url: Option<&str>,
) -> Result<()> {
    if let Some(canal) = guard.as_mut() {
        let reusable = match (&*canal, mode) {
            // A CDP session bound to another port cannot serve this request.
            (Canal::Cdp(cdp), _) if cdp.port != port => false,
            (Canal::Cdp(_), "extension") => false,
            (Canal::Extension, "attach") | (Canal::Extension, "launch") => false,
            _ => true,
        };
        if reusable && canal.est_vivant().await {
            GLOW_ACTIF.store(glow, std::sync::atomic::Ordering::Relaxed);
            if glow {
                canal.glow_on().await;
            }
            canal.tap_on().await;
            return Ok(());
        }
        *guard = None;
    }

    let extension_prete = PontNavigateur::global().est_connecte().await;
    let mut canal = match mode {
        "extension" => {
            if !extension_prete {
                return Err(anyhow!(
                    "The LaRuche extension is not connected. Install it in Chrome and enable it, \
                     or use mode \"launch\" for a browser started by LaRuche."
                ));
            }
            Canal::Extension
        }
        "attach" => {
            if !port_alive(port).await {
                return Err(anyhow!(
                    "Nothing is listening on port {port}. Start your browser with \
                     --remote-debugging-port={port} and --user-data-dir=<some dir>, or use \
                     mode \"launch\", or install the LaRuche extension."
                ));
            }
            Canal::Cdp(Box::new(connect_cdp(port, url).await?))
        }
        "launch" => {
            if !port_alive(port).await {
                launch_chrome(port, headless).await?;
            }
            Canal::Cdp(Box::new(connect_cdp(port, url).await?))
        }
        // auto: the user's own browser first, then anything already listening,
        // and only then a browser of our own.
        _ => {
            if extension_prete {
                Canal::Extension
            } else {
                if !port_alive(port).await {
                    launch_chrome(port, headless).await?;
                }
                Canal::Cdp(Box::new(connect_cdp(port, url).await?))
            }
        }
    };

    GLOW_ACTIF.store(glow, std::sync::atomic::Ordering::Relaxed);
    if glow {
        canal.glow_on().await;
    }
    // The tap goes on regardless of `glow`: console and network are diagnostics,
    // not decoration, and a clean screenshot is no reason to stop recording.
    canal.tap_on().await;
    *guard = Some(canal);
    Ok(())
}

async fn connect_cdp(port: u16, url: Option<&str>) -> Result<Cdp> {
    let ws_url = discover_page(port, url).await?;
    let mut cdp = Cdp::connect(&ws_url, port).await?;
    cdp.call("Page.enable", json!({})).await.ok();
    cdp.call("Runtime.enable", json!({})).await.ok();
    Ok(cdp)
}

/// CDP modifier bitmask.
const MOD_ALT: u32 = 1;
const MOD_CTRL: u32 = 2;
const MOD_META: u32 = 4;
const MOD_SHIFT: u32 = 8;

/// One physical key, described the way `Input.dispatchKeyEvent` wants it.
struct Touche {
    key: String,
    code: String,
    vk: u32,
    /// The character the press produces, empty for a key that types nothing.
    text: String,
}

/// Parse `"Enter"`, `"a"`, `"Control+a"`, `"ctrl+shift+Tab"` into modifiers plus
/// the key itself. The names are matched case-insensitively because a model will
/// write `"enter"` as often as `"Enter"`, and being strict about it only ever
/// produces a silent no-op.
fn parse_touche(spec: &str) -> Option<(u32, Touche)> {
    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }
    // Split on '+', but a trailing '+' is the plus key itself, not a separator.
    let mut parts: Vec<&str> = spec.split('+').collect();
    if spec.ends_with('+') && parts.len() > 1 {
        parts.pop();
        parts.push("+");
    }
    let nom = parts.pop()?;
    let mut modifiers = 0;
    for m in parts {
        match m.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= MOD_CTRL,
            "shift" => modifiers |= MOD_SHIFT,
            "alt" | "option" => modifiers |= MOD_ALT,
            "meta" | "cmd" | "command" | "win" | "super" => modifiers |= MOD_META,
            // An unknown modifier is a typo, and guessing past it would press
            // something the caller did not ask for.
            _ => return None,
        }
    }
    Some((modifiers, touche_nommee(nom, modifiers)?))
}

fn touche_nommee(nom: &str, modifiers: u32) -> Option<Touche> {
    let t = |key: &str, code: &str, vk: u32, text: &str| Touche {
        key: key.to_string(),
        code: code.to_string(),
        vk,
        // A key chorded with Ctrl or Meta types nothing: sending its text anyway
        // is how "Ctrl+A" ends up inserting a literal "a" in the field.
        text: if modifiers & (MOD_CTRL | MOD_META) != 0 {
            String::new()
        } else {
            text.to_string()
        },
    };
    let n = nom.trim();
    let known = match n.to_ascii_lowercase().as_str() {
        "enter" | "return" => t("Enter", "Enter", 13, "\r"),
        "tab" => t("Tab", "Tab", 9, "\t"),
        "escape" | "esc" => t("Escape", "Escape", 27, ""),
        "backspace" => t("Backspace", "Backspace", 8, "\u{8}"),
        "delete" | "del" => t("Delete", "Delete", 46, ""),
        "space" => t(" ", "Space", 32, " "),
        "arrowup" | "up" => t("ArrowUp", "ArrowUp", 38, ""),
        "arrowdown" | "down" => t("ArrowDown", "ArrowDown", 40, ""),
        "arrowleft" | "left" => t("ArrowLeft", "ArrowLeft", 37, ""),
        "arrowright" | "right" => t("ArrowRight", "ArrowRight", 39, ""),
        "home" => t("Home", "Home", 36, ""),
        "end" => t("End", "End", 35, ""),
        "pageup" => t("PageUp", "PageUp", 33, ""),
        "pagedown" => t("PageDown", "PageDown", 34, ""),
        "insert" => t("Insert", "Insert", 45, ""),
        _ => {
            // Function keys, then any single printable character.
            if let Some(num) = n
                .strip_prefix(['f', 'F'])
                .and_then(|d| d.parse::<u32>().ok())
                .filter(|d| (1..=12).contains(d))
            {
                let name = format!("F{num}");
                return Some(t(&name, &name, 111 + num, ""));
            }
            let mut chars = n.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            let code = if c.is_ascii_alphabetic() {
                format!("Key{}", c.to_ascii_uppercase())
            } else if c.is_ascii_digit() {
                format!("Digit{c}")
            } else {
                String::new()
            };
            let vk = c.to_ascii_uppercase() as u32;
            return Some(t(&c.to_string(), &code, vk, &c.to_string()));
        }
    };
    Some(known)
}

/// The modifier keys themselves, so a site listening for `keydown` on Control
/// sees it held rather than only reading the bitmask on the final key.
fn touches_modificatrices(modifiers: u32) -> Vec<Touche> {
    let mut v = Vec::new();
    let mut add = |key: &str, code: &str, vk: u32| {
        v.push(Touche {
            key: key.into(),
            code: code.into(),
            vk,
            text: String::new(),
        })
    };
    if modifiers & MOD_CTRL != 0 {
        add("Control", "ControlLeft", 17);
    }
    if modifiers & MOD_SHIFT != 0 {
        add("Shift", "ShiftLeft", 16);
    }
    if modifiers & MOD_ALT != 0 {
        add("Alt", "AltLeft", 18);
    }
    if modifiers & MOD_META != 0 {
        add("Meta", "MetaLeft", 91);
    }
    v
}

fn evenement_touche(kind: &str, t: &Touche, modifiers: u32) -> Value {
    let mut e = json!({
        "type": kind,
        "key": t.key,
        "code": t.code,
        "windowsVirtualKeyCode": t.vk,
        "nativeVirtualKeyCode": t.vk,
        "modifiers": modifiers,
    });
    if kind == "keyDown" && !t.text.is_empty() {
        e["text"] = json!(t.text);
        e["unmodifiedText"] = json!(t.text);
    }
    e
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max).collect();
    format!(
        "{head}\n\n...(truncated, {} chars total)",
        text.chars().count()
    )
}

pub struct Browser;

#[async_trait]
impl Abeille for Browser {
    fn nom(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Drive a real browser across several steps. The session stays open between calls, so a \
         multi-step flow on one site works. Actions: navigate, read, find, click, fill, key, \
         hover, scroll, wait, eval, screenshot, console, network, back, forward, tabs, select, \
         close. Never pretend to have done one of these without calling it. Typical loop: \
         navigate, then read for a numbered map of the page, then click or fill by ref number, \
         no screenshot needed. Refs come only from the latest read or find, are reset by every \
         read and lost on navigation, so read again after navigating or after the page changes. \
         Submit a form with key Enter rather than hunting for the button; open a dropdown menu \
         with hover, which click does not do; on a page too large to read whole, use find to get \
         only the refs that match a wording. When something has not appeared yet, wait for it \
         instead of reading in a loop. When the user points at a tab they ALREADY \
         have open (\"my Dealabs tab\", \"the page I'm on\"), do NOT navigate, which would load \
         the site fresh in your own tab: call tabs to list every open tab, pick the matching \
         tabId, select it, then read. navigate is only for opening a NEW page. Use \
         eval to read data out of the page or do anything without a dedicated action; use \
         screenshot only when you must see the actual rendering. To debug a page that misbehaves, \
         console shows what it logged and network what it requested. This tool needs approval, so \
         call it DIRECTLY, never wrapped in tool_call or run_script; approving once covers the \
         rest of the session."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["navigate", "read", "find", "click", "fill", "key", "hover", "scroll", "wait", "eval", "screenshot", "console", "network", "back", "forward", "tabs", "select", "close"],
                    "description": "navigate: go to url. read: numbered map of interactive elements plus page text. find: the same map filtered to what matches text, for a page too big to read whole. click/fill: act on a ref from read. key: press a real key (Enter, Tab, Escape, Arrow*, Control+a), optionally in a ref. hover: move the mouse onto a ref, which is what opens menus that click does not. scroll: move the page (or a ref) up/down. wait: block until text or a selector appears. eval: run JavaScript. screenshot: capture the page. console: what the page logged. network: what the page requested. back/forward: move in history. tabs: list every open browser tab. select: drive one of those tabs by tab_id. close: end the session."
                },
                "url": { "type": "string", "description": "For navigate" },
                "ref": { "type": "integer", "description": "Element number from read, without the ref_ prefix. For scroll, optional: scrolls that element into view. For key, optional: focuses that element before pressing." },
                "text": { "type": "string", "description": "For fill: the value to put in the field. For find and wait: the wording to look for. For network: keep only URLs containing it." },
                "key": { "type": "string", "description": "For key: Enter, Tab, Escape, Backspace, Delete, Space, Home, End, PageUp, PageDown, ArrowUp/Down/Left/Right, F1-F12, or a single character. Prefix with Control+, Shift+, Alt+ or Meta+ to chord." },
                "repeat": { "type": "integer", "description": "For key: press it this many times, default 1, max 50" },
                "hold_ms": { "type": "integer", "description": "For key: hold it down this long before releasing, default 0" },
                "selector": { "type": "string", "description": "For wait: a CSS selector to wait for, an alternative to text" },
                "timeout": { "type": "integer", "description": "For wait: seconds before giving up, default 15" },
                "level": { "type": "string", "description": "For console: keep only this level (log, info, warn, error, debug)" },
                "limit": { "type": "integer", "description": "For console and network: how many entries to return, default 40" },
                "tab_id": { "description": "For select: a tabId from the tabs action (a number in extension mode, a string in launch/attach mode)." },
                "direction": { "type": "string", "enum": ["up", "down", "top", "bottom"], "description": "For scroll, default down" },
                "amount": { "type": "integer", "description": "For scroll up/down: pixels, default one viewport" },
                "script": { "type": "string", "description": "For eval: JavaScript wrapped in an async function, so await works and you must `return` a value" },
                "mode": { "type": "string", "enum": ["auto", "extension", "launch", "attach"], "description": "auto (default): the user's own Chrome through the LaRuche extension when it is connected, otherwise a browser started by LaRuche. extension: require the user's Chrome, with its open tabs and logged-in sessions. launch: a browser started by LaRuche on its own persistent profile, no extension needed. attach: an existing browser started with --remote-debugging-port." },
                "port": { "type": "integer", "description": "Debugging port, default 9222" },
                "headless": { "type": "boolean", "description": "For launch: run without a window, default false" },
                "glow": { "type": "boolean", "description": "Amber frame and badge shown on the page while the agent drives it, and a flash on each element touched. Default true. Set false for a clean screenshot." },
                "animate": { "type": "boolean", "description": "Move a visible cursor to each target, type text character by character, and scroll smoothly, so a human can follow. Default true; needs glow on. Set false to act instantly." },
                "speed": { "type": "number", "description": "Animation time multiplier, default 1. Above 1 slows the cursor, typing and scroll down so they are easier to watch." },
                "max_chars": { "type": "integer", "description": "Cap on returned page text, default 6000" }
            },
            "required": ["action"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        // In attach mode this drives the user's own logged-in profile and can
        // submit forms, so it is never a read-only operation.
        NiveauDanger::NeedsApproval
    }

    async fn executer(&self, args: Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let action = args["action"].as_str().unwrap_or_default();
        let mode = args["mode"].as_str().unwrap_or("auto");
        let port = args["port"].as_u64().unwrap_or(DEFAULT_PORT as u64) as u16;
        let headless = args["headless"].as_bool().unwrap_or(false);
        let glow = args["glow"].as_bool().unwrap_or(true);
        // Cursor motion, progressive typing and smooth scroll, on by default so a
        // human watching can follow. It rides on the indicator, so it only shows
        // when glow is on; turn either off for speed or a clean screenshot.
        let animate = glow && args["animate"].as_bool().unwrap_or(true);
        let speed = args["speed"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0);
        let max_chars = args["max_chars"].as_u64().unwrap_or(DEFAULT_MAX_CHARS as u64) as usize;

        let mut guard = session().lock().await;

        if action == "close" {
            let had = match guard.as_mut() {
                Some(canal) => {
                    canal.glow_off().await;
                    true
                }
                None => false,
            };
            *guard = None;
            GLOW_ACTIF.store(false, std::sync::atomic::Ordering::Relaxed);
            return Ok(ResultatAbeille::ok(if had {
                "Browser session closed, indicator removed. The browser process itself is left running."
            } else {
                "No browser session was open."
            }));
        }

        let url = args["url"].as_str();
        if let Err(e) = ensure_session(&mut guard, mode, port, headless, glow, url).await {
            return Ok(ResultatAbeille::err(e.to_string()));
        }
        let canal = guard.as_mut().expect("session just ensured");

        // Announce the action in the on-page panel BEFORE running it. During a
        // 600ms cursor glide the human should already know what it is aiming at;
        // a line written afterwards would always arrive once it no longer helps.
        if glow {
            let court = |s: &str| -> String {
                let s = s.replace('\n', " ");
                if s.chars().count() > 28 {
                    format!("{}...", s.chars().take(28).collect::<String>())
                } else {
                    s
                }
            };
            let r = args["ref"].as_u64();
            let detail = match action {
                "navigate" => court(url.unwrap_or("")),
                "click" | "hover" => r.map(|r| format!("ref_{r}")).unwrap_or_default(),
                "fill" => format!(
                    "ref_{} = {}",
                    r.unwrap_or(0),
                    court(args["text"].as_str().unwrap_or(""))
                ),
                "key" => court(args["key"].as_str().or(args["text"].as_str()).unwrap_or("")),
                "scroll" => match r {
                    Some(r) => format!("to ref_{r}"),
                    None => args["direction"].as_str().unwrap_or("down").to_string(),
                },
                "find" | "wait" => court(
                    args["text"]
                        .as_str()
                        .or(args["selector"].as_str())
                        .unwrap_or(""),
                ),
                "select" => format!("tab {}", args["tab_id"]),
                _ => String::new(),
            };
            let ligne = format!("{action} {detail}");
            canal.hud(ligne.trim_end()).await;
        }

        let outcome: Result<ResultatAbeille> = match action {
            "navigate" => {
                let Some(url) = url else {
                    return Ok(ResultatAbeille::err("navigate needs a 'url'."));
                };
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Ok(ResultatAbeille::err("URL must start with http:// or https://"));
                }
                match canal.navigate(url).await {
                    Err(e) => Err(e),
                    Ok(_) => {
                        if glow {
                            // The registered script covers the new document, but
                            // it runs before <body> exists on a slow page.
                            canal.eval(SCRIPT_GLOW, false).await.ok();
                        }
                        let title = canal
                            .eval("return document.title", true)
                            .await
                            .ok()
                            .and_then(|v| v.as_str().map(str::to_string))
                            .unwrap_or_default();
                        Ok(ResultatAbeille::ok(format!(
                            "Loaded {url}\nTitle: {title}\nTransport: {}",
                            canal.nom()
                        )))
                    }
                }
            }

            "read" => match canal.eval(SCRIPT_READ, false).await {
                Err(e) => Err(e),
                Ok(v) => {
                    let raw = v.as_str().unwrap_or("{}");
                    let snap: Value = serde_json::from_str(raw).unwrap_or(json!({}));
                    let elements = snap["elements"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default();
                    let body = snap["text"].as_str().unwrap_or("");
                    Ok(ResultatAbeille::ok(format!(
                        "URL: {}\nTitle: {}\n\nInteractive elements ({}):\n{}\n\nPage text:\n{}",
                        snap["url"].as_str().unwrap_or(""),
                        snap["title"].as_str().unwrap_or(""),
                        snap["count"].as_u64().unwrap_or(0),
                        truncate(&elements, max_chars),
                        truncate(body, max_chars)
                    )))
                }
            },

            "click" => {
                let Some(r) = args["ref"].as_u64() else {
                    return Ok(ResultatAbeille::err("click needs a 'ref' number from read."));
                };
                let script = format!(
                    r#"window.__lrSpeed = {speed};
                       const el = document.querySelector('[data-lr-ref="{r}"]');
                       if (!el) return 'MISSING';
                       el.scrollIntoView({{block:'center', behavior:'instant'}});
                       if ({animate} && window.__larucheClickAnim) {{
                         await window.__larucheClickAnim(el);
                       }} else {{
                         if (window.__laruchePulse) window.__laruchePulse(el);
                         await new Promise(r => setTimeout(r, 90));
                       }}
                       el.click();
                       return 'clicked ' + (el.tagName || '');"#
                );
                match canal.eval(&script, true).await {
                    Err(e) => Err(e),
                    Ok(v) if v.as_str() == Some("MISSING") => Ok(ResultatAbeille::err(format!(
                        "No element ref_{r} on this page. Run read again: refs are reset on every read and lost on navigation."
                    ))),
                    Ok(v) => Ok(ResultatAbeille::ok(
                        v.as_str().unwrap_or("clicked").to_string(),
                    )),
                }
            }

            "fill" => {
                let Some(r) = args["ref"].as_u64() else {
                    return Ok(ResultatAbeille::err("fill needs a 'ref' number from read."));
                };
                let text = args["text"].as_str().unwrap_or_default();
                // Assigning `.value` directly is invisible to React, Angular and
                // friends: they listen on their own descriptor. Going through the
                // native setter and firing the events is what actually registers.
                let script = format!(
                    r#"window.__lrSpeed = {speed};
                       const el = document.querySelector('[data-lr-ref="{r}"]');
                       if (!el) return 'MISSING';
                       const v = {value};
                       el.scrollIntoView({{block:'center', behavior:'instant'}});
                       if ({animate} && window.__larucheTypeAnim) {{
                         await window.__larucheTypeAnim(el, v);
                       }} else {{
                         if (window.__laruchePulse) window.__laruchePulse(el);
                         el.focus();
                         if (el.isContentEditable) {{
                           el.textContent = v;
                         }} else {{
                           const proto = el instanceof HTMLTextAreaElement
                             ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
                           const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
                           setter.call(el, v);
                         }}
                         el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                         el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                       }}
                       return 'filled ref_{r}';"#,
                    value = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into()),
                );
                match canal.eval(&script, true).await {
                    Err(e) => Err(e),
                    Ok(v) if v.as_str() == Some("MISSING") => Ok(ResultatAbeille::err(format!(
                        "No element ref_{r} on this page. Run read again."
                    ))),
                    Ok(v) => Ok(ResultatAbeille::ok(
                        v.as_str().unwrap_or("filled").to_string(),
                    )),
                }
            }

            "eval" => {
                let Some(script) = args["script"].as_str() else {
                    return Ok(ResultatAbeille::err("eval needs a 'script'."));
                };
                match canal.eval(script, true).await {
                    Err(e) => Ok(ResultatAbeille::err(format!("JavaScript error: {e}"))),
                    Ok(v) => {
                        let rendered = match &v {
                            Value::String(s) => s.clone(),
                            Value::Null => "(no value returned)".to_string(),
                            other => serde_json::to_string_pretty(other).unwrap_or_default(),
                        };
                        Ok(ResultatAbeille::ok(truncate(&rendered, max_chars)))
                    }
                }
            }

            "screenshot" => match canal.screenshot().await {
                Err(e) => Err(e),
                Ok(data) => {
                    // Flash AFTER the capture so the cue is seen but not in the image.
                    if animate {
                        canal
                            .eval("window.__larucheFlash && window.__larucheFlash()", false)
                            .await
                            .ok();
                    }
                    let mut out = ResultatAbeille::ok("Screenshot of the current page.");
                    out.images = vec![data];
                    Ok(out)
                }
            },

            "scroll" => {
                let direction = args["direction"].as_str().unwrap_or("down");
                let behavior = if animate { "smooth" } else { "auto" };
                let script = if let Some(r) = args["ref"].as_u64() {
                    format!(
                        r#"const el = document.querySelector('[data-lr-ref="{r}"]');
                           if (!el) return 'MISSING';
                           el.scrollIntoView({{block:'center', behavior:'{behavior}'}});
                           if ({animate}) await new Promise(x => setTimeout(x, 500));
                           return 'scrolled to ref_{r}';"#
                    )
                } else {
                    // No explicit amount pages by ~90% of the viewport, the little
                    // overlap keeping the reader oriented the way a human scrolls.
                    // The animated helper adds smoothness and a cursor drift; the
                    // instant branch is the fallback when the indicator is off.
                    let amount = args["amount"].as_i64().unwrap_or(0);
                    let target = match direction {
                        "top" => "0".to_string(),
                        "bottom" => "document.documentElement.scrollHeight".to_string(),
                        "up" | "down" => {
                            let sign = if direction == "up" { "-" } else { "" };
                            let by = if amount > 0 {
                                format!("{sign}{amount}")
                            } else {
                                format!("{sign}Math.round(window.innerHeight*0.9)")
                            };
                            format!("window.scrollY + ({by})")
                        }
                        _ => {
                            return Ok(ResultatAbeille::err(
                                "scroll direction must be up, down, top or bottom.".to_string(),
                            ))
                        }
                    };
                    format!(
                        r#"window.__lrSpeed = {speed};
                           const target = {target};
                           if ({animate} && window.__larucheScrollAnim) {{
                             await window.__larucheScrollAnim(target);
                           }} else {{
                             window.scrollTo({{ top: target, behavior: '{behavior}' }});
                           }}
                           return 'scrolled {direction}';"#
                    )
                };
                match canal.eval(&script, true).await {
                    Err(e) => Err(e),
                    Ok(v) if v.as_str() == Some("MISSING") => Ok(ResultatAbeille::err(format!(
                        "No element ref_{} on this page. Run read again.",
                        args["ref"].as_u64().unwrap_or(0)
                    ))),
                    Ok(v) => Ok(ResultatAbeille::ok(
                        v.as_str().unwrap_or("scrolled").to_string(),
                    )),
                }
            }

            "tabs" => match canal.list_tabs().await {
                Err(e) => Err(e),
                Ok(v) => {
                    let empty = vec![];
                    let tabs = v.get("tabs").and_then(Value::as_array).unwrap_or(&empty);
                    let driving = v.get("driving").cloned().unwrap_or(Value::Null);
                    let lignes: Vec<String> = tabs
                        .iter()
                        .map(|t| {
                            let id = t.get("tabId").cloned().unwrap_or(Value::Null);
                            let ours = t.get("ours").and_then(Value::as_bool).unwrap_or(false);
                            let mark = if ours || Some(&id) == Some(&driving) {
                                " (driven)"
                            } else {
                                ""
                            };
                            // Window tag, so two tabs of the same site in different
                            // Chrome windows are told apart; * marks the focused one.
                            let win = match t.get("windowId") {
                                Some(w) if !w.is_null() => {
                                    let f = t
                                        .get("windowFocused")
                                        .and_then(Value::as_bool)
                                        .unwrap_or(false);
                                    format!(" [win {}{}]", w, if f { "*" } else { "" })
                                }
                                _ => String::new(),
                            };
                            format!(
                                "tab {}{}{}  {}  {}",
                                id,
                                win,
                                mark,
                                t.get("title").and_then(Value::as_str).unwrap_or(""),
                                t.get("url").and_then(Value::as_str).unwrap_or("")
                            )
                        })
                        .collect();
                    let wins = v.get("windowCount").and_then(Value::as_u64);
                    let entete = match wins {
                        Some(n) if n > 1 => format!(
                            "{} open tab(s) across {} windows (win* is focused). \
                             Use select with a tabId to drive one.",
                            tabs.len(),
                            n
                        ),
                        _ => format!(
                            "{} open tab(s). Use select with a tabId to drive one.",
                            tabs.len()
                        ),
                    };
                    Ok(ResultatAbeille::ok(format!("{entete}\n{}", lignes.join("\n"))))
                }
            },

            "select" => {
                let id = args.get("tab_id").cloned().unwrap_or(Value::Null);
                if id.is_null() {
                    return Ok(ResultatAbeille::err(
                        "select needs 'tab_id', a value from the tabs action.".to_string(),
                    ));
                }
                match canal.select_tab(&id).await {
                    Err(e) => Err(e),
                    Ok(v) => {
                        if glow {
                            canal.eval(SCRIPT_GLOW, false).await.ok();
                        }
                        Ok(ResultatAbeille::ok(format!(
                            "Now driving tab {} ({}). Run read to map it.",
                            v.get("tabId").cloned().unwrap_or(Value::Null),
                            v.get("url").and_then(Value::as_str).unwrap_or("")
                        )))
                    }
                }
            }

            "key" => {
                let Some(spec) = args["key"].as_str().or_else(|| args["text"].as_str()) else {
                    return Ok(ResultatAbeille::err(
                        "key needs a 'key', for instance \"Enter\", \"Tab\", \"Escape\", \
                         \"ArrowDown\" or \"Control+a\"."
                            .to_string(),
                    ));
                };
                let Some((modifiers, touche)) = parse_touche(spec) else {
                    return Ok(ResultatAbeille::err(format!(
                        "Unrecognised key '{spec}'. Use a single character, a named key \
                         (Enter, Tab, Escape, Backspace, Delete, Space, Home, End, PageUp, \
                         PageDown, Arrow*, F1-F12), optionally prefixed with Control+, \
                         Shift+, Alt+ or Meta+."
                    )));
                };
                // Focusing first is what makes "press Enter in the search box"
                // work: without it the event lands on whatever had focus, which
                // after a fill is usually right, and after a read is usually not.
                if let Some(r) = args["ref"].as_u64() {
                    let focus = format!(
                        r#"const el = document.querySelector('[data-lr-ref="{r}"]');
                           if (!el) return 'MISSING';
                           el.scrollIntoView({{block:'center', behavior:'instant'}});
                           el.focus();
                           return 'ok';"#
                    );
                    match canal.eval(&focus, true).await {
                        Err(e) => return Ok(ResultatAbeille::err(format!("{e}"))),
                        Ok(v) if v.as_str() == Some("MISSING") => {
                            return Ok(ResultatAbeille::err(format!(
                                "No element ref_{r} on this page. Run read again."
                            )))
                        }
                        Ok(_) => {}
                    }
                }
                let repeat = args["repeat"].as_u64().unwrap_or(1).clamp(1, 50);
                let hold_ms = args["hold_ms"].as_u64().unwrap_or(0).min(10_000);
                let mods = touches_modificatrices(modifiers);
                let mut held = 0;
                for m in &mods {
                    held |= match m.key.as_str() {
                        "Control" => MOD_CTRL,
                        "Shift" => MOD_SHIFT,
                        "Alt" => MOD_ALT,
                        _ => MOD_META,
                    };
                    canal
                        .input("Input.dispatchKeyEvent", evenement_touche("rawKeyDown", m, held))
                        .await
                        .ok();
                }
                let mut erreur = None;
                for _ in 0..repeat {
                    if let Err(e) = canal
                        .input(
                            "Input.dispatchKeyEvent",
                            evenement_touche("keyDown", &touche, modifiers),
                        )
                        .await
                    {
                        erreur = Some(e);
                        break;
                    }
                    if hold_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(hold_ms)).await;
                    }
                    canal
                        .input(
                            "Input.dispatchKeyEvent",
                            evenement_touche("keyUp", &touche, modifiers),
                        )
                        .await
                        .ok();
                }
                // Modifiers are released in reverse, and unconditionally: leaving
                // Control stuck down would poison every later interaction, the
                // user's own included.
                for m in mods.iter().rev() {
                    canal
                        .input("Input.dispatchKeyEvent", evenement_touche("keyUp", m, 0))
                        .await
                        .ok();
                }
                match erreur {
                    Some(e) => Err(e),
                    None => {
                        let held = if hold_ms > 0 {
                            format!(", held {hold_ms}ms")
                        } else {
                            String::new()
                        };
                        let times = if repeat > 1 {
                            format!(" x{repeat}")
                        } else {
                            String::new()
                        };
                        Ok(ResultatAbeille::ok(format!("Pressed {spec}{times}{held}.")))
                    }
                }
            }

            "hover" => {
                let Some(r) = args["ref"].as_u64() else {
                    return Ok(ResultatAbeille::err("hover needs a 'ref' number from read."));
                };
                // The real mouse move is what triggers :hover and the menus that
                // only open on it; the animation is only there to be watchable.
                let script = format!(
                    r#"window.__lrSpeed = {speed};
                       const el = document.querySelector('[data-lr-ref="{r}"]');
                       if (!el) return 'MISSING';
                       el.scrollIntoView({{block:'center', behavior:'instant'}});
                       if ({animate} && window.__larucheHoverAnim) {{
                         await window.__larucheHoverAnim(el);
                       }} else if (window.__laruchePulse) {{
                         window.__laruchePulse(el);
                       }}
                       const b = el.getBoundingClientRect();
                       return JSON.stringify({{ x: b.left + b.width / 2, y: b.top + b.height / 2,
                                                tag: el.tagName.toLowerCase() }});"#
                );
                match canal.eval(&script, true).await {
                    Err(e) => Err(e),
                    Ok(v) if v.as_str() == Some("MISSING") => Ok(ResultatAbeille::err(format!(
                        "No element ref_{r} on this page. Run read again."
                    ))),
                    Ok(v) => {
                        let at: Value =
                            serde_json::from_str(v.as_str().unwrap_or("{}")).unwrap_or(json!({}));
                        let (x, y) = (
                            at["x"].as_f64().unwrap_or(0.0),
                            at["y"].as_f64().unwrap_or(0.0),
                        );
                        canal
                            .input(
                                "Input.dispatchMouseEvent",
                                json!({ "type": "mouseMoved", "x": x, "y": y, "button": "none" }),
                            )
                            .await
                            .ok();
                        // Menus open on a timer often enough that returning
                        // immediately would have the agent read the page before
                        // anything appeared.
                        tokio::time::sleep(Duration::from_millis(350)).await;
                        Ok(ResultatAbeille::ok(format!(
                            "Hovering ref_{r} <{}>. Read again to see what it revealed.",
                            at["tag"].as_str().unwrap_or("")
                        )))
                    }
                }
            }

            "find" => {
                let Some(query) = args["text"].as_str().filter(|q| !q.trim().is_empty()) else {
                    return Ok(ResultatAbeille::err(
                        "find needs 'text', the label or wording to look for.".to_string(),
                    ));
                };
                // Deliberately the same mapping pass as read, filtered: refs stay
                // consistent with what a following click expects, and a page with
                // 200 controls comes back as the two lines that matter.
                match canal.eval(SCRIPT_READ, false).await {
                    Err(e) => Err(e),
                    Ok(v) => {
                        let snap: Value =
                            serde_json::from_str(v.as_str().unwrap_or("{}")).unwrap_or(json!({}));
                        let needle = query.to_lowercase();
                        let hits: Vec<&str> = snap["elements"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(Value::as_str)
                                    .filter(|l| l.to_lowercase().contains(&needle))
                                    .collect()
                            })
                            .unwrap_or_default();
                        if hits.is_empty() {
                            let total = snap["count"].as_u64().unwrap_or(0);
                            return Ok(ResultatAbeille::ok(format!(
                                "Nothing matching '{query}' among the {total} interactive \
                                 elements on this page. The wording may differ, the element \
                                 may be hidden behind a menu (try hover), or the page may \
                                 still be loading (try wait)."
                            )));
                        }
                        Ok(ResultatAbeille::ok(format!(
                            "{} match(es) for '{query}' (refs valid until the next read):\n{}",
                            hits.len(),
                            truncate(&hits.join("\n"), max_chars)
                        )))
                    }
                }
            }

            "wait" => {
                let selector = args["selector"].as_str().filter(|s| !s.trim().is_empty());
                let text = args["text"].as_str().filter(|s| !s.trim().is_empty());
                let timeout = args["timeout"].as_u64().unwrap_or(15).clamp(1, 120);
                if selector.is_none() && text.is_none() {
                    // A bare wait is a legitimate ask ("give the animation a
                    // second"), so it is not an error, just capped.
                    let ms = args["amount"].as_u64().unwrap_or(1000).clamp(50, 30_000);
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                    return Ok(ResultatAbeille::ok(format!("Waited {ms}ms.")));
                }
                let condition = match (selector, text) {
                    (Some(s), _) => format!(
                        "return !!document.querySelector({})",
                        serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
                    ),
                    (None, Some(t)) => format!(
                        "return (document.body ? document.body.innerText : '').toLowerCase().includes({})",
                        serde_json::to_string(&t.to_lowercase()).unwrap_or_else(|_| "\"\"".into())
                    ),
                    _ => unreachable!("one of the two is Some"),
                };
                let quoi = selector.map(|s| format!("selector {s}")).unwrap_or_else(|| {
                    format!("text \"{}\"", text.unwrap_or_default())
                });
                let debut = std::time::Instant::now();
                let limite = Duration::from_secs(timeout);
                loop {
                    match canal.eval(&condition, true).await {
                        Err(e) => break Err(e),
                        Ok(v) if v.as_bool() == Some(true) => {
                            break Ok(ResultatAbeille::ok(format!(
                                "Found {quoi} after {:.1}s.",
                                debut.elapsed().as_secs_f32()
                            )))
                        }
                        Ok(_) => {}
                    }
                    if debut.elapsed() >= limite {
                        break Ok(ResultatAbeille::err(format!(
                            "Still no {quoi} after {timeout}s. Read the page: it may have \
                             loaded something else, or asked for a consent click first."
                        )));
                    }
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }

            "console" => {
                let level = args["level"].as_str().unwrap_or("").to_lowercase();
                let limit = args["limit"].as_u64().unwrap_or(40).clamp(1, 200);
                let script = format!(
                    r#"const all = window.__lrLogs || null;
                       if (!all) return 'NOTAP';
                       const want = {level};
                       const rows = (want ? all.filter(l => l.level === want) : all).slice(-{limit});
                       return JSON.stringify({{ total: all.length, rows }});"#,
                    level = serde_json::to_string(&level).unwrap_or_else(|_| "\"\"".into()),
                );
                match canal.eval(&script, true).await {
                    Err(e) => Err(e),
                    Ok(v) if v.as_str() == Some("NOTAP") => Ok(ResultatAbeille::err(
                        "The console tap is not installed on this page yet. It goes on when \
                         the session opens and after each navigation, so navigate or read \
                         once, then try again."
                            .to_string(),
                    )),
                    Ok(v) => {
                        let out: Value =
                            serde_json::from_str(v.as_str().unwrap_or("{}")).unwrap_or(json!({}));
                        let rows = out["rows"].as_array().cloned().unwrap_or_default();
                        if rows.is_empty() {
                            return Ok(ResultatAbeille::ok(
                                "Console empty since the tap was installed (it only sees what \
                                 the page logged after that, not what came before)."
                                    .to_string(),
                            ));
                        }
                        let lignes: Vec<String> = rows
                            .iter()
                            .map(|l| {
                                format!(
                                    "{} [{}] {}",
                                    l["t"].as_str().unwrap_or(""),
                                    l["level"].as_str().unwrap_or("log"),
                                    l["text"].as_str().unwrap_or("")
                                )
                            })
                            .collect();
                        Ok(ResultatAbeille::ok(truncate(
                            &format!(
                                "{} console entries recorded, last {}:\n{}",
                                out["total"].as_u64().unwrap_or(0),
                                lignes.len(),
                                lignes.join("\n")
                            ),
                            max_chars,
                        )))
                    }
                }
            }

            "network" => {
                let limit = args["limit"].as_u64().unwrap_or(40).clamp(1, 200);
                let filtre = args["text"].as_str().unwrap_or("").to_lowercase();
                // The buffer holds fetch and XHR with their status; Resource
                // Timing covers everything else the page loaded, so an empty
                // buffer still answers "what did this page fetch".
                let script = format!(
                    r#"const buf = window.__lrNet || null;
                       if (!buf) return 'NOTAP';
                       const f = {filtre};
                       let rows = buf.filter(r => !f || r.url.toLowerCase().includes(f));
                       let source = 'fetch/xhr';
                       if (!rows.length) {{
                         rows = performance.getEntriesByType('resource')
                           .filter(e => !f || e.name.toLowerCase().includes(f))
                           .map(e => ({{ t: '', method: e.initiatorType, url: e.name,
                                        status: e.responseStatus || 0, ms: Math.round(e.duration), via: 'timing' }}));
                         source = 'resource timing';
                       }}
                       return JSON.stringify({{ total: rows.length, source, rows: rows.slice(-{limit}) }});"#,
                    filtre = serde_json::to_string(&filtre).unwrap_or_else(|_| "\"\"".into()),
                );
                match canal.eval(&script, true).await {
                    Err(e) => Err(e),
                    Ok(v) if v.as_str() == Some("NOTAP") => Ok(ResultatAbeille::err(
                        "The network tap is not installed on this page yet. Navigate or read \
                         once, then try again."
                            .to_string(),
                    )),
                    Ok(v) => {
                        let out: Value =
                            serde_json::from_str(v.as_str().unwrap_or("{}")).unwrap_or(json!({}));
                        let rows = out["rows"].as_array().cloned().unwrap_or_default();
                        if rows.is_empty() {
                            return Ok(ResultatAbeille::ok(
                                "No request recorded on this page since the tap went on."
                                    .to_string(),
                            ));
                        }
                        let lignes: Vec<String> = rows
                            .iter()
                            .map(|r| {
                                format!(
                                    "{:>3} {:<6} {}ms  {}",
                                    r["status"].as_i64().unwrap_or(0),
                                    r["method"].as_str().unwrap_or(""),
                                    r["ms"].as_i64().unwrap_or(0),
                                    r["url"].as_str().unwrap_or("")
                                )
                            })
                            .collect();
                        Ok(ResultatAbeille::ok(truncate(
                            &format!(
                                "{} request(s) via {}, last {}:\n{}",
                                out["total"].as_u64().unwrap_or(0),
                                out["source"].as_str().unwrap_or(""),
                                lignes.len(),
                                lignes.join("\n")
                            ),
                            max_chars,
                        )))
                    }
                }
            }

            "back" | "forward" => {
                let sens = if action == "back" { "back" } else { "forward" };
                let avant = canal
                    .eval("return location.href", true)
                    .await
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                if let Err(e) = canal.eval(&format!("history.{sens}()"), false).await {
                    return Ok(ResultatAbeille::err(format!("{e}")));
                }
                // History moves are asynchronous, and a same-document one fires
                // no load at all, so settle on the URL rather than readyState.
                let debut = std::time::Instant::now();
                let mut apres = avant.clone();
                while debut.elapsed() < Duration::from_secs(8) {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    apres = canal
                        .eval("return location.href", true)
                        .await
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default();
                    if apres != avant && !apres.is_empty() {
                        break;
                    }
                }
                if glow {
                    canal.eval(SCRIPT_GLOW, false).await.ok();
                }
                if apres == avant {
                    return Ok(ResultatAbeille::ok(format!(
                        "Went {sens} but the URL did not change: this tab has no {sens} \
                         history, or the move stayed in the same document."
                    )));
                }
                Ok(ResultatAbeille::ok(format!(
                    "Went {sens} to {apres}. Refs are gone: read again."
                )))
            }

            other => Ok(ResultatAbeille::err(format!(
                "Unknown action '{other}'. Use navigate, read, find, click, fill, key, hover, \
                 scroll, wait, eval, screenshot, console, network, back, forward, tabs, select \
                 or close."
            ))),
        };

        match outcome {
            Ok(r) => Ok(r),
            Err(e) => {
                // A broken pipe must not poison every later call.
                *guard = None;
                Ok(ResultatAbeille::err(format!(
                    "{e}. The session was dropped, the next call reopens one."
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_declares_every_action() {
        let s = Browser.schema();
        let actions = s["properties"]["action"]["enum"].as_array().unwrap();
        for expected in [
            "navigate",
            "read",
            "find",
            "click",
            "fill",
            "key",
            "hover",
            "scroll",
            "wait",
            "eval",
            "screenshot",
            "console",
            "network",
            "back",
            "forward",
            "tabs",
            "select",
            "close",
        ] {
            assert!(
                actions.iter().any(|a| a.as_str() == Some(expected)),
                "missing action {expected}"
            );
            // Every action must also be named in the description: a model that
            // only reads the prose is the common case, and one that believes an
            // action does not exist works around it with eval instead.
            assert!(
                Browser.description().contains(expected),
                "action {expected} missing from the description"
            );
        }
    }

    #[test]
    fn touches_nommees_et_accords() {
        let (m, t) = parse_touche("Enter").expect("Enter is a key");
        assert_eq!((m, t.vk, t.text.as_str()), (0, 13, "\r"));
        // Case must not matter: a model writes "enter" as often as "Enter".
        assert_eq!(parse_touche("enter").unwrap().1.vk, 13);
        assert_eq!(parse_touche("ArrowDown").unwrap().1.vk, 40);
        assert_eq!(parse_touche("F5").unwrap().1.vk, 116);

        let (m, t) = parse_touche("Control+a").expect("chord");
        assert_eq!(m, MOD_CTRL);
        assert_eq!(t.code, "KeyA");
        // The whole point of the chord: Ctrl+A selects, it does not type an "a".
        assert!(t.text.is_empty(), "Ctrl+A must not produce text");

        let (m, _) = parse_touche("ctrl+shift+Tab").expect("two modifiers");
        assert_eq!(m, MOD_CTRL | MOD_SHIFT);

        // A plain character still types itself.
        assert_eq!(parse_touche("k").unwrap().1.text, "k");
        // Nonsense must fail rather than press something else.
        assert!(parse_touche("Ctrl+Nope").is_none());
        assert!(parse_touche("").is_none());
    }

    #[test]
    fn evenement_touche_porte_le_texte_au_bon_moment() {
        let (m, t) = parse_touche("Enter").unwrap();
        let down = evenement_touche("keyDown", &t, m);
        assert_eq!(down["text"], "\r");
        // Only keyDown carries text: a keyUp that types would double every press.
        let up = evenement_touche("keyUp", &t, m);
        assert!(up.get("text").is_none());
    }

    #[test]
    fn tap_script_records_both_channels_once() {
        // Idempotent: it is evaluated again on every action.
        assert!(SCRIPT_TAP.contains("if (window.__lrTap) return"));
        assert!(SCRIPT_TAP.contains("__lrLogs"));
        assert!(SCRIPT_TAP.contains("__lrNet"));
        // Uncaught errors never reach the console patch, and they matter most.
        assert!(SCRIPT_TAP.contains("unhandledrejection"));
        // The page's own console must keep working.
        assert!(SCRIPT_TAP.contains("original(...args)"));
    }

    #[test]
    fn truncate_keeps_short_text_untouched() {
        assert_eq!(truncate("hello", 10), "hello");
        assert!(truncate("hello world", 5).starts_with("hello\n"));
    }

    #[test]
    fn read_script_tags_refs_and_returns_json() {
        assert!(SCRIPT_READ.contains("data-lr-ref"));
        assert!(SCRIPT_READ.contains("JSON.stringify"));
    }

    #[test]
    fn glow_script_is_isolated_and_reversible() {
        assert!(SCRIPT_GLOW.contains("attachShadow"));
        assert!(SCRIPT_GLOW.contains("pointer-events:none"));
        assert!(SCRIPT_GLOW.contains("__larucheGlowOff"));
        assert!(SCRIPT_GLOW.contains("prefers-reduced-motion"));
        // The animation helpers the action scripts rely on must exist.
        assert!(SCRIPT_GLOW.contains("__larucheClickAnim"));
        assert!(SCRIPT_GLOW.contains("__larucheTypeAnim"));
        assert!(SCRIPT_GLOW.contains("__larucheScrollAnim"));
        assert!(SCRIPT_GLOW.contains("__larucheHoverAnim"));
    }

    #[test]
    fn hud_lives_in_the_shadow_root_and_survives_reruns() {
        assert!(SCRIPT_GLOW.contains("__larucheHud"));
        // The panel is the one part that takes clicks, so it can be dragged.
        assert!(SCRIPT_GLOW.contains("pointer-events:auto"));
        // Position, folded state and log are read back from window on each run,
        // otherwise the panel would jump home at every single action.
        for state in ["__lrHudPos", "__lrHudMin", "__lrHudLog"] {
            assert!(SCRIPT_GLOW.contains(state), "{state} not carried across runs");
        }
    }

    #[test]
    fn le_panneau_porte_le_chat_et_le_cloisonne() {
        // Narration poussee, reponse deposee: les deux moities du compagnon.
        assert!(SCRIPT_GLOW.contains("__larucheChat"));
        assert!(SCRIPT_GLOW.contains("__lrSorties"));
        // La saisie doit prendre les evenements souris ET arreter les touches:
        // le shadow DOM ne cloisonne pas les evenements clavier, et taper ici
        // declencherait sinon les raccourcis de la page hote.
        assert!(SCRIPT_GLOW.contains("stopPropagation"));
    }

    #[test]
    fn le_script_de_releve_pousse_et_ramene() {
        let script = script_releve("bonjour", false, true);
        // Il pousse quand il y a du neuf...
        assert!(script.contains("__larucheChat(\"bonjour\", false)"));
        // ...et il vide la file dans tous les cas, sinon une reponse tapee
        // pendant un silence du modele resterait coincee dans la page.
        assert!(script.contains("window.__lrSorties = []"));

        // Rien de neuf: on ne pousse pas, mais on releve quand meme.
        let script = script_releve("bonjour", true, false);
        assert!(script.contains("if (false &&"));
        assert!(script.contains("window.__lrSorties = []"));

        // Le texte est encode, pas concatene: une narration avec un guillemet
        // ou un saut de ligne casserait le script sinon.
        let script = script_releve("il a dit \"non\"\nensuite", false, true);
        assert!(script.contains(r#"\"non\""#), "{script}");
        assert!(!script.contains("\nensuite"), "saut de ligne non echappe");
    }

    #[test]
    fn la_narration_ne_grandit_pas_sans_fin() {
        use crate::evenements::ChatEvent;
        GLOW_ACTIF.store(true, std::sync::atomic::Ordering::Relaxed);
        // Repartir d'un tampon propre sans passer par brancher_pilotage, qui
        // toucherait au canal de pilotage d'un autre test.
        *narration().lock().unwrap() = Narration::default();

        for _ in 0..400 {
            narrer(&ChatEvent::Token {
                text: "chaque jeton compte ".into(),
            });
        }
        let n = narration().lock().unwrap();
        assert!(
            n.texte.chars().count() <= NARRATION_MAX * 2,
            "le tampon a enfle: {}",
            n.texte.chars().count()
        );
        // Et c'est bien la FIN qui est gardee: le debut d'une reponse longue
        // n'interesse plus personne au moment ou on la lit.
        assert!(n.texte.ends_with("chaque jeton compte "));
        drop(n);
        GLOW_ACTIF.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    #[test]
    fn sans_page_pilotee_narrer_ne_coute_rien() {
        use crate::evenements::ChatEvent;
        GLOW_ACTIF.store(false, std::sync::atomic::Ordering::Relaxed);
        *narration().lock().unwrap() = Narration::default();
        narrer(&ChatEvent::Token {
            text: "personne ne regarde".into(),
        });
        assert!(
            narration().lock().unwrap().texte.is_empty(),
            "un evenement doit etre ignore quand aucune page n'est pilotee"
        );
    }

    /// End-to-end against a real headless Chrome, on a page built in memory so
    /// the test needs no network. Ignored by default: CI machines have no
    /// browser, and the run leaves a Chrome process behind for a few seconds.
    ///
    /// Run it with:
    ///   cargo test -p laruche-essaim --lib browser_cdp -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "requires a local Chrome installation"]
    async fn live_round_trip() {
        let ctx = ContextExecution::default();
        let port = 9333; // off the usual 9222 to avoid stealing a real session

        let build_page = r#"
            // The profile is persistent, so a previous run's tab is still there
            // with its counters. Reset them or the assertions below measure the
            // history of every run since the browser started.
            window.__enter = 0;
            window.__hovered = false;
            document.body.innerHTML = `
              <h1>Ruche test</h1>
              <input id="q" placeholder="search field">
              <button id="go">Launch</button>`;
            document.getElementById('go').addEventListener('click', () => {
              document.body.insertAdjacentHTML('beforeend', '<p id="done">clicked</p>');
            });
            // Records a REAL key press: an untrusted synthetic event would not
            // set isTrusted, so this doubles as a check that key goes through
            // the Input domain rather than dispatchEvent.
            document.getElementById('q').addEventListener('keydown', (e) => {
              if (e.key === 'Enter' && e.isTrusted) window.__enter = (window.__enter || 0) + 1;
            });
            document.getElementById('go').addEventListener('mouseover', () => {
              window.__hovered = true;
            });
            console.warn('ruche test warning');
            return 'page ready';
        "#;

        let out = Browser
            .executer(
                json!({ "action": "eval", "script": build_page,
                        "mode": "launch", "headless": true, "port": port }),
                &ctx,
            )
            .await
            .expect("eval must not fail hard");
        assert!(out.success, "could not build the page: {:?}", out.error);

        // read must find both controls and number them.
        let out = Browser
            .executer(json!({ "action": "read", "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "read failed: {:?}", out.error);
        assert!(out.output.contains("ref_1"), "no refs in:\n{}", out.output);
        assert!(
            out.output.contains("search field"),
            "input not mapped:\n{}",
            out.output
        );

        // fill then click, addressed only by ref: the no-vision path.
        let out = Browser
            .executer(
                json!({ "action": "fill", "ref": 1, "text": "hello", "port": port }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "fill failed: {:?}", out.error);

        let out = Browser
            .executer(json!({ "action": "click", "ref": 2, "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "click failed: {:?}", out.error);

        // The click handler must have run, and the typed value must have landed.
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "return (document.getElementById('done') ? 'ok' : 'no') + ':' + document.getElementById('q').value" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "ok:hello", "click or fill had no effect");

        // scroll must run and report, not error like the missing action it used to be.
        let out = Browser
            .executer(
                json!({ "action": "scroll", "direction": "bottom", "port": port }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "scroll failed: {:?}", out.error);
        assert!(out.output.contains("bottom"), "scroll wrong: {}", out.output);

        // find must return only what matches, addressed the same way as read.
        let out = Browser
            .executer(
                json!({ "action": "find", "text": "launch", "port": port }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "find failed: {:?}", out.error);
        assert!(out.output.contains("ref_2"), "find missed it:\n{}", out.output);
        assert!(
            !out.output.contains("search field"),
            "find returned the whole map:\n{}",
            out.output
        );

        // key must produce a TRUSTED Enter inside the field, which is the whole
        // reason it goes through Input rather than dispatchEvent.
        let out = Browser
            .executer(
                json!({ "action": "key", "key": "Enter", "ref": 1, "port": port }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "key failed: {:?}", out.error);
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port, "script": "return String(window.__enter || 0)" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "1", "no trusted Enter reached the field");

        // repeat must press it again, not just once more in appearance.
        Browser
            .executer(
                json!({ "action": "key", "key": "Enter", "ref": 1, "repeat": 2, "port": port }),
                &ctx,
            )
            .await
            .unwrap();
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port, "script": "return String(window.__enter || 0)" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "3", "repeat did not press twice");

        // hover must move a real mouse, which is what fires mouseover.
        let out = Browser
            .executer(json!({ "action": "hover", "ref": 2, "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "hover failed: {:?}", out.error);
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port, "script": "return String(!!window.__hovered)" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "true", "hover fired no mouseover");

        // wait must return as soon as the text lands, not on timeout.
        Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "setTimeout(() => document.body.insertAdjacentHTML('beforeend', '<p>tardif</p>'), 600); return 'armed'" }),
                &ctx,
            )
            .await
            .unwrap();
        let t0 = std::time::Instant::now();
        let out = Browser
            .executer(
                json!({ "action": "wait", "text": "tardif", "timeout": 10, "port": port }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "wait failed: {:?}", out.error);
        assert!(t0.elapsed().as_secs() < 5, "wait did not return promptly");

        // The console tap must have caught the page's own warning.
        let out = Browser
            .executer(json!({ "action": "console", "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "console failed: {:?}", out.error);
        assert!(
            out.output.contains("ruche test warning"),
            "console tap missed it:\n{}",
            out.output
        );

        // And the network tap must record a fetch the page makes.
        Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "await fetch('data:text/plain,ok'); return 'fetched'" }),
                &ctx,
            )
            .await
            .unwrap();
        let out = Browser
            .executer(json!({ "action": "network", "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "network failed: {:?}", out.error);
        assert!(
            out.output.contains("data:text/plain"),
            "network tap missed the fetch:\n{}",
            out.output
        );

        // tabs must list at least the page we built, and select must adopt it.
        let out = Browser
            .executer(json!({ "action": "tabs", "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "tabs failed: {:?}", out.error);
        assert!(out.output.contains("open tab"), "tabs wrong: {}", out.output);

        // The indicator must be present, in a shadow root, and click-through.
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "const h=document.getElementById('__laruche_glow__'); return h ? (h.shadowRoot ? 'shadow' : 'plain') + ':' + getComputedStyle(h).pointerEvents : 'absent'" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "shadow:none", "indicator misbehaving");

        let out = Browser
            .executer(json!({ "action": "screenshot", "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "screenshot failed: {:?}", out.error);
        assert_eq!(out.images.len(), 1, "no image returned to the model");
        assert!(out.images[0].len() > 1000, "image suspiciously small");

        // close must take the indicator down with it.
        let out = Browser
            .executer(json!({ "action": "close", "port": port }), &ctx)
            .await
            .unwrap();
        assert!(out.success);

        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port, "glow": false,
                        "script": "return document.getElementById('__laruche_glow__') ? 'still there' : 'gone'" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "gone", "indicator survived close");

        Browser
            .executer(json!({ "action": "close", "port": port }), &ctx)
            .await
            .ok();
    }

    /// Le compagnon de page, dans les DEUX sens, contre un vrai Chrome: la
    /// narration descend jusqu'a la page, et ce qui y est tape remonte dans le
    /// canal de pilotage de la session.
    ///
    ///   cargo test -p laruche-essaim --lib compagnon_vivant -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "requires a local Chrome installation"]
    async fn compagnon_vivant() {
        use crate::evenements::ChatEvent;
        let ctx = ContextExecution::default();
        let port = 9334;

        let out = Browser
            .executer(
                json!({ "action": "eval", "mode": "launch", "headless": true, "port": port,
                        "script": "document.body.innerHTML = '<h1>compagnon</h1>'; return 'ok'" }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "ouverture: {:?}", out.error);

        // Le noeud declare le canal de pilotage du tour, puis le modele parle.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(8);
        brancher_pilotage(tx);
        narrer(&ChatEvent::Token {
            text: "je regarde la page".into(),
        });

        // La releve passe deux fois par seconde: on lui laisse un tour.
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "return String(window.__lrChat || 'rien')" }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(
            out.output.contains("je regarde la page"),
            "la narration n'est pas descendue: {}",
            out.output
        );

        // Puis l'humain repond dans le panneau. On depose directement dans la
        // file, ce que fait le bouton, pour ne pas dependre du rendu.
        Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "window.__lrSorties = ['arrete, mauvaise page']; return 'depose'" }),
                &ctx,
            )
            .await
            .unwrap();

        let recu = tokio::time::timeout(Duration::from_secs(4), rx.recv())
            .await
            .expect("la reponse doit remonter avant l'expiration")
            .expect("canal ouvert");
        assert_eq!(recu, "arrete, mauvaise page");

        // Et la file de la page doit avoir ete videe, sinon le meme message
        // repartirait a chaque passage.
        let out = Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "return String((window.__lrSorties || []).length)" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(out.output.trim(), "0", "la file n'a pas ete videe");

        debrancher_pilotage();
        Browser
            .executer(json!({ "action": "close", "port": port }), &ctx)
            .await
            .ok();
    }

    /// Photographie le panneau tel qu'un utilisateur le voit, avec sa narration
    /// et sa zone de reponse. A regarder, pas a asserter.
    ///
    ///   cargo test -p laruche-essaim --lib panneau_visuel -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "requires a local Chrome installation"]
    async fn panneau_visuel() {
        use crate::evenements::ChatEvent;
        use base64::Engine;
        let ctx = ContextExecution::default();
        let port = 9335;

        let page = "document.body.style.cssText = 'margin:0;height:100vh;background:#f4f1ea;\
                    font:16px system-ui;display:flex;align-items:center;justify-content:center'; \
                    document.body.innerHTML = '<div>une page ordinaire, pilotee par LaRuche</div>'; \
                    return 'prete'";
        let out = Browser
            .executer(
                json!({ "action": "eval", "mode": "launch", "headless": false, "port": port,
                        "script": page }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "ouverture: {:?}", out.error);

        // Une action, pour que la liste du haut ne soit pas vide.
        Browser
            .executer(json!({ "action": "read", "port": port }), &ctx)
            .await
            .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(8);
        brancher_pilotage(tx);
        for morceau in [
            "Je regarde la page. ",
            "Elle ne contient qu'un titre, ",
            "donc rien a remplir ici.",
        ] {
            narrer(&ChatEvent::Token {
                text: morceau.into(),
            });
        }
        tokio::time::sleep(Duration::from_millis(1200)).await;

        // Une reponse deja tapee, pour montrer les deux voix dans le panneau.
        Browser
            .executer(
                json!({ "action": "eval", "port": port,
                        "script": "window.__lrDit = ['prends plutot la tab d a cote']; \
                                   window.__larucheChat(window.__lrChat, true); return 'ok'" }),
                &ctx,
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;

        let out = Browser
            .executer(json!({ "action": "screenshot", "port": port }), &ctx)
            .await
            .unwrap();
        let octets = base64::engine::general_purpose::STANDARD
            .decode(&out.images[0])
            .expect("png");
        let chemin = std::env::temp_dir().join("laruche-panneau.png");
        std::fs::write(&chemin, octets).expect("ecriture");
        println!("PANNEAU ECRIT: {}", chemin.display());

        debrancher_pilotage();
        Browser
            .executer(json!({ "action": "close", "port": port }), &ctx)
            .await
            .ok();
    }
}
