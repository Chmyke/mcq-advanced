// Drives the web builds in Microsoft Edge (headless) through what a teacher and a student do,
// and checks what only a real browser shows: answer boxes that keep their state from one question
// to another, an import added to a written paper, the clock against the wall clock, the keyboard
// and the focus, the unfinished copy announced after a reload.
// Run: node tools/web-check.mjs <folder with studio/ and player/, player/check.mcqpack inside>
// tools/check.ps1 builds that folder (debug builds, a practice package of the French demo).

import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { launch, serve, sleep, steps } from './edge.mjs';

const site = process.argv[2] && resolve(process.argv[2]);
if (!site) {
  console.error('usage: node tools/web-check.mjs <site folder>');
  process.exit(2);
}
for (const file of ['studio/index.html', 'player/index.html', 'player/check.mcqpack']) {
  if (!existsSync(join(site, file))) throw new Error(`No ${file} in ${site}`);
}

const failures = [];
const expect = (ok, what) => {
  if (!ok) failures.push(what);
};

// Every answer row of the editor shows ticked exactly the answers the model holds right.
const BOXES_MATCH = `[...document.querySelectorAll('.answers .answer')].every((a) =>
  a.querySelector('input[type=radio], input[type=checkbox]').checked === a.classList.contains('correct'))`;

// The sample class, from the welcome screen, gets ready: its tour shows.
async function sampleClass(page, base, send) {
  await send('Page.navigate', { url: `${base}/studio/?lang=en` });
  const button = "[...document.querySelectorAll('.welcome-studio button')].find((b) => b.textContent.includes('sample class'))";
  await page.click(button, 'the sample class button');
  await page.waitFor("document.querySelector('details.tour')", 'the sample class and its tour');
  // A fresh draft for what follows.
  await page.run("indexedDB.deleteDatabase('mcq-studio'); 0");
}

async function studio(page, base, send) {
  await send('Page.navigate', { url: `${base}/studio/?lang=en` });
  await page.waitFor("document.querySelectorAll('.answers .answer-text').length === 4", 'the new paper');
  const boxes = "document.querySelectorAll('.answers input[type=radio], .answers input[type=checkbox]')";
  // Question 1, single answer: A. Question 2, several answers: A and B.
  await page.click(`${boxes}[0]`, 'answer A of question 1');
  await page.click("document.querySelector('.theme-head button')", 'the add-question button');
  await page.waitFor("document.querySelectorAll('.q-item').length === 2", 'question 2');
  await page.click("document.querySelectorAll('.kinds button')[1]", 'the several-answers kind');
  await page.click(`${boxes}[0]`, 'answer A of question 2');
  await page.click(`${boxes}[1]`, 'answer B of question 2');
  for (const [n, item] of [[1, 0], [2, 1], [1, 0], [2, 1]]) {
    await page.click(`document.querySelectorAll('.q-item')[${item}]`, `question ${n} in the list`);
    expect(await page.run(BOXES_MATCH), `studio: back on question ${n}, a box does not show its answer's state`);
  }

  // A bank pasted over a written paper: added after it on request, nothing lost.
  await page.type("document.querySelector('.statement textarea')", 'Which are prime?', 'the statement of question 2');
  await page.run(`(() => { const d = new DataTransfer();
    d.setData('text/plain', 'What is 2 + 2?\\nA. 3\\nB. 4\\nANSWER: B\\n');
    document.body.dispatchEvent(new ClipboardEvent('paste', { clipboardData: d, bubbles: true })); })()`);
  await page.waitFor("document.querySelector('dialog.choice-dialog[open]')", 'the add or replace question');
  expect(await page.run("document.querySelectorAll('dialog.choice-dialog button').length") === 3,
    'studio: the import question does not offer add, replace and cancel');
  expect(await page.run("document.querySelector('dialog.choice-dialog p').textContent.includes('the pasted text brings 1')"),
    'studio: the import question does not say the questions come from pasted text');
  await page.click("document.querySelector('dialog.choice-dialog button')", 'add to my paper');
  await page.waitFor("document.querySelectorAll('.q-item').length === 3", 'the pasted question added');
  expect(await page.run("document.querySelector('.status-line').textContent.includes('1 question added')"),
    'studio: the notice does not say the question was added');
  // Moodle XML pasted is read as the import reads its file; its question goes to a theme named
  // "Imported questions", not after the paper.
  await page.run(`(() => { const d = new DataTransfer();
    d.setData('text/plain', ${JSON.stringify(MOODLE_XML)});
    document.body.dispatchEvent(new ClipboardEvent('paste', { clipboardData: d, bubbles: true })); })()`);
  await page.click("document.querySelector('dialog.choice-dialog[open] button')", 'add the Moodle question');
  await page.waitFor("document.querySelectorAll('.q-item').length === 4", 'the pasted Moodle XML question added');
  expect(await page.run("[...document.querySelectorAll('.theme-head h2')].some((h) => h.textContent === 'Imported questions')"),
    'studio: questions pasted without a category are not in "Imported questions"');

  // The help menu is always there, the AI prompt in it.
  await page.click("document.querySelector('[aria-label^=\"Help\"]')", 'the help menu');
  expect(await page.run("[...document.querySelectorAll('.menu-list button')].some((b) => b.textContent.includes('AI prompt'))"),
    'studio: the help menu does not offer the AI prompt');
  expect(await page.run("document.querySelectorAll('.menu-list a').length") === 2,
    'studio: the help menu does not link the tutorial and the guide');

  // The sample class over a draft with work: asked first, the draft set aside for the visit and
  // back in the editor on leaving it.
  await sleep(2500); // the autosave keeps the draft
  await send('Page.navigate', { url: `${base}/studio/?lang=en&demo=teacher` });
  await page.waitFor("document.querySelector('dialog.choice-dialog[open]')", 'the sample class question');
  expect(await page.run("document.querySelector('dialog.choice-dialog p').textContent.includes('set aside')"),
    'studio: opening the sample class does not say the draft is set aside');
  await page.click("document.querySelector('dialog.choice-dialog button')", 'open the sample class');
  await page.waitFor("document.querySelector('details.tour')", 'the sample class tour');
  await page.waitFor("!document.querySelector('.statement textarea') || document.querySelector('.statement textarea').value !== 'Which are prime?'", 'the demo paper');
  expect(await page.run("document.querySelectorAll('.q-item').length") !== 4,
    'studio: the editor keeps the draft while the sample class says its paper is in');
  await page.click("document.querySelector('details.tour .leave-tour')", 'leave the sample class');
  await page.waitFor("!document.querySelector('details.tour') && document.querySelectorAll('.q-item').length === 4", 'the draft back');
  expect(await page.run("location.search.includes('demo')") === false,
    'studio: leaving the sample class keeps its link, a reload would open it again');
}

// A one-question Moodle XML bank, as copied from its file.
const MOODLE_XML = `<?xml version="1.0" encoding="UTF-8"?>
<quiz>
  <question type="multichoice">
    <name><text>Capital</text></name>
    <questiontext format="html"><text><![CDATA[<p>Which city is the capital of Italy?</p>]]></text></questiontext>
    <single>true</single>
    <answer fraction="100"><text>Rome</text></answer>
    <answer fraction="0"><text>Milan</text></answer>
  </question>
</quiz>
`;

// Remaining time on the clock, in seconds ("9 min 58 s" or "42 s").
const REMAINING = `(() => { const t = document.querySelector('.clock-value').textContent;
  const m = /(?:(\\d+) min )?(\\d+) s/.exec(t); return m ? Number(m[1] || 0) * 60 + Number(m[2]) : -1; })()`;
const FOCUSED = "document.activeElement && document.activeElement.id";
// The number of the question on screen ("Question 2 sur 6" gives "2").
const NUMBER = "document.getElementById('q-title').textContent.split(' ')[1]";

async function player(page, base, send, dialogs) {
  const key = async (key, code, keyCode) => {
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode: keyCode });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: keyCode });
    await sleep(250);
  };
  await send('Page.navigate', { url: `${base}/player/?lang=en&paper=check.mcqpack` });
  await page.waitFor("document.getElementById('name')", 'the name field');
  await sleep(300);
  expect(await page.run(FOCUSED) === 'name', 'player: the identification screen does not focus the name');
  await page.type("document.getElementById('name')", 'Web Check', 'the name field');
  await page.click("document.querySelector('form.identify button[type=submit]')", 'open the paper');
  // The paper is French: so is the interface from here on.
  await page.click("document.querySelector('main button.primary')", 'start');
  await page.waitFor("document.getElementById('q-title')", 'the first question');
  await sleep(300);
  expect(await page.run(FOCUSED) === 'q-title', 'player: the exam does not start with the focus on the question');
  expect(await page.run("!!document.querySelector('.skip-link')"), 'player: no way past the header to the question');

  // Enter on an answer's box moves on, as the keyboard help says.
  await page.run("document.querySelector('.options input').focus()");
  await key('Enter', 'Enter', 13);
  expect(await page.run(NUMBER) === '2',
    'player: Enter on an answer box does not move to the next question');

  // Question 5 asks for two answers: one ticked leaves it incomplete, and the hand-in question
  // says so in the singular, with a way back to it.
  await page.click("document.querySelectorAll('.exam-segments button')[4]", 'question 5 in the overview');
  await page.click("document.querySelector('.options input[type=checkbox]')", 'one box of question 5');
  await page.click("document.getElementById('finish')", 'finish');
  const confirm = await page.run("document.querySelector('.confirm').textContent");
  expect(confirm.includes('1 incomplète,'), `player: the hand-in question miscounts: ${confirm}`);
  await page.click("document.querySelector('.incomplete-links button')", 'the incomplete question');
  expect(await page.run(NUMBER) === '5',
    'player: the link of the hand-in question does not lead to the incomplete question');

  // A page asleep for two minutes (a locked phone) counts them as soon as it wakes.
  const before = await page.run(REMAINING);
  await page.run('(() => { const real = Date.now.bind(Date); Date.now = () => real() + 120000; })()');
  await sleep(1500);
  const after = await page.run(REMAINING);
  expect(before > 0 && before - after >= 120, `player: two minutes asleep counted ${before - after} s`);

  // Reloaded: the leave question came, then the welcome screen names the unfinished copy.
  const asked = dialogs();
  await send('Page.navigate', { url: `${base}/player/?lang=en` });
  await page.waitFor("document.querySelector('.welcome')", 'the welcome screen');
  expect(dialogs() > asked, 'player: leaving during the exam asks nothing');
  await page.waitFor("document.querySelector('.welcome .notice')", 'the unfinished copy notice');
  expect(await page.run("document.querySelector('.welcome .notice').textContent.includes('unfinished')"),
    'player: the welcome screen does not mention the unfinished copy');
}

const server = await serve(site);
const base = `http://127.0.0.1:${server.address().port}`;
const { send, close, dialogs } = await launch(1280, 900);
try {
  await send('Page.enable');
  await send('Runtime.enable');
  const page = steps(send);
  await sampleClass(page, base, send);
  await studio(page, base, send);
  await player(page, base, send, dialogs);
} catch (error) {
  failures.push(error.message);
} finally {
  await close();
  server.close();
}
for (const failure of failures) console.log(`FAIL ${failure}`);
if (failures.length) process.exit(1);
console.log('web check: studio and player OK');
