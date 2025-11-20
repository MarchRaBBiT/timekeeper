import { chromium } from "playwright";

const base = process.env.FRONTEND_BASE_URL || "http://localhost:8080";
const adminUsername = process.env.E2E_ADMIN_USER || "admin";
const adminPassword = process.env.E2E_ADMIN_PASS || "admin123";

const uniqueTag = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;

const formatDate = (offsetDays) => {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  return date.toISOString().split("T")[0];
};

const weeklyStartDate = formatDate(7);
const holidayDate = formatDate(10);
const holidayName = `E2E祝日-${uniqueTag}`;

const log = (message) => console.log(`[admin-dashboard-e2e] ${message}`);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
page.setDefaultTimeout(20000);

try {
  log("ログインを開始します");
  await page.goto(`${base}/login`, { waitUntil: "domcontentloaded" });
  await page.fill("#username", adminUsername);
  await page.fill("#password", adminPassword);
  await page.click('button[type="submit"]');
  await page.waitForURL("**/dashboard", { timeout: 20000 });
  log("ログインに成功しました");

  log("管理者ダッシュボードへ遷移します");
  await page.goto(`${base}/admin`, { waitUntil: "domcontentloaded" });
  await page.waitForSelector("text=管理者ツール");

  log("週次休日を登録します");
  const weeklyForm = page.locator("form").filter({ hasText: "週次休日を登録" }).first();
  await weeklyForm.locator("select").selectOption("1");
  const weeklyDates = weeklyForm.locator('input[type="date"]');
  await weeklyDates.first().fill(weeklyStartDate);
  if ((await weeklyDates.count()) > 1) {
    await weeklyDates.nth(1).fill("");
  }
  await weeklyForm.locator('button:has-text("週次休日を登録")').click();
  await page.waitForSelector("text=登録しました", { timeout: 20000 });
  log("週次休日の登録メッセージを確認しました");

  log("祝日を登録します");
  const holidayForm = page.locator("form").filter({ hasText: "祝日を登録" }).first();
  await holidayForm.locator('input[type="date"]').fill(holidayDate);
  await holidayForm.locator('label:has-text("名称") + input').fill(holidayName);
  await holidayForm.locator('label:has-text("備考（任意）") + input').fill("Playwright generated holiday");
  await holidayForm.locator('button:has-text("祝日を登録")').click();
  await page.waitForSelector(`text=${holidayName}`, { timeout: 20000 });
  log("祝日の登録完了を確認しました");

  log("MFA リセットの入力検証を確認します");
  const mfaCard = page.locator("div").filter({ hasText: "MFA リセット" }).first();
  await mfaCard.locator('button:has-text("MFA をリセット")').click();
  await page.waitForSelector("text=ユーザーIDを入力してください。", { timeout: 10000 });
  log("MFA リセットのバリデーションメッセージを確認しました");

  await browser.close();
  log("Admin Dashboard シナリオが完了しました");
  process.exit(0);
} catch (error) {
  console.error("[admin-dashboard-e2e] テストが失敗しました:", error);
  try {
    console.error("Final URL:", page.url());
  } catch {
    // ignore
  }
  await browser.close();
  process.exit(1);
}
