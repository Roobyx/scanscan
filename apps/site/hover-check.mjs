import { chromium } from "playwright";

const browser = await chromium.launch({ channel: "chrome" });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto("http://127.0.0.1:4400/", { waitUntil: "networkidle" });
const button = page.locator(".copy-btn").first();
await button.scrollIntoViewIfNeeded();
await page.waitForTimeout(400);
await button.screenshot({ path: "../../.shots/copy-rest.png" });
const read = (el) => {
  const cs = getComputedStyle(el, "::before");
  return {
    transform: cs.transform,
    background: cs.backgroundColor,
    zIndex: cs.zIndex,
    color: getComputedStyle(el).color,
    rect: el.getBoundingClientRect().toJSON(),
  };
};
const before = await button.evaluate(read);
await button.hover();
await page.waitForTimeout(600);
await button.screenshot({ path: "../../.shots/copy-hover.png" });
const after = await button.evaluate(read);
console.log(JSON.stringify({ before, after }, null, 2));
await browser.close();


