import { chromium } from "playwright";

const base = process.env.FRONTEND_BASE_URL || "http://localhost:8080";
const adminUsername = process.env.E2E_ADMIN_USER || "admin";
const adminPassword = process.env.E2E_ADMIN_PASS || "admin123";

const timestamp = Date.now().toString(36);
const rand = Math.random().toString(36).slice(2, 6);
const newUsername = `admin-e2e-${timestamp}-${rand}`;
const newFullName = `自動テスト ${rand}`;
const newPassword = `Pass!${timestamp}${rand}`;

const log = (message) => console.log(`[admin-users-e2e] ${message}`);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
page.setDefaultTimeout(20000);

try {
  log("ログイン開始");
  await page.goto(`${base}/login`, { waitUntil: "domcontentloaded" });
  await page.fill("#username", adminUsername);
  await page.fill("#password", adminPassword);
  await page.click('button[type="submit"]');
  await page.waitForURL("**/dashboard", { timeout: 20000 });
  log("ログイン成功");

  log("Admin Users ページへ遷移");
  await page.goto(`${base}/admin/users`, { waitUntil: "domcontentloaded" });
  await page.waitForSelector('text=ユーザー招待 (管理者専用)', { timeout: 15000 });

  log("必須チェックを検証");
  await page.click('button:has-text("ユーザーを作成")');
  await page.waitForSelector('text=すべての必須項目を入力してください。');

  log(`ユーザー ${newUsername} を作成`);
  await page.fill('input[placeholder="username"]', newUsername);
  await page.fill('input[placeholder="山田太郎"]', newFullName);
  await page.fill('input[type="password"]', newPassword);
  await page.selectOption("select", "employee");
  await page.click('button:has-text("ユーザーを作成")');
  await page.waitForSelector(`text=ユーザー '${newUsername}' を作成しました。`, {
    timeout: 20000,
  });

  const rowLocator = page.locator("tbody tr").filter({ hasText: newUsername }).first();
  await rowLocator.waitFor({ state: "visible", timeout: 20000 });
  log("一覧に新規ユーザーが表示されたことを確認");

  log("ユーザー詳細ドロワーを開く");
  await rowLocator.click();
  await page.waitForSelector(`text=@${newUsername}`);

  const resetButton = page.locator('button:has-text("MFA をリセット")');
  await resetButton.waitFor({ state: "visible", timeout: 10000 });
  await resetButton.click();
  await page.waitForSelector('text=MFA をリセットしました。', { timeout: 20000 });
  log("MFA リセット完了を確認");

  await page.locator('button:has-text("✕")').click();
  await page.waitForSelector('text=MFA をリセットしました。', { state: "detached", timeout: 10000 });
  await page.waitForSelector(`text=@${newUsername}`, { state: "detached", timeout: 10000 });
  log("ドロワーを閉じて完了");

  await browser.close();
  log("Admin Users エンドツーエンドシナリオ完了");
  process.exit(0);
} catch (error) {
  console.error("[admin-users-e2e] テスト失敗:", error);
  try {
    console.error("Final URL:", page.url());
  } catch (_) {
    // ignore
  }
  await browser.close();
  process.exit(1);
}
