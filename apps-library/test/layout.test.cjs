/* Mise en page des jeux: carre, et sans barre de defilement.
 *
 * Une App vit dans un panneau dont elle ne choisit ni la largeur ni la
 * hauteur, et son plateau doit rester carre dans tous les cas. Deux defauts y
 * ont echappe longtemps parce que rien ne les mesurait:
 *
 * - une piste de grille `1fr` garde un plancher a la taille de son contenu, et
 *   2048 sortait trois lignes de 145.875 px puis une de 90.375 pour des
 *   colonnes de 132: des cases ni carrees ni egales entre elles;
 * - un `aspect-ratio` dans un flex ne sait pas retrecir sa hauteur quand
 *   `max-width` mord, donc le plateau devenait deux fois plus haut que large
 *   des qu'on dezoomait.
 *
 * Ce test mesure les vraies boites dans un vrai navigateur, a plusieurs
 * tailles de panneau et a plusieurs zooms. Il n'a besoin ni d'un noeud
 * LaRuche ni d'un modele: les cases sont posees dans le DOM, car c'est la
 * geometrie qui est en cause, pas le jeu.
 *
 *     node apps-library/test/layout.test.cjs
 *
 * Sans Chrome ni Edge, il s'annonce ignore et rend 0.
 */
'use strict';

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { spawn } = require('node:child_process');

const RACINE = path.resolve(__dirname, '..');

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.wasm': 'application/wasm',
};

/* Tailles de panneau a couvrir: large, court, etroit, et un telephone. */
const TAILLES = [[872, 900], [872, 620], [420, 820], [1400, 900], [360, 640]];
const ZOOMS = [0.6, 0.8, 1, 1.25];

const JEUX = {
  '2048': { cellule: '.tile', poser: 16, grille: 4 },
  checkers: { cellule: '.cell', poser: 64, grille: 8 },
};

function trouverNavigateur() {
  const candidats = [
    process.env.CHROME_PATH,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  ];
  return candidats.find((c) => c && fs.existsSync(c)) || null;
}

/* L'App est chargee dans un iframe, comme dans le panneau de LaRuche: en page
   pleine, c'est le document qui defile et le debordement ne se voit pas. */
function page(jeu) {
  const spec = JEUX[jeu];
  return `<!doctype html><meta charset="utf-8"><title>mesure</title>
<style>html,body{margin:0;height:100vh;overflow:hidden;background:#000}
#cadre{display:block;border:0}</style>
<iframe id="cadre" src="/index.html" sandbox="allow-scripts allow-same-origin"></iframe>
<script>
const TAILLES=${JSON.stringify(TAILLES)}, ZOOMS=${JSON.stringify(ZOOMS)};
const CELLULE=${JSON.stringify(spec.cellule)}, NB=${spec.poser}, COTE=${spec.grille};
const dormir=ms=>new Promise(r=>setTimeout(r,ms));
addEventListener('load',async function(){
  const f=document.getElementById('cadre');
  const resultats=[];
  try{
    await dormir(1200);
    const w=f.contentWindow, d=w.document;
    const plateau=d.querySelector('.plateau');
    if(!plateau) throw new Error('aucun element .plateau: le plateau ne peut pas etre carre par lui-meme');
    const b=d.getElementById('board');
    /* Une derniere ligne VIDE, comme une vraie partie. C'est ce qui revelait
       le plancher des pistes: les lignes pleines faisaient 145.875 px et la
       derniere 90.375, pour des colonnes de 132. Avec du contenu partout, les
       pistes s'egalisent et le defaut ne se voit plus. */
    b.innerHTML=Array.from({length:NB},(_,i)=>
      '<div class="'+CELLULE.slice(1)+'" style="--delay:0s">'
      +((i<COTE*2 && i%COTE<3)?'2048':'')+'</div>').join('');
    /* Un texte d'aide bien rempli: il pousse le plateau, et c'est justement ce
       qui faisait deborder l'App hors de son cadre. */
    const aide=d.getElementById('agentAide'); if(aide) aide.textContent='x'.repeat(240);
    function mesurer(nom){
      const cs=[...d.querySelectorAll(CELLULE)].map(e=>{const r=e.getBoundingClientRect();
        return [Math.round(r.width),Math.round(r.height)];});
      const p=plateau.getBoundingClientRect();
      /* La place qui lui est donnee. Un plateau plus grand qu'elle ne deborde
         pas de la page, il defile dans sa zone: la page a l'air saine et le
         plateau est pourtant coupe. C'est ce que rendait un aspect-ratio dans
         un flex, et c'est pour cela que ce controle existe a cote des
         autres. */
      const place=plateau.parentElement;
      /* Ou il tient dans sa place, ou il est exactement a son plancher, parce
         que zoomer fort finit par ne plus laisser de quoi jouer. Toute autre
         taille veut dire qu'il a pris une hauteur qu'il n'avait pas. */
      /* offsetWidth et clientWidth sont dans le meme espace; un rectangle
         client, lui, est deja mis a l'echelle par le zoom. Les melanger
         donnait un ecart d'exactement le facteur de zoom, et faisait passer
         pour un debordement une boite qui remplissait sa place. */
      const plancher=parseFloat(w.getComputedStyle(plateau).minWidth)||0;
      const tient=plateau.offsetWidth<=place.clientWidth+1 && plateau.offsetHeight<=place.clientHeight+1;
      const auPlancher=Math.abs(plateau.offsetWidth-plancher)<=1 && Math.abs(plateau.offsetHeight-plancher)<=1;
      const debordement=d.documentElement.scrollHeight-w.innerHeight;
      resultats.push({nom,
        carre: Math.abs(p.width-p.height)<=1,
        tientDansSaPlace: tient || auPlancher,
        cellulesEgales: cs.length===NB && cs.every(c=>c[0]===cs[0][0]&&c[1]===cs[0][1]),
        cellulesCarrees: cs.length===NB && cs.every(c=>Math.abs(c[0]-c[1])<=1),
        sansDebordement: debordement<=0,
        detail: plateau.offsetWidth+'x'+plateau.offsetHeight+' dans '+place.clientWidth+'x'+place.clientHeight
          +', cellule '+cs[0]+', deborde '+debordement});
    }
    for(const [lg,ht] of TAILLES){
      f.style.width=lg+'px'; f.style.height=ht+'px'; await dormir(260);
      mesurer('panneau '+lg+'x'+ht);
    }
    f.style.width='872px'; f.style.height='900px'; await dormir(220);
    for(const z of ZOOMS){
      d.documentElement.style.setProperty('--zoom',String(z)); await dormir(300);
      mesurer('zoom '+Math.round(z*100)+'%');
    }
  }catch(e){ resultats.push({nom:'mesure', carre:false, tientDansSaPlace:false, cellulesEgales:false,
    cellulesCarrees:false, sansDebordement:false, detail:String(e&&e.message||e)}); }
  await fetch('/report',{method:'POST',headers:{'Content-Type':'application/json'},
    body:JSON.stringify(resultats)});
});
</script>`;
}

function servir(jeu) {
  const UI = path.join(RACINE, jeu, 'package', 'ui');
  return new Promise((resolve) => {
    let rendu = null;
    let attente = null;
    const serveur = http.createServer((rq, rs) => {
      if (rq.method === 'POST' && rq.url === '/report') {
        let brut = '';
        rq.on('data', (c) => { brut += c; });
        rq.on('end', () => {
          try { rendu = JSON.parse(brut); } catch (e) { rendu = null; }
          rs.writeHead(204).end();
          if (attente) attente(rendu);
        });
        return;
      }
      if (rq.url === '/frame.html') {
        rs.writeHead(200, { 'Content-Type': TYPES['.html'] });
        return rs.end(page(jeu));
      }
      /* Le pont de l'hote n'existe pas ici: l'App reste sur son ecran de
         depart, et c'est sa geometrie qu'on mesure, pas son jeu. */
      if (rq.url === '/apps-runtime/v1.js') {
        rs.writeHead(200, { 'Content-Type': TYPES['.js'] });
        return rs.end('window.LaRucheApp=undefined;');
      }
      const propre = decodeURIComponent(rq.url.split('?')[0]);
      const cible = path.join(UI, propre === '/' ? 'index.html' : propre);
      if (!cible.startsWith(UI) || !fs.existsSync(cible) || fs.statSync(cible).isDirectory()) {
        return rs.writeHead(404).end();
      }
      rs.writeHead(200, { 'Content-Type': TYPES[path.extname(cible)] || 'application/octet-stream' });
      rs.end(fs.readFileSync(cible));
    });
    serveur.listen(0, '127.0.0.1', () => resolve({
      serveur,
      port: serveur.address().port,
      attendre: (delai) => new Promise((ok, ko) => {
        if (rendu) return ok(rendu);
        const t = setTimeout(() => ko(new Error('la page n\'a jamais repondu')), delai);
        attente = (r) => { clearTimeout(t); ok(r); };
      }),
    }));
  });
}

async function mesurerJeu(navigateur, jeu) {
  const hote = await servir(jeu);
  const profil = fs.mkdtempSync(path.join(os.tmpdir(), 'laruche-layout-'));
  const enfant = spawn(navigateur, [
    '--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
    '--disable-extensions', '--user-data-dir=' + profil, '--window-size=1500,1000',
    'http://127.0.0.1:' + hote.port + '/frame.html',
  ], { stdio: 'ignore', windowsHide: true });
  try {
    return await hote.attendre(60000);
  } finally {
    enfant.kill();
    hote.serveur.close();
    try { fs.rmSync(profil, { recursive: true, force: true, maxRetries: 5, retryDelay: 120 }); }
    catch (e) { /* profil temporaire encore tenu par le navigateur qui s'arrete */ }
  }
}

(async function main() {
  const navigateur = trouverNavigateur();
  if (!navigateur) {
    process.stdout.write('App layout: skipped, no Chrome or Edge found. Set CHROME_PATH to run it.\n');
    return;
  }
  let echecs = 0;
  let total = 0;
  for (const jeu of Object.keys(JEUX)) {
    const mesures = await mesurerJeu(navigateur, jeu);
    for (const m of mesures) {
      for (const [regle, ok] of [['plateau carre', m.carre], ['tient dans sa place', m.tientDansSaPlace],
        ['cellules egales', m.cellulesEgales], ['cellules carrees', m.cellulesCarrees],
        ['tient dans le cadre', m.sansDebordement]]) {
        total += 1;
        if (!ok) {
          echecs += 1;
          process.stdout.write('  FAIL  ' + jeu + ' ' + m.nom + ': ' + regle + ' -> ' + m.detail + '\n');
        }
      }
    }
  }
  process.stdout.write(echecs
    ? 'App layout: ' + echecs + ' of ' + total + ' checks failed.\n'
    : 'App layout: ' + total + ' checks passed for 2048 and checkers.\n');
  process.exitCode = echecs ? 1 : 0;
})().catch((e) => { console.error(e); process.exitCode = 1; });
