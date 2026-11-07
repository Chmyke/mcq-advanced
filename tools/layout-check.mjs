// Lays out the student's correction in Microsoft Edge (headless) at phone, tablet and desktop widths
// and fails when two pieces of a theme line overlap: the name, its bar, its success rate. On a phone
// the rate must sit under the name. The markup is the real one, written by the player's render test
// `the_correction_markup_is_handed_to_the_layout_check`; the stylesheet is the player's own.
// Then the Studio's bar and question list, from its own render test (see `studio` below).
// Run (after `cargo test`): node tools/layout-check.mjs [screenshot.png]
// The optional screenshot shows the theme lines at phone width with the longest names tried.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch } from './edge.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const fixture = join(root, 'target', 'tmp', 'player-review.html');
const shot = process.argv[2] && resolve(process.argv[2]);
if (!existsSync(fixture)) throw new Error(`No ${fixture}: run cargo test first.`);
const review = readFileSync(fixture, 'utf8');
const css = readFileSync(join(root, 'apps', 'mcq-player', 'web', 'style.css'), 'utf8');
const page = `<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><style>${css}</style></head>
<body><div class="app" data-theme="light"><main>${review}</main></div></body></html>`;

const PHONE = 560;
const WIDTHS = [320, 375, 414, 560, 768, 1280];
// Theme names as a teacher may write them: short, a long sentence, one unbreakable word.
const NAMES = [
  ['Boucles', 'Tableaux'],
  ['Programmation orientée objet : héritage, polymorphisme et interfaces', 'Les structures de contrôle conditionnelles imbriquées'],
  ['Anticonstitutionnellementinformatiquementparlant', 'Électroencéphalographiquementvôtre'],
];

// Every theme line: its three boxes, and the room it has.
const MEASURE = `(() => [...document.querySelectorAll('.theme-bars li')].map((li) => {
  const box = (e) => { const r = e.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top, bottom: r.bottom }; };
  return { line: box(li), name: box(li.querySelector('.theme-name')), bar: box(li.querySelector('.bar')), rate: box(li.querySelector('.theme-rate')) };
}))()`;

// The Studio: its bar with the Help menu open and a notice shown, over its Write space, from the
// markup and stylesheets of the Studio's render test `the_studio_markup_is_handed_to_the_layout_check`.
// No sideways scroll at any width; on a phone the menu stays on screen and the notice spans the
// width with "saved" under it; the question list ends long statements in an ellipsis without a
// scroll bar, and the add-question tip covers no question.
const STUDIO_APP = join(root, 'target', 'tmp', 'studio-app.html');
const STUDIO_WRITE = join(root, 'target', 'tmp', 'studio-write.html');
const STUDIO_WIDTHS = [375, 414, 768, 1280];
// What the static markup lacks: the open menu, a notice and the "saved" mark, as the app shows them.
const STUDIO_STATE = `(() => {
  const app = document.querySelector('.app');
  const main = document.createElement('div');
  main.className = 'studio-main';
  main.innerHTML = ${JSON.stringify(existsSync(STUDIO_WRITE) ? readFileSync(STUDIO_WRITE, 'utf8') : '')};
  app.append(main);
  const help = document.querySelector('.studio-top [aria-label^="Help"]').closest('.icon-menu');
  help.insertAdjacentHTML('beforeend', '<div class="menu-list"><a href="#">Prise en main (site)</a><a href="#">Écrire un bon QCM (site)</a><button>Copier le prompt pour l&#39;IA</button></div>');
  document.querySelector('.status-line [role=status]').innerHTML =
    '<span>Le navigateur n&#39;a pas donné accès au presse-papiers : cliquez sur une zone vide de la page et appuyez sur Ctrl+V.</span><button class="link">x</button>';
  document.querySelector('.status-line').insertAdjacentHTML('beforeend', '<span class="muted saved">Enregistré dans ce navigateur</span>');
  return 0;
})()`;
const STUDIO_MEASURE = `(() => {
  const box = (e) => { const r = e.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, width: r.width }; };
  const list = document.querySelector('.question-list');
  // A tip shows on hover: the list must not scroll sideways then either.
  let scroll = list.scrollWidth;
  const tips = [...document.querySelectorAll('.theme-head [data-icon] > .tip')].map((tip) => {
    tip.style.display = 'block';
    const r = box(tip);
    scroll = Math.max(scroll, list.scrollWidth);
    tip.style.display = '';
    return r;
  });
  const text = document.querySelector('.q-text');
  return {
    scroll: document.documentElement.scrollWidth,
    menu: box(document.querySelector('.menu-list')),
    line: box(document.querySelector('.status-line')),
    notice: box(document.querySelector('.status-line [role=status]')),
    saved: box(document.querySelector('.status-line .saved')),
    list: { ...box(list), scroll, client: list.clientWidth },
    clipped: text.scrollWidth > text.clientWidth,
    items: [...document.querySelectorAll('.q-item')].map(box),
    tips,
  };
})()`;

async function studio(send, run, frameId) {
  if (!existsSync(STUDIO_APP) || !existsSync(STUDIO_WRITE)) throw new Error('No Studio fixture: run cargo test first.');
  const html = `<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"></head><body>${readFileSync(STUDIO_APP, 'utf8')}</body></html>`;
  // Mouse and touch screens: on a touch screen the bar's labels show under their icons.
  for (const touch of [false, true]) {
    await send('Emulation.setTouchEmulationEnabled', { enabled: touch, maxTouchPoints: touch ? 5 : 1 });
    for (const width of STUDIO_WIDTHS) {
      await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: width < 768 });
      await send('Page.setDocumentContent', { frameId, html });
      await run(STUDIO_STATE);
      const m = await run(STUDIO_MEASURE);
      const where = `studio, ${width}px${touch ? ', touch' : ''}`;
      if (m.scroll > width) failures.push(`${where}: the page scrolls sideways (${m.scroll}px) with the Help menu open`);
      if (m.menu.left < -0.5 || m.menu.right > width + 0.5) failures.push(`${where}: the Help menu leaves the screen`);
      if (width <= PHONE) {
        if (m.notice.width < m.line.width * 0.8) failures.push(`${where}: the notice is squeezed (${Math.round(m.notice.width)}px)`);
        if (m.saved.top < m.notice.bottom - 0.5) failures.push(`${where}: "saved" sits beside the notice instead of under it`);
      }
      if (m.list.scroll > m.list.client + 1) failures.push(`${where}: the question list scrolls sideways`);
      // The list shows up to 70 characters of a statement: only a narrow list has to cut them.
      if (m.list.width < 420 && !m.clipped) failures.push(`${where}: the long statement is not cut by an ellipsis`);
      for (const tip of m.tips) {
        if (m.items.some((item) => overlap(tip, item))) failures.push(`${where}: the add-question tip covers a question`);
      }
    }
  }
}

const overlap = (a, b) => a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;

const { send, close } = await launch(1280, 900);
const failures = [];
try {
  await send('Page.enable');
  await send('Runtime.enable');
  await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
  const { frameTree } = await send('Page.getFrameTree');
  await send('Page.setDocumentContent', { frameId: frameTree.frame.id, html: page });
  const run = async (expression) => {
    const { result, exceptionDetails } = await send('Runtime.evaluate', { expression, returnByValue: true });
    if (exceptionDetails) throw new Error(`${exceptionDetails.text}: ${expression}`);
    return result.value;
  };
  if ((await run("document.querySelectorAll('.theme-bars li').length")) < 2) throw new Error('No theme line in the fixture.');
  for (const width of WIDTHS) {
    await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: width < 768 });
    for (const names of NAMES) {
      await run(`document.querySelectorAll('.theme-bars .theme-name').forEach((e, i) => { e.textContent = ${JSON.stringify(names)}[i % 2]; })`);
      const scroll = await run('document.documentElement.scrollWidth');
      if (scroll > width) failures.push(`${width}px: the page scrolls sideways (${scroll}px)`);
      for (const [i, { line, name, bar, rate }] of (await run(MEASURE)).entries()) {
        const where = `${width}px, "${names[i % 2].slice(0, 30)}"`;
        for (const [a, b, what] of [[name, rate, 'name and rate'], [name, bar, 'name and bar'], [bar, rate, 'bar and rate']]) {
          if (overlap(a, b)) failures.push(`${where}: ${what} overlap`);
        }
        for (const [box, what] of [[name, 'name'], [bar, 'bar'], [rate, 'rate']]) {
          if (box.right > line.right + 0.5 || box.left < line.left - 0.5) failures.push(`${where}: the ${what} leaves its line`);
        }
        if (width <= PHONE && rate.top < name.bottom - 0.5) failures.push(`${where}: the rate is not under the name`);
      }
      if (shot && width === 375 && names === NAMES[1]) {
        const clip = await run(`(() => { const r = document.querySelector('.theme-bars').getBoundingClientRect();
          return { x: 0, y: r.top + scrollY - 16, width: ${width}, height: r.height + 32, scale: 1 }; })()`);
        const { data } = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip });
        writeFileSync(shot, Buffer.from(data, 'base64'));
      }
    }
  }
  await studio(send, run, frameTree.frame.id);
} finally {
  await close();
}
if (failures.length > 0) {
  for (const f of failures) console.log(`layout: ${f}`);
  process.exit(1);
}
console.log(`layout: theme lines OK at ${WIDTHS.join(', ')} px; Studio bar and question list OK at ${STUDIO_WIDTHS.join(', ')} px`);
