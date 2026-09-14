/* La file des propositions, relue sans defaire ce que le lecteur a ouvert.
 *
 * Le panneau est rafraichi toutes les vingt secondes et il etait reconstruit
 * en entier a chaque fois. Une proposition depliee pour etre lue se refermait
 * donc toute seule au milieu de la lecture. Ce qui est mesure ici: ce que le
 * lecteur a ouvert survit a une relecture, un contenu identique ne redessine
 * rien, et l'etat d'une proposition decidee ne se reporte sur personne.
 *
 * Vrai Chrome, pas de compte ni de modele. MEMORY_JS designe une autre version.
 *     node laruche/laruche-dashboard/test/memory-proposals.test.cjs
 */
'use strict';
const fs = require('node:fs'), path = require('node:path'), os = require('node:os');
const http = require('node:http'), { spawn } = require('node:child_process');

const source = fs.readFileSync(
  process.env.MEMORY_JS || path.join(__dirname, '../src/templates/js/memory.js'), 'utf8');
const debut = source.indexOf('  /* Ce que le lecteur a deplie');
const fin = source.indexOf('  function approveProposal(');
if (debut < 0 || fin < 0) throw Error('Proposals panel markers missing');
const panneau = source.slice(debut, fin);

async function scenario() {
  const resultats = [];
  const check = (nom, ok, detail) => resultats.push({ nom, ok: !!ok, detail: String(detail || '') });
  const liste = document.getElementById('memProposalsList');
  const prop = (id, texte) => ({ id, status: 'EnAttente', target: 'projects.' + id,
    preview: 'resume ' + id, full: texte || ('contenu complet de ' + id), type: 'note',
    provenance: 'mission', risk: 'Normal' });
  const ouvert = (id) => {
    const bloc = liste.querySelector('.mem-prop-item[data-prop-id="' + id + '"]');
    const plein = bloc && bloc.querySelector('.mem-prop-full');
    return !!plein && plein.style.display !== 'none';
  };
  const deplier = (id) => liste
    .querySelector('.mem-prop-item[data-prop-id="' + id + '"] [data-prop-toggle]')
    .dispatchEvent(new MouseEvent('click', { bubbles: true }));

  try {
    const trois = [prop('a'), prop('b'), prop('c')];
    renderProposalsPanel(trois);
    check('la file s\'affiche', liste.querySelectorAll('.mem-prop-item').length === 3,
      liste.querySelectorAll('.mem-prop-item').length);
    check('tout est replie au depart', !ouvert('a') && !ouvert('b') && !ouvert('c'));

    deplier('b');
    check('un clic deplie', ouvert('b') && !ouvert('a'));

    const noeudAvant = liste.querySelector('.mem-prop-item');
    renderProposalsPanel([prop('a'), prop('b'), prop('c')]);
    check('une relecture identique ne redessine rien',
      liste.querySelector('.mem-prop-item') === noeudAvant, 'le noeud a ete remplace');
    check('ce qui etait deplie le reste', ouvert('b'));

    /* La file change vraiment: une proposition decidee disparait, une nouvelle
       arrive. Le redessin est alors necessaire, et il doit garder ce qui vit. */
    renderProposalsPanel([prop('b'), prop('c'), prop('d')]);
    check('un vrai changement redessine',
      liste.querySelector('.mem-prop-item') !== noeudAvant, 'le noeud n\'a pas bouge');
    check('le deplie survit au redessin', ouvert('b'));
    check('les autres restent replies', !ouvert('c') && !ouvert('d'));

    /* L'etat d'une proposition partie ne doit pas se reporter sur une arrivante. */
    deplier('d');
    renderProposalsPanel([prop('b'), prop('c')]);
    renderProposalsPanel([prop('b'), prop('c'), prop('d')]);
    check('l\'etat d\'une proposition partie ne revient pas', !ouvert('d'), 'd est revenue depliee');
    check('le deplie de b tient toujours', ouvert('b'));

    /* Un contenu modifie doit se voir, meme si l'identifiant ne bouge pas. */
    renderProposalsPanel([prop('b', 'texte revise'), prop('c')]);
    const texte = liste.querySelector('.mem-prop-item[data-prop-id="b"] .mem-prop-full').textContent;
    check('un contenu revise est bien affiche', texte.indexOf('texte revise') >= 0, texte.slice(0, 60));

    renderProposalsPanel([]);
    check('une file vide masque le panneau',
      document.getElementById('memProposalsPanel').style.display === 'none');
  } catch (e) { check('scenario', false, e.stack || e); }
  await fetch('/report', { method: 'POST', body: JSON.stringify(resultats) });
}

(async () => {
  const navigateur = [process.env.CHROME_PATH,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
    '/usr/bin/google-chrome', '/usr/bin/chromium'].find((p) => p && fs.existsSync(p));
  if (!navigateur) {
    process.stdout.write('Memory proposals: skipped, no Chrome or Edge found. Set CHROME_PATH to run it.\n');
    return;
  }
  let resoudre;
  const rapport = new Promise((r) => { resoudre = r; });
  const serveur = http.createServer((rq, rs) => {
    if (rq.url === '/report') {
      let corps = '';
      rq.on('data', (d) => { corps += d; });
      rq.on('end', () => { rs.end('ok'); resoudre(JSON.parse(corps)); });
      return;
    }
    rs.setHeader('Content-Type', 'text/html; charset=utf-8');
    rs.end('<div id="memProposalsPanel"><span id="memProposalsCount"></span>'
      + '<div id="memProposalsList"></div></div><script>'
      + 'function esc(s){return String(s==null?"":s).replace(/[&<>"]/g,'
      + 'function(c){return {"&":"&amp;","<":"&lt;",">":"&gt;","\\"":"&quot;"}[c];});}'
      + 'var LaRuche={i18n:{t:function(k){return k;}}};'
      + panneau + '\n(' + scenario.toString() + ')()</script>');
  });
  await new Promise((r) => serveur.listen(0, '127.0.0.1', r));
  const profil = fs.mkdtempSync(path.join(os.tmpdir(), 'laruche-memory-prop-'));
  const enfant = spawn(navigateur, ['--headless=new', '--disable-gpu', '--no-first-run',
    '--no-default-browser-check', '--user-data-dir=' + profil,
    'http://127.0.0.1:' + serveur.address().port + '/'], { stdio: 'ignore', windowsHide: true });
  const minuteur = setTimeout(() => resoudre([{ nom: 'la page n\'a jamais repondu', ok: false, detail: '' }]), 30000);
  const resultats = await rapport;
  clearTimeout(minuteur);
  enfant.kill();
  serveur.close();
  try { fs.rmSync(profil, { recursive: true, force: true, maxRetries: 5, retryDelay: 120 }); } catch (e) {}
  const rates = resultats.filter((r) => !r.ok);
  rates.forEach((r) => process.stdout.write('  FAIL  ' + r.nom + (r.detail ? ' -> ' + r.detail : '') + '\n'));
  process.stdout.write(rates.length
    ? 'Memory proposals: ' + rates.length + ' of ' + resultats.length + ' failed.\n'
    : 'Memory proposals: ' + resultats.length + '/' + resultats.length + ' passed.\n');
  process.exitCode = rates.length ? 1 : 0;
})().catch((e) => { console.error(e); process.exitCode = 1; });
