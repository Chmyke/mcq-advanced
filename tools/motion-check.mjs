// Checks in Microsoft Edge (headless) that every control of the student's screens answers the
// pointer: hovering changes how it looks, pressing it pushes it in (a segment of the progress bar
// grows instead). With reduced motion the changes stay but nothing moves: no transition, no
// entrance. The markup is the real one, written by the player's render tests
// (`the_exam_markup_is_handed_to_the_motion_check`, `the_correction_markup_is_handed_to_the_layout_check`);
// the stylesheet is the player's own.
// Run (after `cargo test`): node tools/motion-check.mjs

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch } from './edge.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const fixtures = ['player-exam.html', 'player-review.html'].map((name) => join(root, 'target', 'tmp', name));
for (const file of fixtures) if (!existsSync(file)) throw new Error(`No ${file}: run cargo test first.`);
const markup = fixtures.map((file) => readFileSync(file, 'utf8')).join('\n');
const css = readFileSync(join(root, 'apps', 'mcq-player', 'web', 'style.css'), 'utf8');
const page = `<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><style>${css}</style></head>
<body><div class="app" data-theme="light"><main>${markup}</main></div></body></html>`;

const CONTROLS = 'button:not(:disabled), .file, a.button, .option';
// What the pointer may change: the control itself, and the bar a segment draws.
const LOOK = `(i) => { const e = document.querySelectorAll(${JSON.stringify(CONTROLS)})[i];
  const s = getComputedStyle(e), b = getComputedStyle(e, '::before');
  return { look: [s.backgroundColor, s.borderColor, s.color, s.filter, b.transform].join(' | '), pushed: s.transform, segment: e.classList.contains('segment'),
    name: (e.className || e.tagName) + ' "' + e.textContent.trim().slice(0, 20) + '"' }; }`;

const { send, close } = await launch(1280, 900);
const failures = [];
try {
  await send('Page.enable');
  await send('Runtime.enable');
  await send('DOM.enable');
  await send('CSS.enable');
  const run = async (expression) => {
    const { result, exceptionDetails } = await send('Runtime.evaluate', { expression, returnByValue: true });
    if (exceptionDetails) throw new Error(`${exceptionDetails.text}: ${expression}`);
    return result.value;
  };
  const motion = (value) => send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value }] });
  const { frameTree } = await send('Page.getFrameTree');
  await send('Page.setDocumentContent', { frameId: frameTree.frame.id, html: page });

  // Motion on: controls ease between states and a question fades in.
  await motion('no-preference');
  const timing = await run(`(() => ({ button: getComputedStyle(document.querySelector('button')).transitionDuration,
    question: getComputedStyle(document.querySelector('.question')).animationName }))()`);
  if (/^0s(, 0s)*$/.test(timing.button)) failures.push(`motion on: buttons have no transition (${timing.button})`);
  if (timing.question !== 'appear') failures.push(`motion on: the question has no entrance (${timing.question})`);

  // Reduced motion: states change at once, so they are read without waiting.
  await motion('reduce');
  const still = await run(`(() => ({ button: getComputedStyle(document.querySelector('button')).transitionDuration,
    question: getComputedStyle(document.querySelector('.question')).animationName }))()`);
  if (!/^0s(, 0s)*$/.test(still.button)) failures.push(`reduced motion: buttons still move (${still.button})`);
  if (still.question !== 'none') failures.push(`reduced motion: the question still fades in (${still.question})`);

  const { root: doc } = await send('DOM.getDocument', { depth: -1 });
  const { nodeIds } = await send('DOM.querySelectorAll', { nodeId: doc.nodeId, selector: CONTROLS });
  if (nodeIds.length < 5) throw new Error(`Only ${nodeIds.length} controls in the fixtures.`);
  for (const [i, nodeId] of nodeIds.entries()) {
    const look = async (state) => {
      await send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: state });
      return run(`(${LOOK})(${i})`);
    };
    const rest = await look([]);
    const hover = await look(['hover']);
    const pressed = await look(['hover', 'active']);
    await look([]);
    if (hover.look === rest.look) failures.push(`${rest.name}: nothing changes on hover`);
    if (!rest.segment && pressed.pushed === rest.pushed) failures.push(`${rest.name}: nothing moves when pressed`);
  }
  console.log(`${nodeIds.length} controls checked.`);
} finally {
  await close();
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('Motion: every control answers the pointer; reduced motion keeps everything still.');
