/* ── Table ronde: la deliberation multi-agents ─────────────────────────────
   Deux onglets discrets dans le chat: la conversation habituelle, et la table.
   Le chat reste le mode par defaut - la deliberation est un detour qu'on choisit,
   pas un mode dans lequel on tombe.

   Ce que cette vue montre, et pourquoi c'est dans cet ordre:

   1. la table, avec un indicateur par specialiste. Les indicateurs viennent des
      champs DECLARES par chacun (accord, confiance, changement), jamais d'une
      deduction de notre part: un accord que nous inferions serait notre lecture
      presentee comme son avis.
   2. les DESACCORDS, avant la synthese. C'est l'information qu'une table ronde
      produit et qu'un modele seul ne donne jamais; l'enterrer sous un resume
      ferait perdre tout l'interet du dispositif.
   3. le transcript, depliable par tour.

   Et volontairement AUCUN pourcentage de consensus. Un accord entre modeles
   mesure la conformite, pas la justesse, et serait lu comme une confiance -
   l'inverse exact de ce que dit la constitution. */
LaRuche.TableRonde = (function(){
  var pool = [];
  var missions = [];
  var missionActive = 'reponse';
  var dernier = null;      // le dernier debat rendu
  var enCours = false;
  var question = '';       // la question posee, affichee en tete
  var actifs = [];         // qui reflechit EN CE MOMENT
  var etapeNom = '';
  var flux = [];           // interventions recues au fil de l'eau

  function esc(t){ return LaRuche.Utils.esc(t); }
  function specDe(id){ for(var i=0;i<pool.length;i++){ if(pool[i].id===id) return pool[i]; } return null; }
  function nomDe(id){ var s=specDe(id); return s?s.nom:id; }

  /* ── Chargement ──────────────────────────────────────────────────────── */
  async function charger(){
    try{
      var d = await fetch('/api/deliberation/pool').then(function(r){ return r.json(); });
      pool = d.specialistes||[]; missions = d.missions||[];
    }catch(e){ pool=[]; missions=[]; }
  }

  /* Un avatar est soit un emoji, soit une image deposee (data-URL). Les deux
     doivent etre rendus, jamais echappes tels quels: une data-URL affichee en
     texte brut deborde sur toute la largeur de l'ecran. */
  function avatarHtml(v){
    if(v && v.indexOf('data:') === 0) return '<img class="tr-av-img" src="'+esc(v)+'" alt="">';
    return esc(v || '●');
  }

  /* ── La table ────────────────────────────────────────────────────────── */
  function indicateurs(id){
    // Pendant le debat on lit le flux, une fois fini la repartition finale: les
    // indicateurs vivent donc DES le premier tour, au lieu d'apparaitre a la fin.
    var derniere = null;
    flux.forEach(function(iv){ if(iv.specialiste===id) derniere = iv; });
    var r = null, i;
    if(dernier){
      for(i=0;i<(dernier.repartition||[]).length;i++){
        if(dernier.repartition[i].specialiste===id) r = dernier.repartition[i];
      }
    }
    if(!r && derniere){
      var sym = {approuve:'✔', reserve:'⚠', oppose:'✖'}[derniere.accord] || '⚠';
      r = { accord: derniere.accord, symbole: sym, confiance: derniere.confiance };
    }
    if(!r) return '';
    var bouge = false;
    if(derniere){
      var c=(derniere.changement||'').trim().toLowerCase();
      bouge = !!c && c!=='aucun' && c!=='aucune' && c!=='-';
    }
    var cls = r.accord==='approuve' ? 'ok' : (r.accord==='oppose' ? 'ko' : 'mid');
    return '<span class="tr-ind tr-ind--'+cls+'" title="'+esc(r.accord)+'">'+esc(r.symbole)+'</span>'+
           '<span class="tr-conf" title="confiance declaree">'+r.confiance+'</span>'+
           (bouge ? '<span class="tr-bouge" title="a change d\'avis a ce tour">↻</span>' : '');
  }

  function placeHtml(s){
    // Seul celui qui parle a ce tour s'anime. Animer toute la table donnait
    // l'impression que tout le monde travaille en permanence, ce qui est faux et
    // empeche de voir la delegation se faire.
    var actif = actifs.indexOf(s.id) >= 0 ? ' tr-place--actif' : '';
    return '<div class="tr-place'+(s.embauche?'':' tr-place--reserve')+actif+'" data-id="'+esc(s.id)+'" '+
             'style="--tr-couleur:'+esc(s.couleur||'#f59e0b')+'" title="'+esc(s.mission||'')+'">'+
             '<div class="tr-avatar">'+avatarHtml(s.avatar)+'</div>'+
             '<div class="tr-nom">'+esc(s.nom)+'</div>'+
             '<div class="tr-indics">'+indicateurs(s.id)+'</div>'+
           '</div>';
  }

  function tableHtml(){
    var embauches = pool.filter(function(s){ return s.embauche && s.role!=='orchestrateur' && s.role!=='arbitre'; });
    var orch = pool.filter(function(s){ return s.role==='orchestrateur'; });
    var arb  = pool.filter(function(s){ return s.role==='arbitre'; });
    return '<div class="tr-table">'+
             '<div class="tr-rang tr-rang--haut">'+orch.map(placeHtml).join('')+'</div>'+
             '<div class="tr-rang tr-rang--cercle">'+
               (embauches.length ? embauches.map(placeHtml).join('')
                                 : '<div class="tr-vide">'+LaRuche.i18n.t('tr.aucunEmbauche')+'</div>')+
             '</div>'+
             '<div class="tr-rang tr-rang--bas">'+arb.map(placeHtml).join('')+'</div>'+
           '</div>';
  }

  /* ── Le resultat ─────────────────────────────────────────────────────── */
  /* La reponse finale, en evidence. Elle etait noyee: l'arbitre apparaissait comme
     une intervention parmi d'autres au fond du transcript, et la ligne « aucun
     desaccord » pouvait passer pour la conclusion alors qu'elle ne dit rien du
     fond - seulement que personne ne s'oppose. */
  function verdictHtml(){
    if(!dernier) return '';
    var arb = null;
    (dernier.interventions||[]).forEach(function(iv){
      var sp = specDe(iv.specialiste);
      if(sp && sp.role === 'arbitre') arb = iv;
    });
    if(!arb) return '';
    var sp = specDe(arb.specialiste);
    return '<div class="tr-verdict">'+
      '<div class="tr-verdict-tete">'+avatarHtml(sp && sp.avatar)+
        '<span class="tr-verdict-eti">'+LaRuche.i18n.t('tr.verdict')+'</span>'+
        '<span class="tr-note">'+esc(nomDe(arb.specialiste))+' · '+
          LaRuche.i18n.t('tr.confiance')+' '+arb.confiance+'</span>'+
      '</div>'+
      '<div class="tr-verdict-corps">'+esc(arb.position)+'</div>'+
    '</div>';
  }

  function desaccordsHtml(){
    if(!dernier) return '';
    var d = dernier.dissidents||[];
    if(!d.length){
      return '<div class="tr-bloc tr-bloc--accord">'+
               '<div class="tr-bloc-titre">'+LaRuche.i18n.t('tr.aucunDesaccord')+'</div>'+
               '<div class="tr-note">'+LaRuche.i18n.t('tr.accordNote')+'</div>'+
             '</div>';
    }
    return '<div class="tr-bloc tr-bloc--desaccord">'+
             '<div class="tr-bloc-titre">'+LaRuche.i18n.t('tr.desaccords')+'</div>'+
             d.map(function(id){
               var pos='';
               (dernier.interventions||[]).forEach(function(iv){ if(iv.specialiste===id) pos=iv.position; });
               return '<div class="tr-dissident"><strong>'+esc(nomDe(id))+'</strong> — '+
                      esc(String(pos).slice(0,400))+'</div>';
             }).join('')+
           '</div>';
  }

  function transcriptHtml(){
    if(!dernier || !(dernier.interventions||[]).length) return '';
    var parTour = {};
    dernier.interventions.forEach(function(iv){
      (parTour[iv.tour] = parTour[iv.tour] || []).push(iv);
    });
    var tours = Object.keys(parTour).sort(function(a,b){ return a-b; });
    return '<div class="tr-bloc">'+
      '<div class="tr-bloc-titre">'+LaRuche.i18n.t('tr.transcript')+'</div>'+
      tours.map(function(t){
        return '<details class="tr-tour"><summary>'+LaRuche.i18n.t('tr.tour',{n:t})+
                 ' <span class="tr-note">('+parTour[t].length+')</span></summary>'+
          parTour[t].map(function(iv){
            var s = specDe(iv.specialiste);
            return '<div class="tr-inter" style="--tr-couleur:'+esc((s&&s.couleur)||'#8a8a92')+'">'+
              '<div class="tr-inter-tete">'+avatarHtml(s&&s.avatar)+' <strong>'+esc(nomDe(iv.specialiste))+'</strong>'+
                '<span class="tr-note">confiance '+iv.confiance+'</span></div>'+
              (iv.changement && iv.changement.toLowerCase().indexOf('aucun')<0
                 ? '<div class="tr-change">↻ '+esc(iv.changement)+'</div>' : '')+
              (iv.refutable ? '<div class="tr-note">réfutable si : '+esc(iv.refutable)+'</div>' : '')+
              '<div class="tr-position">'+esc(iv.position)+'</div>'+
            '</div>';
          }).join('')+
        '</details>';
      }).join('')+
    '</div>';
  }

  /* Les interventions au fil de l'eau: on voit la reflexion arriver, plutot que
     d'attendre plusieurs minutes devant une table qui s'agite. */
  function fluxHtml(){
    if(!flux.length) return '';
    return '<div class="tr-bloc">'+
      '<div class="tr-bloc-titre">'+LaRuche.i18n.t('tr.enDirect')+'</div>'+
      flux.slice().reverse().map(function(iv){
        var s = specDe(iv.specialiste);
        return '<div class="tr-inter tr-inter--neuf" style="--tr-couleur:'+esc((s&&s.couleur)||'#8a8a92')+'">'+
          '<div class="tr-inter-tete">'+avatarHtml(s&&s.avatar)+' <strong>'+esc(nomDe(iv.specialiste))+'</strong>'+
            '<span class="tr-note">'+LaRuche.i18n.t('tr.tour',{n:iv.tour})+' · confiance '+iv.confiance+'</span></div>'+
          (iv.changement && iv.changement.toLowerCase().indexOf('aucun')<0
             ? '<div class="tr-change">↻ '+esc(iv.changement)+'</div>' : '')+
          '<div class="tr-position">'+esc(iv.position)+'</div>'+
        '</div>';
      }).join('')+
    '</div>';
  }

  function bilanHtml(){
    if(!dernier) return '';
    var arret = {convergence:'tr.arretConvergence', tours_epuises:'tr.arretTours',
                 budget_epuise:'tr.arretBudget', vide:'tr.arretVide'}[dernier.arret];
    return '<div class="tr-bilan">'+
      '<span>'+LaRuche.i18n.t('tr.tours')+' <strong>'+(dernier.tours||0)+'</strong></span>'+
      '<span>'+LaRuche.i18n.t('tr.jetons')+' <strong>'+(dernier.jetons||0)+'</strong></span>'+
      (arret ? '<span class="tr-note">'+LaRuche.i18n.t(arret)+'</span>' : '')+
    '</div>';
  }

  /* ── Rendu ───────────────────────────────────────────────────────────── */
  function rendre(){
    var el = document.getElementById('tableRonde');
    if(!el) return;
    var m = missions.map(function(mi){
      return '<option value="'+esc(mi.id)+'"'+(mi.id===missionActive?' selected':'')+'>'+
             esc(mi.nom)+' · '+esc(mi.livrable||mi.acces)+'</option>';
    }).join('');
    // Tant qu'aucun outil n'est ouvert, on le DIT. Annoncer « ecriture de fichiers »
    // a quelqu'un qui demande de creer un site sur son bureau est un mensonge par
    // omission - et il s'en apercoit seulement apres avoir paye le debat.
    var miss = null;
    missions.forEach(function(mi){ if(mi.id===missionActive) miss = mi; });
    var sansOutils = miss && miss.outils === false && missionActive !== 'reponse';
    var etapes = {solo:'tr.etapeSolo', relecture:'tr.etapeRelecture',
                  contradiction:'tr.etapeContradiction', reponse:'tr.etapeReponse',
                  synthese:'tr.etapeSynthese'};
    el.innerHTML =
      '<div class="tr-barre">'+
        '<select id="trMission" class="tr-select">'+m+'</select>'+
        '<button class="tl-btn" id="trGerer">'+LaRuche.i18n.t('tr.gerer')+'</button>'+
        (enCours ? '<span class="tr-encours">'+LaRuche.i18n.t('tr.encours')+'</span>' : '')+
      '</div>'+
      (sansOutils ? '<div class="tr-avertit">'+LaRuche.i18n.t('tr.sansOutils')+'</div>' : '')+
      // La question reste a l'ecran. Elle disparaissait a l'envoi: on voyait la
      // table s'agiter sans plus savoir sur quoi.
      (question ? '<div class="tr-question"><span class="tr-question-eti">'+
                  LaRuche.i18n.t('tr.question')+'</span>'+esc(question)+'</div>' : '')+
      (enCours && etapeNom
        ? '<div class="tr-etape">'+esc(LaRuche.i18n.t(etapes[etapeNom]||'tr.encours'))+
          (actifs.length ? ' — '+actifs.map(function(id){ return esc(nomDe(id)); }).join(', ') : '')+
          '</div>'
        : '')+
      tableHtml()+
      // Le flux en direct pendant le debat, le bilan complet apres.
      (enCours ? fluxHtml() : verdictHtml()+desaccordsHtml()+bilanHtml()+transcriptHtml());

    var sel = document.getElementById('trMission');
    if(sel) sel.onchange = function(){ missionActive = sel.value; };
    var g = document.getElementById('trGerer');
    if(g) g.onclick = ouvrirPool;
  }

  /* ── Lancer un debat ─────────────────────────────────────────────────── */
  async function lancer(q){
    if(enCours || !q || !q.trim()) return;
    enCours = true; dernier = null; question = q.trim();
    flux = []; actifs = []; etapeNom = '';
    rendre();
    try{
      var rep = await fetch('/api/deliberation/run', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body: JSON.stringify({ question: question, mission: missionActive })
      });
      // NDJSON: une ligne par evenement. On lit au fur et a mesure plutot que
      // d'attendre la fin - c'est tout l'interet, et ca evite au navigateur
      // d'abandonner sur un debat de plusieurs minutes.
      var lecteur = rep.body.getReader();
      var dec = new TextDecoder();
      var reste = '';
      while(true){
        var bloc = await lecteur.read();
        if(bloc.done) break;
        reste += dec.decode(bloc.value, {stream:true});
        var lignes = reste.split('\n');
        reste = lignes.pop();  // la derniere peut etre incomplete
        lignes.forEach(function(l){
          if(!l.trim()) return;
          var ev; try{ ev = JSON.parse(l); }catch(e){ return; }
          if(ev.type==='debut'){ question = ev.question || question; }
          else if(ev.type==='etape'){ etapeNom = ev.nom; actifs = ev.acteurs||[]; }
          else if(ev.type==='intervention'){
            flux.push(ev.intervention);
            // Celui qui vient de parler n'y reflechit plus.
            actifs = actifs.filter(function(id){ return id !== ev.intervention.specialiste; });
          }
          else if(ev.type==='fin'){
            actifs = []; etapeNom = '';
            if(ev.erreur){ LaRuche.Toast.show(ev.erreur, 'error'); }
            else {
              dernier = Object.assign({}, ev, { interventions: flux });
              // Le debat vient d'etre enregistre cote noeud: la liste doit le montrer.
              chargerTours();
            }
          }
          rendre();
        });
      }
    }catch(e){
      LaRuche.Toast.show(LaRuche.i18n.t('tr.echec'), 'error');
    }
    enCours = false; actifs = []; etapeNom = ''; rendre();
  }

  /* ── Gestion du pool ─────────────────────────────────────────────────── */
  /* Avatar: emoji tape a la main, ou image deposee. Meme mecanique que la photo
     de profil - redimensionnee dans un canvas puis stockee en data-URL, pour que
     rien n'ait a etre servi depuis un fichier separe. */
  function choisirImage(surCharge){
    var inp = document.createElement('input');
    inp.type = 'file'; inp.accept = 'image/*';
    inp.onchange = function(){
      var f = inp.files && inp.files[0];
      if(!f) return;
      var fr = new FileReader();
      fr.onload = function(){
        var img = new Image();
        img.onload = function(){
          // 96 px suffit: l'avatar s'affiche a 22 px, et une image de 4 Mo dans un
          // fichier de configuration serait absurde.
          var c = document.createElement('canvas');
          c.width = c.height = 96;
          var g = c.getContext('2d');
          var cote = Math.min(img.width, img.height);
          g.drawImage(img, (img.width-cote)/2, (img.height-cote)/2, cote, cote, 0, 0, 96, 96);
          surCharge(c.toDataURL('image/png'));
        };
        img.src = fr.result;
      };
      fr.readAsDataURL(f);
    };
    inp.click();
  }

  function ligneHtml(s){
    var estImage = !!(s.avatar && s.avatar.indexOf('data:') === 0);
    return '<div class="tr-pool-ligne" data-id="'+esc(s.id)+'">'+
      '<label class="tr-pool-embauche" title="'+esc(LaRuche.i18n.t('tr.embaucher'))+'">'+
        '<input type="checkbox" class="tr-emb" '+(s.embauche?'checked':'')+'></label>'+
      '<button class="tr-av-btn" title="'+esc(LaRuche.i18n.t('tr.avatarAide'))+'">'+avatarHtml(s.avatar)+'</button>'+
      '<input type="text" class="tr-av" value="'+esc(estImage ? '' : (s.avatar||''))+'" maxlength="4" placeholder="🙂">'+
      '<input type="text" class="tr-nom-in" value="'+esc(s.nom)+'">'+
      '<select class="tr-role">'+
        ['contributeur','contradicteur','arbitre','orchestrateur'].map(function(r){
          return '<option value="'+r+'"'+(s.role===r?' selected':'')+'>'+r+'</option>';
        }).join('')+
      '</select>'+
      '<input type="text" class="tr-prof" value="'+esc(s.profil||'')+'" placeholder="'+esc(LaRuche.i18n.t('tr.profilDefaut'))+'">'+
      '<button class="tr-strat-btn" title="'+esc(LaRuche.i18n.t('tr.editerStrategie'))+'">✎</button>'+
      '<button class="tr-sup-btn" title="'+esc(LaRuche.i18n.t('tr.retirer'))+'">✕</button>'+
      '<textarea class="tr-strat" rows="7" hidden>'+esc(s.strategie||'')+'</textarea>'+
      '<input type="hidden" class="tr-couleur" value="'+esc(s.couleur||'#f59e0b')+'">'+
      '<input type="hidden" class="tr-mission" value="'+esc(s.mission||'')+'">'+
      '<input type="hidden" class="tr-avdata" value="'+esc(estImage ? s.avatar : '')+'">'+
    '</div>';
  }

  function brancherLigne(l){
    l.querySelector('.tr-av-btn').onclick = function(){
      choisirImage(function(url){
        l.querySelector('.tr-avdata').value = url;
        l.querySelector('.tr-av').value = '';
        l.querySelector('.tr-av-btn').innerHTML = '<img class="tr-av-img" src="'+url+'" alt="">';
      });
    };
    l.querySelector('.tr-strat-btn').onclick = function(){
      var t = l.querySelector('.tr-strat');
      t.hidden = !t.hidden;
      if(!t.hidden) t.focus();
    };
    l.querySelector('.tr-sup-btn').onclick = function(){ l.remove(); };
  }

  function ouvrirPool(){
    var ov = document.createElement('div');
    ov.className = 'lr-modal-ov'; ov.id = 'trPoolModal';
    ov.innerHTML = '<div class="lr-modal tr-pool" role="dialog" aria-modal="true">'+
      '<h3>'+LaRuche.i18n.t('tr.poolTitre')+'</h3>'+
      '<p class="lr-modal-sub">'+LaRuche.i18n.t('tr.poolSous')+'</p>'+
      '<div class="tr-pool-liste" id="trPoolListe">'+pool.map(ligneHtml).join('')+'</div>'+
      '<button class="tl-btn" id="trAjouter">'+LaRuche.i18n.t('tr.ajouter')+'</button>'+
      '<div class="lr-modal-hint">'+LaRuche.i18n.t('tr.poolAide')+'</div>'+
      '<div class="lr-modal-actions">'+
        '<button class="tl-btn" id="trPoolAnnuler">'+LaRuche.i18n.t('common.cancel')+'</button>'+
        '<button class="tl-btn tl-btn--active" id="trPoolSauver">'+LaRuche.i18n.t('common.save')+'</button>'+
      '</div></div>';
    document.body.appendChild(ov);
    ov.addEventListener('click', function(e){ if(e.target===ov) ov.remove(); });
    ov.querySelectorAll('.tr-pool-ligne').forEach(brancherLigne);

    document.getElementById('trAjouter').onclick = function(){
      var id = 'perso-' + Date.now().toString(36);
      var neuf = { id:id, nom:LaRuche.i18n.t('tr.nouveau'), avatar:'🙂', couleur:'#8a8a92',
                   role:'contributeur', mission:'', strategie:'', profil:'',
                   embauche:false, ordre:50, livre:false };
      var liste = document.getElementById('trPoolListe');
      liste.insertAdjacentHTML('beforeend', ligneHtml(neuf));
      var l = liste.lastElementChild;
      brancherLigne(l);
      // Strategie ouverte d'emblee: un specialiste sans strategie n'est qu'un nom,
      // et c'est justement ce qu'il reste a ecrire.
      l.querySelector('.tr-strat').hidden = false;
      l.querySelector('.tr-nom-in').focus();
    };
    document.getElementById('trPoolAnnuler').onclick = function(){ ov.remove(); };
    document.getElementById('trPoolSauver').onclick = async function(){
      var liste = [];
      ov.querySelectorAll('.tr-pool-ligne').forEach(function(l){
        var id = l.getAttribute('data-id');
        var base = specDe(id) || {};
        var img = l.querySelector('.tr-avdata').value;
        var emo = l.querySelector('.tr-av').value.trim();
        // On renvoie le specialiste ENTIER: le fichier personnalise REMPLACE
        // l'entree livree, il ne la complete pas. Renvoyer un objet partiel
        // effacerait la strategie de ceux qu'on n'a pas touches.
        liste.push(Object.assign({}, base, {
          id:        id,
          nom:       l.querySelector('.tr-nom-in').value || id,
          avatar:    img || emo || '●',
          couleur:   l.querySelector('.tr-couleur').value,
          role:      l.querySelector('.tr-role').value,
          mission:   l.querySelector('.tr-mission').value,
          strategie: l.querySelector('.tr-strat').value,
          profil:    l.querySelector('.tr-prof').value.trim(),
          embauche:  l.querySelector('.tr-emb').checked,
          ordre:     base.ordre != null ? base.ordre : 50
        }));
      });
      try{
        await fetch('/api/deliberation/pool', {
          method:'POST', headers:{'Content-Type':'application/json'},
          body: JSON.stringify(liste)
        });
        await charger(); rendre(); ov.remove();
        LaRuche.Toast.show(LaRuche.i18n.t('toast.saved'), 'success');
      }catch(e){ LaRuche.Toast.show(LaRuche.i18n.t('toast.failed'), 'error'); }
    };
  }

  /* ── Volet lateral: les tours de table remplacent les conversations ──── */
  async function chargerTours(){
    var liste = document.getElementById('toursSidebarList');
    if(!liste) return;
    var tours = [];
    try{
      tours = (await fetch('/api/deliberation/tours').then(function(r){ return r.json(); })).tours||[];
    }catch(e){}
    if(!tours.length){
      liste.innerHTML = '<div class="tr-note" style="padding:8px 12px">'+
        LaRuche.i18n.t('tr.aucunTour')+'</div>';
      return;
    }
    liste.innerHTML = tours.map(function(t){
      var n = (t.dissidents||[]).length;
      return '<div class="session-item tr-tour-item" data-id="'+esc(t.id)+'">'+
        '<span class="tr-tour-q">'+esc(String(t.question||'').slice(0,64))+'</span>'+
        '<span class="tr-note">'+(t.tours||0)+' '+LaRuche.i18n.t('tr.toursCourt')+
          (n ? ' · '+n+' '+LaRuche.i18n.t('tr.dissidentsCourt') : '')+'</span>'+
      '</div>';
    }).join('');
    liste.querySelectorAll('.tr-tour-item').forEach(function(el){
      el.onclick = function(){ ouvrirTour(el.getAttribute('data-id')); };
    });
  }

  async function ouvrirTour(id){
    try{
      var t = await fetch('/api/deliberation/tour/'+encodeURIComponent(id))
                .then(function(r){ return r.json(); });
      if(t.error){ LaRuche.Toast.show(t.error, 'error'); return; }
      question = t.question || '';
      flux = t.interventions || [];
      dernier = t;
      enCours = false; actifs = []; etapeNom = '';
      rendre();
    }catch(e){ LaRuche.Toast.show(LaRuche.i18n.t('toast.failed'), 'error'); }
  }

  function nouveauTour(){
    question = ''; flux = []; dernier = null; actifs = []; etapeNom = '';
    rendre();
    var champ = document.getElementById('userInput');
    if(champ) champ.focus();
  }

  /* ── Onglets ─────────────────────────────────────────────────────────── */
  function basculer(vue){
    var chat = document.getElementById('chatContainer');
    var tr = document.getElementById('tableRonde');
    if(!chat || !tr) return;
    var surTable = vue === 'table';
    chat.style.display = surTable ? 'none' : '';
    tr.style.display = surTable ? '' : 'none';
    document.querySelectorAll('#chatOnglets .chat-onglet').forEach(function(b){
      b.classList.toggle('actif', b.dataset.vue === vue);
    });
    // Le volet suit la vue: les debats passes remplacent les conversations.
    var vConv = document.getElementById('sessionsSidebarSection');
    var vTours = document.getElementById('toursSidebarSection');
    if(vConv) vConv.style.display = surTable ? 'none' : '';
    if(vTours){ vTours.style.display = surTable ? '' : 'none'; if(surTable) chargerTours(); }
    if(surTable){ if(!pool.length){ charger().then(rendre); } else { rendre(); } }
  }

  function surTable(){
    var tr = document.getElementById('tableRonde');
    return !!(tr && tr.style.display !== 'none');
  }

  function init(){
    var neuf = document.getElementById('trNouveau');
    if(neuf) neuf.onclick = nouveauTour;
    var barre = document.getElementById('chatOnglets');
    if(barre){
      barre.addEventListener('click', function(e){
        var b = e.target.closest('.chat-onglet');
        if(b) basculer(b.dataset.vue);
      });
    }
  }

  return { init:init, basculer:basculer, lancer:lancer, surTable:surTable,
           ouvrirPool:ouvrirPool, nouveauTour:nouveauTour, ouvrirTour:ouvrirTour };
})();

LaRuche.i18n.add({
  'tr.gerer':         { fr:"Gérer l'équipe", en:'Manage the team' },
  'tr.encours':       { fr:'Délibération en cours…', en:'Deliberating…' },
  'tr.aucunEmbauche': { fr:'Aucun spécialiste embauché.', en:'No specialist hired.' },
  'tr.desaccords':    { fr:'Désaccords', en:'Disagreements' },
  'tr.aucunDesaccord':{ fr:'Aucun désaccord', en:'No disagreement' },
  'tr.accordNote':    { fr:"Un accord entre modèles mesure la conformité, pas la justesse. Il ne vaut que ce que valent les arguments.",
                        en:'Agreement between models measures conformity, not correctness. It is worth only what the arguments are worth.' },
  'tr.transcript':    { fr:'Transcript', en:'Transcript' },
  'tr.tour':          { fr:'Tour {n}', en:'Round {n}' },
  'tr.tours':         { fr:'Tours', en:'Rounds' },
  'tr.jetons':        { fr:'Jetons', en:'Tokens' },
  'tr.arretConvergence':{ fr:'arrêt : les positions ont cessé de bouger', en:'stopped: positions stopped moving' },
  'tr.arretTours':    { fr:'arrêt : plafond de tours atteint', en:'stopped: round limit reached' },
  'tr.arretBudget':   { fr:'arrêt : plafond de jetons atteint', en:'stopped: token budget reached' },
  'tr.arretVide':     { fr:'arrêt : personne n’a répondu', en:'stopped: nobody answered' },
  'tr.echec':         { fr:'La délibération a échoué', en:'Deliberation failed' },
  'tr.poolTitre':     { fr:"L'équipe", en:'The team' },
  'tr.poolSous':      { fr:"Embauche, avatar, nom et fournisseur. La stratégie de raisonnement s'édite dans les réglages.",
                        en:'Hiring, avatar, name and provider. The reasoning strategy is edited in Settings.' },
  'tr.poolAide':      { fr:"Un fournisseur vide = le profil actif. Faire varier les modèles est le seul moyen d'avoir de vrais désaccords : dix stratégies sur un même modèle partagent ses angles morts.",
                        en:'Empty provider = active profile. Varying models is the only way to get real disagreement: ten strategies on one model share its blind spots.' },
  'tr.profilDefaut':  { fr:'profil actif', en:'active profile' },
  'tr.question':      { fr:'Question', en:'Question' },
  'tr.verdict':       { fr:'Réponse finale', en:'Final answer' },
  'tr.sansOutils':    { fr:"Aucun outil n'est encore ouvert : les spécialistes raisonnent et rendent du texte. Ils n'écrivent aucun fichier et ne consultent aucune page.",
                        en:'No tools are wired yet: the specialists reason and return text. They write no files and browse nothing.' },
  'tr.aucunTour':     { fr:'Aucun tour de table.', en:'No round table yet.' },
  'tr.toursCourt':    { fr:'tours', en:'rounds' },
  'tr.dissidentsCourt':{ fr:'dissident(s)', en:'dissenter(s)' },
  'tr.confiance':     { fr:'confiance', en:'confidence' },
  'tr.ajouter':       { fr:'+ Nouveau spécialiste', en:'+ New specialist' },
  'tr.nouveau':       { fr:'Sans nom', en:'Unnamed' },
  'tr.embaucher':     { fr:'Embaucher pour les délibérations', en:'Hire for deliberations' },
  'tr.avatarAide':    { fr:'Cliquer pour choisir une image', en:'Click to pick an image' },
  'tr.editerStrategie':{ fr:'Éditer la stratégie de raisonnement', en:'Edit the reasoning strategy' },
  'tr.retirer':       { fr:'Retirer du pool', en:'Remove from pool' },
  'tr.enDirect':      { fr:'En direct', en:'Live' },
  'tr.etapeSolo':          { fr:'Chacun réfléchit seul', en:'Each thinking alone' },
  'tr.etapeRelecture':     { fr:'Chacun relit les autres', en:'Each reading the others' },
  'tr.etapeContradiction': { fr:'Le contradicteur attaque', en:'The contrarian attacks' },
  'tr.etapeReponse':       { fr:'Réponses aux objections', en:'Answering objections' },
  'tr.etapeSynthese':      { fr:"L'arbitre tranche", en:'The arbiter decides' }
});
