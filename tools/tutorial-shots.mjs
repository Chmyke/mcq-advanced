// Takes the screenshots of the website tutorial ("my first paper in five minutes") from the web
// builds themselves, so they never drift from the interface: a local server serves the site folder,
// Microsoft Edge (headless) opens the Studio and types the tutorial's question like a teacher would.
// Every step checks that what it shows is on screen, and fails loudly otherwise.
// Run: node tools/tutorial-shots.mjs <site folder with studio/> <output folder>
// Writes <output>/<lang>/01-open.webp ... 05-publish.webp for each language; nothing else.
// No dependency: Node's own http server and WebSocket, Edge's DevTools protocol.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize, resolve, sep } from 'node:path';
import { EDGE, launch, sleep } from './edge.mjs';

const [site, out] = process.argv.slice(2).map((p) => p && resolve(p));
if (!site || !out) {
  console.error('usage: node tools/tutorial-shots.mjs <site folder> <output folder>');
  process.exit(2);
}
const WIDTH = 1280;
const HEIGHT = 820;
if (!EDGE) throw new Error('Microsoft Edge is required to take the tutorial screenshots.');

// The question the tutorial writes, in each language: a correct answer and three distractors that
// are each a real mistake, every option explained. The rule and the revision place are left out on
// purpose, so the validator has advice to show at step 3.
const LESSONS = {
  fr: {
    statement: 'Combien vaut 7 × 8 ?',
    answers: [
      ['56', 'Sept fois huit, cinquante-six.'],
      ['54', 'C\'est 6 × 9 : les deux résultats se confondent souvent.'],
      ['48', 'C\'est 6 × 8 : une fois huit de moins.'],
      ['63', 'C\'est 7 × 9 : une fois sept de trop.'],
    ],
  },
  en: {
    statement: 'What is 7 × 8?',
    answers: [
      ['56', 'Seven times eight, fifty-six.'],
      ['54', 'That is 6 × 9: the two results are often mixed up.'],
      ['48', 'That is 6 × 8: one eight short.'],
      ['63', 'That is 7 × 9: one seven too many.'],
    ],
  },
};

const TYPES = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.wasm': 'application/wasm',
  '.css': 'text/css', '.svg': 'image/svg+xml', '.json': 'application/json', '.txt': 'text/plain',
};

function serve(root) {
  const server = createServer((request, response) => {
    let path = decodeURIComponent(new URL(request.url, 'http://x').pathname);
    if (path.endsWith('/')) path += 'index.html';
    const file = normalize(join(root, path));
    if (!file.startsWith(root + sep) || !existsSync(file)) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, { 'Content-Type': TYPES[extname(file)] ?? 'application/octet-stream' });
    response.end(readFileSync(file));
  });
  return new Promise((ok) => server.listen(0, '127.0.0.1', () => ok(server)));
}

// A page driven step by step; `run` evaluates JavaScript in it and returns the value.
function driver(send) {
  const run = async (expression) => {
    const { result, exceptionDetails } = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (exceptionDetails) throw new Error(`${exceptionDetails.text}: ${expression}`);
    return result.value;
  };
  const waitFor = async (expression, what) => {
    for (let i = 0; i < 150; i++) {
      if (await run(`!!(${expression})`)) return;
      await sleep(100);
    }
    throw new Error(`Not on screen: ${what}`);
  };
  const click = async (expression, what) => {
    await waitFor(expression, what);
    await run(`(${expression}).click()`);
    await sleep(250);
  };
  const type = async (expression, text, what) => {
    await waitFor(expression, what);
    await run(`(${expression}).focus()`);
    await send('Input.insertText', { text });
    await sleep(120);
  };
  // The box around what the shot shows (one element, or several from the first to the last), a
  // little margin around it, within the page; the whole window without an expression.
  const shoot = async (file, expression) => {
    // From the top: a sticky bar would otherwise cover what is below it.
    await run('document.activeElement && document.activeElement.blur(); scrollTo(0, 0)');
    await sleep(300);
    let clip = { x: 0, y: 0, width: WIDTH, height: HEIGHT };
    if (expression) {
      await waitFor(expression, file);
      clip = await run(`(() => { const shown = [].concat(${expression}).map((e) => e.getBoundingClientRect());
        const left = Math.min(...shown.map((r) => r.left)), top = Math.min(...shown.map((r) => r.top));
        const right = Math.max(...shown.map((r) => r.right)), bottom = Math.max(...shown.map((r) => r.bottom));
        const m = 16, x = Math.max(0, left + scrollX - m), y = Math.max(0, top + scrollY - m);
        return { x, y, width: Math.min(right - left + 2 * m, ${WIDTH} - x), height: Math.min(bottom - top + 2 * m, 1400) }; })()`);
    }
    const { data } = await send('Page.captureScreenshot', { format: 'webp', quality: 82, captureBeyondViewport: true, clip: { ...clip, scale: 1 } });
    writeFileSync(file, Buffer.from(data, 'base64'));
    console.log(file);
  };
  return { run, waitFor, click, type, shoot };
}

async function tutorial(base, lang) {
  const lesson = LESSONS[lang];
  const folder = join(out, lang);
  mkdirSync(folder, { recursive: true });
  const { send, close } = await launch(WIDTH, HEIGHT);
  try {
    await send('Page.enable');
    await send('Runtime.enable');
    await send('Emulation.setDeviceMetricsOverride', { width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false });
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }, { name: 'prefers-reduced-motion', value: 'reduce' }] });
    await send('Page.navigate', { url: `${base}/studio/?lang=${lang}` });
    const page = driver(send);
    const statement = "document.querySelector('section.statement')";
    await page.waitFor(`${statement} && document.querySelectorAll('.answer-text').length === 4`, 'the new paper');

    // 1. A fresh Studio: one empty question, nothing to install.
    await page.shoot(join(folder, '01-open.webp'));

    // 2. The statement, the answers, the correct one ticked, every option explained.
    await page.click(`${statement}.querySelector('.toolbar button')`, 'the add-text button');
    await page.type(`${statement}.querySelector('textarea')`, lesson.statement, 'the statement field');
    for (const [i, [answer, why]] of lesson.answers.entries()) {
      await page.type(`document.querySelectorAll('.answer-text')[${i}]`, answer, `answer ${i + 1}`);
      await page.type(`document.querySelectorAll('.explanation-text')[${i}]`, why, `explanation ${i + 1}`);
    }
    await page.click("document.querySelector('section.answers input[type=radio]')", 'the correct answer box');
    await page.waitFor("!document.querySelector('li.issue.error')", 'a question without errors');
    await page.shoot(join(folder, '02-write.webp'), `[${statement}, document.querySelector('section.answers')]`);

    // 3. The validator's advice: no rule to remember, nowhere to revise.
    await page.waitFor("document.querySelectorAll('li.issue.warning').length >= 2", 'the validator advice');
    await page.shoot(join(folder, '03-advice.webp'), "document.querySelector('section.advice')");

    // 4. What the student will see, live.
    await page.shoot(join(folder, '04-preview.webp'), "document.querySelector('section.preview')");

    // 5. Publishing: the paper becomes a package to hand out.
    await page.click("document.querySelectorAll('nav.spaces button')[2]", 'the publish space');
    await page.shoot(join(folder, '05-publish.webp'));
  } finally {
    await close();
  }
}

if (!existsSync(join(site, 'studio', 'index.html'))) throw new Error(`No Studio build in ${site}`);
const server = await serve(site);
try {
  const base = `http://127.0.0.1:${server.address().port}`;
  for (const lang of Object.keys(LESSONS)) await tutorial(base, lang);
} finally {
  server.close();
}
