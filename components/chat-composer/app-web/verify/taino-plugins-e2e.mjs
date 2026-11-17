// E5a 端到端：原生插件生态（slash-commands / file-completion / word-count）。
// 判据（D022）：
//   @ 菜单合并两插件来源（attachment-files + project-files）；
//   / 菜单来自 slash-commands 插件；/clear 提交清草稿不出 outbox；
//   /help 提交弹命令清单；word-count 状态栏随输入更新。
import { writeFile, mkdir, cp } from "node:fs/promises";
import { chromium } from "playwright-core";
import { startStaticServer, INDEX_HTML } from "./static-server.mjs";

const distRoot = new URL("./dist/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
await mkdir(distRoot, { recursive: true });
await cp(new URL("../vendor/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"), distRoot, { recursive: true });
await writeFile(distRoot + "index.html", INDEX_HTML);

const launchChannel = () => {
  const channel = process.env.CE_BROWSER_CHANNEL;
  return channel && channel !== "chromium" ? channel : undefined;
};
const server = await startStaticServer(distRoot);
const browser = await chromium.launch({ channel: launchChannel(), headless: true });
const page = await (await browser.newContext()).newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(String(e).slice(0, 200)));

const results = [];
const check = (name, cond, detail) => { results.push({ name, pass: Boolean(cond), detail }); console.log(`${cond ? "✔" : "✖"} ${name}${cond ? "" : ` — ${detail}`}`); };
const composerView = () => page.evaluate(() => window.__composerView());
const editorText = () => page.evaluate(() => document.querySelector('[contenteditable="true"]')?.textContent ?? "");
const focusEditor = () => page.evaluate(() => document.querySelector('[contenteditable="true"]').focus());
const freshPage = async () => {
  const scope = `plug-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  await page.goto(`${server.url}?editor=taino&scope=${scope}`);
  await page.waitForFunction(() => window.__composerView !== undefined, { timeout: 30000 });
  await page.waitForSelector('[contenteditable="true"]', { timeout: 15000 });
  await focusEditor();
};

await freshPage();

// 1. @ 菜单合并两插件来源
await page.keyboard.type("@");
await page.waitForFunction(() => document.querySelector('[data-testid="candidate-menu"]') !== null, undefined, { timeout: 10000 });
const atItems = await page.evaluate(() => Array.from(document.querySelectorAll('[data-testid="candidate-menu"] [role="option"]')).map((li) => li.textContent));
check("1a @ 菜单合并 file-completion 插件候选", atItems?.some((t) => t?.includes("project-docs.md")) === true, JSON.stringify(atItems));
check("1b attachment-tools 来源仍在（多插件合并）", atItems?.some((t) => t?.includes("截图.png")) === true, JSON.stringify(atItems));
await page.keyboard.press("Escape");

// 2. / 菜单来自 slash-commands 插件
await freshPage();
await page.keyboard.type("/");
await page.waitForFunction(() => document.querySelector('[data-testid="candidate-menu"]') !== null, undefined, { timeout: 10000 });
const slashItems = await page.evaluate(() => Array.from(document.querySelectorAll('[data-testid="candidate-menu"] [role="option"]')).map((li) => li.textContent));
check("2 slash 菜单来自插件（/help /clear）", slashItems?.length === 2 && slashItems?.[0]?.includes("/help"), JSON.stringify(slashItems));

// 3. /clear 提交 → 草稿清空，不出 outbox
// 流程：type "/" → 菜单开 → Escape 关菜单 → 补 "clear" → Enter → 命令面（拦截 /clear）。
// 直接 type "/clear" 时菜单开着，Enter 会走「接受候选」写回 "/clear "（KD has_menu 语义）。
await freshPage();
await page.keyboard.type("/clear");
await page.waitForFunction(() => document.querySelector('[data-testid="candidate-menu"]') !== null, undefined, { timeout: 10000 });
// Escape 关菜单后直接 Enter——中途不再打字（detect_trigger 对同一 token 会重开菜单，
// 打字后 Enter 又走 accept——E3d 实录）。
await page.keyboard.press("Escape");
await page.keyboard.press("Enter");
await page.waitForTimeout(500);
let view = await composerView();
const textAfterClear = await editorText();
check("3a /clear 清草稿（命令面）", textAfterClear === "" && view.state.draft.snapshot.text === "", JSON.stringify({ dom: textAfterClear, draft: view.state.draft.snapshot.text }));
check("3b /clear 不进 outbox（命令面非消息）", view.state.submissions.length === 0, JSON.stringify(view.state.submissions));

// 4. /help 提交 → 命令清单呈现（同 3 的 Escape 流程）
await freshPage();
await page.keyboard.type("/help");
await page.waitForFunction(() => document.querySelector('[data-testid="candidate-menu"]') !== null, undefined, { timeout: 10000 });
await page.keyboard.press("Escape");
await page.keyboard.press("Enter");
await page.waitForTimeout(500);
view = await composerView();
const helpShown = await page.evaluate(() => {
  const toast = document.querySelector('[data-testid="slash-help"]');
  return toast ? toast.textContent : null;
});
check("4a /help 命令清单呈现", typeof helpShown === "string" && helpShown.includes("slash."), JSON.stringify(helpShown));
check("4b /help 不进 outbox", view.state.submissions.length === 0, JSON.stringify(view.state.submissions));

// 5. word-count 状态栏
await freshPage();
await page.keyboard.type("hello world 中文");
await page.waitForTimeout(400);
const wcText = await page.evaluate(() => document.querySelector('[data-testid="word-count"]')?.textContent ?? null);
const wcDump = await page.evaluate(() => ({
  text: document.querySelector('[data-testid="word-count"]')?.textContent,
  display: document.querySelector('[data-testid="word-count"]')?.style.display,
  outer: document.querySelector('[data-testid="word-count"]')?.outerHTML?.slice(0, 150),
}));
console.log("5 wcDump:", JSON.stringify(wcDump));
check("5 word-count 状态栏更新（14 字符 / 3 词）", typeof wcText === "string" && wcText.includes("14 字符") && wcText.includes("3 词"), JSON.stringify(wcText));

// 6. 零未捕获错误
check("6a 零未捕获页面错误", errors.length === 0, JSON.stringify(errors));

const failed = results.filter((item) => !item.pass);
console.log(`\n${results.length - failed.length}/${results.length} passed`);
await browser.close();
server.close();
process.exit(failed.length ? 1 : 0);
