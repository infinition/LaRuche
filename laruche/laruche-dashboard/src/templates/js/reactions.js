/* Reactions on an answer: one click instead of a sentence.
 *
 * The picker is attached LAZILY, on first hover or focus of an assistant row, through
 * delegation on the chat container. That way live messages and messages restored from
 * history go through the same path, and a long conversation does not build hundreds of
 * pickers nobody will ever open.
 *
 * The index sent to the backend is the REAL session index when we know it (restored
 * history carries it), and -1 otherwise, which the API resolves to the last answer. A
 * message that just finished streaming has no index client-side yet, and waiting for a
 * round trip to learn it would put a delay on the one interaction that must feel
 * instant.
 */
(function(){
  'use strict';

  // Fallback only. The real palette comes from /api/reactions/palette so the
  // vocabulary lives in ONE place (reactions.rs) and the UI cannot drift from what
  // the model is actually told.
  var PALETTE = [
    {key:'up', emoji:'👍'},
    {key:'down', emoji:'👎'},
    {key:'love', emoji:'❤️'},
    {key:'haha', emoji:'😂'},
    {key:'wow', emoji:'😮'},
    {key:'confused', emoji:'😕'}
  ];

  var openPicker = null;

  function t(key, fallback){
    try {
      var v = window.LaRuche && LaRuche.i18n && LaRuche.i18n.t(key);
      return (v && v !== key) ? v : fallback;
    } catch(e){ return fallback; }
  }

  function emojiFor(key){
    for(var i=0;i<PALETTE.length;i++){ if(PALETTE[i].key===key) return PALETTE[i].emoji; }
    return '';
  }

  function loadPalette(){
    fetch('/api/reactions/palette')
      .then(function(r){ return r.ok ? r.json() : null; })
      .then(function(d){ if(d && d.reactions && d.reactions.length) PALETTE = d.reactions; })
      .catch(function(){ /* keep the fallback: the feature still works offline */ });
  }

  function sessionId(){
    return (window.LaRuche && LaRuche.Chat && LaRuche.Chat.getSessionId && LaRuche.Chat.getSessionId()) || null;
  }

  function closePicker(){
    if(!openPicker) return;
    openPicker.classList.remove('open');
    var trigger = openPicker.parentNode && openPicker.parentNode.querySelector('.reaction-trigger');
    if(trigger) trigger.setAttribute('aria-expanded','false');
    openPicker = null;
  }

  /* Persist, then reflect. On failure we roll the chip back rather than leave the UI
   * claiming a reaction the agent will never see. */
  function send(row, key){
    var id = sessionId();
    var previous = row.dataset.reaction || '';
    var next = (previous === key) ? '' : key;   // second click on the same one clears it
    render(row, next);
    if(!id){ return; }
    var raw = row.dataset.msgIndex;
    var index = (raw === undefined || raw === '') ? -1 : parseInt(raw, 10);
    fetch('/api/sessions/'+encodeURIComponent(id)+'/reaction', {
      method:'POST',
      headers:{'Content-Type':'application/json'},
      body: JSON.stringify({ index: isNaN(index) ? -1 : index, reaction: next })
    }).then(function(r){
      if(!r.ok) throw new Error('http '+r.status);
    }).catch(function(){
      render(row, previous);
      if(window.LaRuche && LaRuche.Utils && LaRuche.Utils.toast){
        LaRuche.Utils.toast(t('reactions.failed','Reaction not saved'), 'error');
      }
    });
  }

  /* Draw the chosen reaction under the bubble, or remove the chip when cleared. */
  function render(row, key){
    row.dataset.reaction = key || '';
    var wrapper = row.querySelector('.message-wrapper') || row;
    var chip = wrapper.querySelector('.reaction-chip');
    if(!key){
      if(chip) chip.remove();
      var tr0 = row.querySelector('.reaction-trigger');
      if(tr0) tr0.setAttribute('aria-pressed','false');
      return;
    }
    if(!chip){
      chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'reaction-chip';
      chip.addEventListener('click', function(ev){
        ev.stopPropagation();
        send(row, row.dataset.reaction);   // clicking the chip clears it
      });
      var ts = wrapper.querySelector('.msg-timestamp');
      if(ts) wrapper.insertBefore(chip, ts); else wrapper.appendChild(chip);
    }
    chip.textContent = emojiFor(key) || key;
    chip.title = t('reactions.clear','Click to remove your reaction');
    chip.setAttribute('aria-label', t('reactions.clear','Click to remove your reaction'));
    chip.classList.remove('pop');
    void chip.offsetWidth;             // restart the animation on a re-pick
    chip.classList.add('pop');
    var tr = row.querySelector('.reaction-trigger');
    if(tr) tr.setAttribute('aria-pressed','true');
  }

  function buildPicker(row){
    var picker = document.createElement('div');
    picker.className = 'reaction-picker';
    picker.setAttribute('role','menu');
    PALETTE.forEach(function(r, i){
      var b = document.createElement('button');
      b.type = 'button';
      b.className = 'reaction-option';
      b.dataset.key = r.key;
      b.textContent = r.emoji;
      b.setAttribute('role','menuitem');
      b.style.setProperty('--i', i);     // staggered entrance
      var label = t('reactions.'+r.key, r.key);
      b.title = label;
      b.setAttribute('aria-label', label);
      b.addEventListener('click', function(ev){
        ev.stopPropagation();
        send(row, r.key);
        closePicker();
      });
      b.addEventListener('keydown', function(ev){
        var opts = Array.prototype.slice.call(picker.querySelectorAll('.reaction-option'));
        var at = opts.indexOf(b);
        if(ev.key === 'ArrowRight' || ev.key === 'ArrowLeft'){
          ev.preventDefault();
          var step = ev.key === 'ArrowRight' ? 1 : -1;
          opts[(at + step + opts.length) % opts.length].focus();
        }
      });
      picker.appendChild(b);
    });
    return picker;
  }

  /* One trigger + one picker per assistant row, created on first interaction. */
  function ensureControls(row){
    if(row.dataset.reactionsReady === '1') return;
    row.dataset.reactionsReady = '1';

    var wrapper = row.querySelector('.message-wrapper');
    if(!wrapper) return;

    var host = document.createElement('div');
    host.className = 'reaction-host';

    var trigger = document.createElement('button');
    trigger.type = 'button';
    trigger.className = 'reaction-trigger';
    trigger.innerHTML = '<span aria-hidden="true">🙂</span>';
    trigger.title = t('reactions.react','React to this answer');
    trigger.setAttribute('aria-label', t('reactions.react','React to this answer'));
    trigger.setAttribute('aria-haspopup','true');
    trigger.setAttribute('aria-expanded','false');
    trigger.setAttribute('aria-pressed', row.dataset.reaction ? 'true' : 'false');

    var picker = buildPicker(row);

    trigger.addEventListener('click', function(ev){
      ev.stopPropagation();
      var wasOpen = (openPicker === picker);
      closePicker();
      if(wasOpen) return;
      picker.classList.add('open');
      trigger.setAttribute('aria-expanded','true');
      openPicker = picker;
      var first = picker.querySelector('.reaction-option');
      if(first) first.focus();
    });

    host.appendChild(trigger);
    host.appendChild(picker);
    wrapper.appendChild(host);
  }

  function isAssistantRow(el){
    return el && el.classList && el.classList.contains('message-row')
        && el.classList.contains('assistant')
        && !el.classList.contains('assistant-intermediate')
        && !el.classList.contains('tool-row');
  }

  function init(){
    var container = document.getElementById('chatContainer')
                 || document.querySelector('.chat-container');
    if(!container) return;

    loadPalette();

    // Lazy attach: the first time a row is hovered or focused, it gets its controls.
    ['mouseover','focusin'].forEach(function(evt){
      container.addEventListener(evt, function(ev){
        var row = ev.target && ev.target.closest && ev.target.closest('.message-row');
        if(isAssistantRow(row)) ensureControls(row);
      });
    });

    document.addEventListener('click', closePicker);
    document.addEventListener('keydown', function(ev){
      if(ev.key === 'Escape' && openPicker){
        var trigger = openPicker.parentNode.querySelector('.reaction-trigger');
        closePicker();
        if(trigger) trigger.focus();
      }
    });
  }

  /* Restore the reactions of a reloaded conversation. Called by chat.js once the
   * history is in the DOM, with the map the backend keeps keyed by real index. */
  function restore(map){
    if(!map) return;
    var container = document.getElementById('chatContainer')
                 || document.querySelector('.chat-container');
    if(!container) return;
    Object.keys(map).forEach(function(index){
      var row = container.querySelector('.message-row.assistant[data-msg-index="'+index+'"]');
      if(row) render(row, map[index]);
    });
  }

  window.LaRuche = window.LaRuche || {};
  window.LaRuche.Reactions = { init: init, restore: restore, render: render };

  if(document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
