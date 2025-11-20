import { chromium } from "playwright";

const base = process.env.FRONTEND_BASE_URL || "http://localhost:8080";
const adminUsername = process.env.E2E_ADMIN_USER || "admin";
const adminPassword = process.env.E2E_ADMIN_PASS || "admin123";

const formatDate = (offsetDays) => {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  return date.toISOString().split("T")[0];
};

const leaveStart = formatDate(5);
const leaveEnd = formatDate(7);
const overtimeDate = formatDate(3);
const leaveReason = `家族行事-${Date.now().toString(36)}`;
const overtimeReason = `夜間対応-${Math.random().toString(36).slice(2, 6)}`;

const log = (message) => console.log(`[requests-e2e] ${message}`);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
page.setDefaultTimeout(20000);

try {
  log("ログインを開始");
  await page.goto(`${base}/login`, { waitUntil: "domcontentloaded" });
  await page.fill("#username", adminUsername);
  await page.fill("#password", adminPassword);
  await page.click('button[type="submit"]');
  await page.waitForURL("**/dashboard", { timeout: 20000 });
  log("ログイン成功");

  log("Requests ページへ遷移");
  await page.goto(`${base}/requests`, { waitUntil: "domcontentloaded" });
  await page.waitForSelector("text=申請管理");

  log("休暇申請フォームを送信");
  const leaveForm = page.locator("form").filter({ hasText: "休暇申請" }).first();
  await leaveForm.locator("select").selectOption("annual");
  const leaveDates = leaveForm.locator('input[type="date"]');
  await leaveDates.first().fill(leaveStart);
  await leaveDates.nth(1).fill(leaveEnd);
  await leaveForm.locator("textarea").fill(leaveReason);
  await leaveForm.locator('button:has-text("休暇申請を送信")').click();
  await page.waitForSelector("text=休暇申請を送信しました。", { timeout: 20000 });

  log("残業申請フォームを送信");
  const overtimeForm = page.locator("form").filter({ hasText: "残業申請" }).first();
  await overtimeForm.locator('input[type="date"]').fill(overtimeDate);
  await overtimeForm.locator('input[type="number"]').fill("2");
  await overtimeForm.locator("textarea").fill(overtimeReason);
  await overtimeForm.locator('button:has-text("残業申請を送信")').click();
  await page.waitForSelector("text=残業申請を送信しました。", { timeout: 20000 });

  log("フィルタの切り替えを確認");
  const filterSelect = page
    .locator("div")
    .filter({ hasText: "申請の絞り込み" })
    .first()
    .locator("select");
  await filterSelect.selectOption("pending");
  await filterSelect.selectOption("");

  await browser.close();
  log("Requests ページ E2E シナリオ完了");
  process.exit(0);
} catch (error) {
  console.error("[requests-e2e] テスト失敗:", error);
  try {
    console.error("Final URL:", page.url());
  } catch {
    // ignore
  }
  await browser.close();
  process.exit(1);
}
