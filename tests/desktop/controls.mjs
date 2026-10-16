import assert from "node:assert/strict";
import fs from "node:fs/promises";
import { connect } from "./webview.mjs";
const c = await connect();
const log = [];
await fs.mkdir("temp/m05-evidence", { recursive: true });
const ev = c.evaluate;
const mode = (await ev("location.hostname")) === "tauri.localhost" ? "production" : "development";
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
async function wait(expression) {
  const end = Date.now() + 12000;
  while (Date.now() < end) {
    if (await ev(expression)) return;
    await delay(80);
  }
  throw new Error(`UI condition timed out: ${expression}`);
}
async function click(text) {
  await ev(
    `(()=>{const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===${JSON.stringify(text)}&&!b.disabled);if(!b)throw Error('Missing enabled button');b.click()})()`,
  );
}
async function key(key, code, keyCode, modifiers = 0) {
  await c.send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key,
    code,
    windowsVirtualKeyCode: keyCode,
    modifiers,
    ...(key === "Enter" ? { text: "\r" } : {}),
  });
  await c.send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key,
    code,
    windowsVirtualKeyCode: keyCode,
    modifiers,
  });
}
async function fill(value) {
  await ev(
    `document.querySelector('#alias-name').focus();document.querySelector('#alias-name').select()`,
  );
  await key("Backspace", "Backspace", 8);
  if (value) await c.send("Input.insertText", { text: value });
}
async function shot(name) {
  const { data } = await c.send("Page.captureScreenshot", { format: "png" });
  await fs.writeFile(`temp/m05-evidence/${mode}-${name}.png`, Buffer.from(data, "base64"));
}
try {
  if (await ev(`!!document.querySelector('dialog[open]')`)) await key("Escape", "Escape", 27);
  await ev(
    `(()=>{const b=[...document.querySelectorAll('.file-item')].filter(b=>b.textContent.includes('sentinel.txt')).sort((a,b)=>Number(b.textContent.match(/([0-9]+) aliases/)?.[1]||0)-Number(a.textContent.match(/([0-9]+) aliases/)?.[1]||0))[0];if(!b)throw Error('Expected isolated fixture');b.click()})()`,
  );
  await wait(`document.querySelector('.saved-path')?.textContent.includes('temp\\\\m05-fixtures')`);
  if (!(await ev(`!!document.querySelector('#alias-name')`))) await click("+ Add");
  await wait(`!!document.querySelector('#alias-name')`);
  for (const name of [
    "",
    " a",
    "../bad",
    "CON.pdf",
    "a?",
    "a.",
    "a".repeat(256),
    "é".repeat(128),
  ]) {
    await fill(name);
    await click("Save");
    await wait(`document.querySelector('#alias-name')?.getAttribute('aria-invalid')==='true'`);
    assert.equal(await ev(`document.activeElement.id`), "alias-name");
  }
  await fill("CONTEXT.pdf");
  await click("Save");
  await wait(
    `document.querySelector('[role=alert]')?.textContent.includes('already has an alias')`,
  );
  assert.equal(await ev(`document.querySelector('#alias-name').value`), "CONTEXT.pdf");
  assert.equal(await ev(`document.activeElement.id`), "alias-name");
  await key("Escape", "Escape", 27);
  await wait(`!document.querySelector('#alias-name')`);
  log.push("PASS: real UI validation categories/ASCII duplicate/draft/focus/Escape");
  for (const name of ["É.pdf", "é.pdf", "e\u0301.pdf", "é".repeat(127) + "a"]) {
    if (
      await ev(
        `[...document.querySelectorAll('.alias-row span')].some(x=>x.textContent===${JSON.stringify(name)})`,
      )
    )
      continue;
    if (!(await ev(`!!document.querySelector('#alias-name')`))) await click("+ Add");
    await wait(`!!document.querySelector('#alias-name')`);
    await fill(name);
    await click("Save");
    await wait(`!document.querySelector('#alias-name')`);
  }
  log.push(
    "PASS: Unicode case/normalization distinctions and 255 UTF-8-byte name saved through real UI/IPC",
  );
  await ev(`document.querySelector('[aria-label="Actions for Third.txt"]').focus()`);
  if (
    !(await ev(`document.querySelector('[aria-label="Actions for Third.txt"]').parentElement.open`))
  )
    await key(" ", "Space", 32);
  await wait(`document.querySelector('[aria-label="Actions for Third.txt"]').parentElement.open`);
  await ev(
    `document.querySelector('[aria-label="Actions for Third.txt"]').parentElement.querySelectorAll('button')[1].focus()`,
  );
  await key("Enter", "Enter", 13);
  await wait(`!!document.querySelector('dialog[open]')`);
  assert.equal(await ev(`document.activeElement.textContent.trim()`), "Cancel");
  for (let i = 0; i < 5; i++) {
    await key("Tab", "Tab", 9);
    assert.equal(await ev(`document.activeElement.closest('dialog')!==null`), true);
  }
  await key("Escape", "Escape", 27);
  await wait(`!document.querySelector('dialog')`);
  await wait(`document.activeElement.textContent.trim()==='Delete'`);
  log.push("PASS: keyboard menu/delete dialog/Cancel default/Tab trap/Escape/focus return");
  await ev(`document.querySelector('[aria-label="Theme"]').focus()`);
  await key("Home", "Home", 36);
  await key("ArrowDown", "ArrowDown", 40);
  await wait(`document.querySelector('[aria-label=Theme]').value==='light'`);
  assert.equal(await ev(`document.documentElement.dataset.theme`), "light");
  await shot("light");
  await key("End", "End", 35);
  await wait(`document.documentElement.dataset.theme==='dark'`);
  assert.equal(await ev(`localStorage.getItem('filenym-theme')`), "dark");
  await shot("dark");
  await key("Home", "Home", 36);
  await wait(`document.querySelector('[aria-label=Theme]').value==='system'`);
  for (const mode of ["light", "dark"]) {
    await c.send("Emulation.setEmulatedMedia", {
      features: [{ name: "prefers-color-scheme", value: mode }],
    });
    await wait(`document.documentElement.dataset.theme===${JSON.stringify(mode)}`);
  }
  log.push(
    "PASS: real select keyboard controls/persisted enum/System matchMedia changes (OS signal emulated)",
  );
  for (const [width, height, dpr] of [
    [900, 640, 1],
    [640, 440, 1.25],
    [640, 440, 1.5],
    [640, 440, 2],
  ]) {
    await c.send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: dpr,
      mobile: false,
    });
    await delay(150);
    assert.equal(await ev(`document.documentElement.scrollWidth<=innerWidth`), true);
    const button = await ev(
      `document.querySelector('.alias-heading button').getBoundingClientRect().width`,
    );
    assert(button < 90);
    if (width === 640)
      assert.equal(
        await ev(`getComputedStyle(document.querySelector('.file-panel')).display`),
        "none",
      );
    await shot(`layout-${width}-${dpr}`);
  }
  log.push(
    "PASS: actual WebView responsive layouts at 900/640 with emulated 100/125/150/200% rendering; not OS DPI changes",
  );
  await c.send("Emulation.clearDeviceMetricsOverride");
  await c.send("Emulation.setEmulatedMedia", {
    features: [
      { name: "prefers-reduced-motion", value: "reduce" },
      { name: "forced-colors", value: "active" },
    ],
  });
  assert.equal(
    await ev(
      `matchMedia('(prefers-reduced-motion: reduce)').matches&&matchMedia('(forced-colors: active)').matches`,
    ),
    true,
  );
  await shot("forced-colors-reduced-motion");
  await c.send("Emulation.setEmulatedMedia", { features: [] });
  await ev(`document.querySelector('[aria-label=Theme]').focus()`);
  await key("End", "End", 35);
  await wait(`document.documentElement.dataset.theme==='dark'`);
  await fs.writeFile(
    `temp/m05-evidence/${mode}-controls.json`,
    JSON.stringify({ log, text: await ev("document.body.innerText") }, null, 2),
  );
  console.log(log.join("\n"));
} finally {
  await c.send("Emulation.clearDeviceMetricsOverride").catch(() => {});
  await c.send("Emulation.setEmulatedMedia", { features: [] }).catch(() => {});
  c.close();
}
