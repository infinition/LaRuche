/* Summon LaReine by hand, on the conversation as it stands.
 *
 * She ships switched off, so the most distinctive part of the project is something
 * most people never see once. A crown on the bee's avatar makes it discoverable and
 * reversible: one click, a verdict, nothing rewritten behind your back.
 *
 * Deliberately judge-only. The automatic path sends the worker back to redo the work,
 * which is right when she is on duty and watching every turn. Asked for by hand on a
 * message already on screen, silently replacing it would be the wrong move.
 */
(function(){
  'use strict';

  var busy = false;

  function t(key, fallback){
    try {
      var v = window.LaRuche && LaRuche.i18n && LaRuche.i18n.t(key);
      return (v && v !== key) ? v : fallback;
    } catch(e){ return fallback; }
  }

  function esc(s){
    return (window.LaRuche && LaRuche.Utils && LaRuche.Utils.esc)
      ? LaRuche.Utils.esc(s) : String(s == null ? '' : s);
  }

  function sessionId(){
    return (window.LaRuche && LaRuche.Chat && LaRuche.Chat.getSessionId && LaRuche.Chat.getSessionId()) || null;
  }

  /* True when LaReine is not on duty. The button only makes sense then: with a mode
   * active she already reviews every turn, and a second verdict would just be noise. */
  function inactive(){
    try {
      var m = window.LaRuche && LaRuche.Settings && LaRuche.Settings.reineMode && LaRuche.Settings.reineMode();
      return !m || m === 'off';
    } catch(e){ return true; }
  }

  function scoreBar(label, value){
    var v = Math.max(0, Math.min(100, Number(value) || 0));
    var tone = v >= 70 ? 'good' : (v >= 40 ? 'mid' : 'low');
    return '<div class="reine-score">'
         +   '<span class="reine-score-label">'+esc(label)+'</span>'
         +   '<span class="reine-score-track"><span class="reine-score-fill '+tone+'" style="width:'+v+'%"></span></span>'
         +   '<span class="reine-score-value">'+v+'</span>'
         + '</div>';
  }

  function renderCard(row, data){
    var host = row.querySelector('.message-wrapper') || row;
    var old = host.querySelector('.reine-appel-card');
    if(old) old.remove();

    var card = document.createElement('div');
    card.className = 'reine-appel-card avis-' + (data.avis || 'revise');
    card.setAttribute('role','status');

    if(data.ok === false){
      card.innerHTML = '<div class="reine-appel-head"><span class="reine-crown">👑</span>'
                     + '<span>'+esc(data.error || t('reine.appelFailed','LaReine could not deliver a verdict.'))+'</span></div>';
      host.appendChild(card);
      return;
    }

    var s = data.scores || {};
    var html = '<div class="reine-appel-head">'
             +   '<span class="reine-crown">👑</span>'
             +   '<span class="reine-appel-verdict">'+esc(data.verdict || '')+'</span>'
             + '</div>'
             + '<div class="reine-scores">'
             +   scoreBar(t('reine.relevance','Relevance'), s.pertinence)
             +   scoreBar(t('reine.method','Method'), s.methodologie)
             +   scoreBar(t('reine.objective','Objective'), s.objectif)
             +   scoreBar(t('reine.brand','Brand'), s.conformite_marque)
             + '</div>';

    // Her corrective instruction is the actionable part. When she asks for a revision,
    // the button actually SENDS THE WORK BACK: a fresh agentic run with her instruction,
    // streamed into the chat. That is the whole point of summoning her; a verdict you
    // then have to act on by hand is only half the feature.
    if(data.instruction){
      html += '<div class="reine-appel-instruction"><strong>'+esc(t('reine.instruction','What she asks for'))+'</strong> '
            + esc(data.instruction)+'</div>';
    }
    if(data.avis === 'revise' || data.avis === 'escalate'){
      html += '<button type="button" class="reine-appel-apply">'+esc(t('reine.sendBack','Send LaRuche back to work'))+'</button>';
    }
    if(data.analyse){
      html += '<details class="reine-appel-analyse"><summary>'+esc(t('reine.analysis','Her reasoning'))+'</summary>'
            + '<div>'+esc(data.analyse)+'</div></details>';
    }
    card.innerHTML = html;

    var apply = card.querySelector('.reine-appel-apply');
    if(apply){
      apply.addEventListener('click', function(){
        var id = sessionId();
        if(!id) return;
        apply.disabled = true;
        apply.textContent = t('reine.sendingBack','LaRuche is redoing the work...');
        fetch('/api/reine/renvoyer', {
          method:'POST',
          headers:{'Content-Type':'application/json'},
          body: JSON.stringify({ session_id: id })
        }).then(function(r){
          if(!r.ok) throw new Error('http '+r.status);
          // From here the verdict and the fresh run stream in over the chat channel,
          // exactly as they do with LaReine on duty. This card has done its job.
          card.classList.add('handed-over');
        }).catch(function(){
          apply.disabled = false;
          apply.textContent = t('reine.sendBack','Send LaRuche back to work');
          if(window.LaRuche && LaRuche.Toast) LaRuche.Toast.show(t('reine.appelFailed','LaReine could not deliver a verdict.'),'err');
        });
      });
    }
    host.appendChild(card);
    card.scrollIntoView({block:'nearest', behavior:'smooth'});
  }

  function summon(row, btn){
    var id = sessionId();
    if(!id || busy) return;
    busy = true;
    btn.classList.add('working');
    btn.disabled = true;
    btn.title = t('reine.thinking','LaReine is reading the conversation...');

    fetch('/api/reine/appel', {
      method:'POST',
      headers:{'Content-Type':'application/json'},
      body: JSON.stringify({ session_id: id })
    })
      .then(function(r){
        if(!r.ok) throw new Error('http '+r.status);
        return r.json();
      })
      .then(function(d){ renderCard(row, d); })
      .catch(function(){
        renderCard(row, {ok:false, error:t('reine.appelFailed','LaReine could not deliver a verdict.')});
      })
      .then(function(){
        busy = false;
        btn.classList.remove('working');
        btn.disabled = false;
        btn.title = t('reine.summon','Ask LaReine to review this');
      });
  }

  function ensureButton(row){
    if(row.dataset.reineReady === '1') return;
    row.dataset.reineReady = '1';
    if(!inactive()) return;                 // she is already on duty: no second verdict

    var avatar = row.querySelector('.assistant-avatar');
    if(!avatar) return;
    avatar.classList.add('has-reine-call');

    var btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'reine-call-btn';
    btn.innerHTML = '<span aria-hidden="true">👑</span>';
    btn.title = t('reine.summon','Ask LaReine to review this');
    btn.setAttribute('aria-label', t('reine.summon','Ask LaReine to review this'));
    btn.addEventListener('click', function(ev){
      ev.stopPropagation();
      summon(row, btn);
    });
    avatar.appendChild(btn);
  }

  /* LaReine's own rows are excluded. `reine-row` carries her verdicts and her
   * failure notices, not an answer from LaRuche: reacting to a verdict steers
   * nothing, and offering to summon her on top of her own output is nonsense.
   * `reine-rework-row` IS a real answer (LaRuche redoing the work at her request),
   * so it keeps its controls. */
  function isAssistantRow(el){
    return el && el.classList && el.classList.contains('message-row')
        && el.classList.contains('assistant')
        && !el.classList.contains('reine-row')
        && !el.classList.contains('assistant-intermediate')
        && !el.classList.contains('tool-row');
  }

  function init(){
    var container = document.getElementById('chatContainer')
                 || document.querySelector('.chat-container');
    if(!container) return;
    ['mouseover','focusin'].forEach(function(evt){
      container.addEventListener(evt, function(ev){
        var row = ev.target && ev.target.closest && ev.target.closest('.message-row');
        if(isAssistantRow(row)) ensureButton(row);
      });
    });
  }

  window.LaRuche = window.LaRuche || {};
  window.LaRuche.LaReineAppel = { init: init };

  if(document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
