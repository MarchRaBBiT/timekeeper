import { chromium } from "playwright";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const base = process.env.FRONTEND_BASE_URL || "http://localhost:8080";
const adminUsername = process.env.E2E_ADMIN_USER || "admin";
const adminPassword = process.env.E2E_ADMIN_PASS || "admin123";
const adminTotpCode = process.env.E2E_TOTP_CODE || "";
const nonAdminUsername = process.env.E2E_NON_ADMIN_USER || "guest";
const nonAdminPassword = process.env.E2E_NON_ADMIN_PASS || "guest123";
const nonAdminTotpCode = process.env.E2E_NON_ADMIN_TOTP_CODE || "";

const viewports = [
  { name: "desktop", width: 1280, height: 720 },
  { name: "mobile", width: 768, height: 1024 },
];

const pages = [
  {
    name: "home",
    path: "/",
    waitFor: "text=少人数向けの勤怠管理システム",
    requiresAuth: false,
  },
  { name: "login", path: "/login", waitFor: "#username", requiresAuth: false },
  { name: "dashboard", path: "/dashboard", waitFor: "text=ダッシュボード", requiresAuth: true },
  { name: "attendance", path: "/attendance", waitFor: "text=勤怠管理", requiresAuth: true },
  { name: "requests", path: "/requests", waitFor: "text=申請管理", requiresAuth: true },
  {
    name: "mfa",
    path: "/mfa/register",
    waitForAny: [
      'h1:has-text("MFA 設定")',
      'button:has-text("シークレットを発行")',
      "text=/MFA\\s*設定/",
    ],
    waitForUrl: "**/mfa/register",
    requiresAuth: true,
  },
  { name: "admin", path: "/admin", waitFor: "text=管理者ツール", requiresAuth: true },
  {
    name: "admin-users",
    path: "/admin/users",
    waitForAny: [
      "text=ユーザー管理",
      "text=このページはシステム管理者のみ利用できます。",
    ],
    requiresAuth: true,
  },
  {
    name: "admin-export-empty",
    path: "/admin/export",
    waitFor: "text=データエクスポート",
    requiresAuth: true,
    expectAbsentSelector: "text=プレビュー (先頭2KB)",
  },
];

const unauthorizedPages = [
  {
    name: "admin-unauthorized",
    path: "/admin",
    waitFor: "text=このページは管理者以上の権限が必要です。",
  },
];

const log = (message) => console.log(`[screenshots] ${message}`);

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const outputDir = process.env.SCREENSHOT_DIR || path.join(__dirname, "screenshots");

fs.mkdirSync(outputDir, { recursive: true });

async function login(page, { username, password, totp }) {
  log("ログイン開始");
  await page.goto(`${base}/login`, { waitUntil: "domcontentloaded" });
  await page.fill("#username", username);
  await page.fill("#password", password);
  if (totp) {
    await page.fill("#totp_code", totp);
  }
  await page.click('button[type="submit"]');
  await page.waitForURL("**/dashboard", { timeout: 20000 });
  log("ログイン成功");
}

async function waitForAnySelector(page, selectors, timeoutMs) {
  const tasks = selectors.map((selector) =>
    page.waitForSelector(selector, { timeout: timeoutMs })
  );
  await Promise.any(tasks);
}

async function capturePage(page, viewportName, pageDef) {
  const url = `${base}${pageDef.path}`;
  log(`${viewportName} ${pageDef.name} を取得中: ${url}`);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  if (pageDef.waitForUrl) {
    await page.waitForURL(pageDef.waitForUrl, { timeout: 20000 });
  }
  if (pageDef.waitForAny && pageDef.waitForAny.length > 0) {
    try {
      await waitForAnySelector(page, pageDef.waitForAny, 20000);
    } catch (error) {
      if (!pageDef.allowMissingSelector) {
        throw error;
      }
      log(
        `${pageDef.name} の待機セレクタに失敗しました。現在のURL: ${page.url()}`
      );
    }
  } else if (pageDef.waitFor) {
    try {
      await page.waitForSelector(pageDef.waitFor, { timeout: 20000 });
    } catch (error) {
      if (!pageDef.allowMissingSelector) {
        throw error;
      }
      log(
        `${pageDef.name} の待機セレクタに失敗しました。現在のURL: ${page.url()}`
      );
    }
  }
  if (pageDef.expectAbsentSelector) {
    const count = await page.locator(pageDef.expectAbsentSelector).count();
    if (count > 0) {
      throw new Error(
        `Expected ${pageDef.expectAbsentSelector} to be absent on ${pageDef.name}.`
      );
    }
  }
  const filename = `${pageDef.name}-${viewportName}.png`;
  const filePath = path.join(outputDir, filename);
  await page.screenshot({ path: filePath, fullPage: true });
  log(`保存: ${filePath}`);
}

const browser = await chromium.launch({ headless: true });
const hasNonAdminCredentials = Boolean(nonAdminUsername && nonAdminPassword);

try {
  if (!hasNonAdminCredentials) {
    log("非管理者ユーザーが未設定のため未許可画面をスキップします。");
    log("E2E_NON_ADMIN_USER と E2E_NON_ADMIN_PASS を設定してください。");
  }

  for (const viewport of viewports) {
    const context = await browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
    });
    const page = await context.newPage();
    page.setDefaultTimeout(20000);

    for (const pageDef of pages) {
      if (pageDef.requiresAuth) {
        break;
      }
      await capturePage(page, viewport.name, pageDef);
    }

    await login(page, {
      username: adminUsername,
      password: adminPassword,
      totp: adminTotpCode,
    });

    for (const pageDef of pages.filter((p) => p.requiresAuth)) {
      await capturePage(page, viewport.name, pageDef);
    }

    await context.close();

    if (hasNonAdminCredentials) {
      const nonAdminContext = await browser.newContext({
        viewport: { width: viewport.width, height: viewport.height },
      });
      const nonAdminPage = await nonAdminContext.newPage();
      nonAdminPage.setDefaultTimeout(20000);

      await login(nonAdminPage, {
        username: nonAdminUsername,
        password: nonAdminPassword,
        totp: nonAdminTotpCode,
      });

      for (const pageDef of unauthorizedPages) {
        await capturePage(nonAdminPage, viewport.name, pageDef);
      }

      await nonAdminContext.close();
    }
  }

  await browser.close();
  log("スクリーンショット取得完了");
  process.exit(0);
} catch (error) {
  console.error("[screenshots] 失敗:", error);
  try {
    await browser.close();
  } catch {
    // ignore
  }
  process.exit(1);
}
