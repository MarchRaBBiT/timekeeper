import { chromium } from "playwright";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const base = process.env.FRONTEND_BASE_URL || "http://localhost:8080";
const adminUsername = process.env.E2E_ADMIN_USER || "admin";
const adminPassword = process.env.E2E_ADMIN_PASS || "admin123";
const adminTotpCode = process.env.E2E_TOTP_CODE || "";
const nonAdminUsername = process.env.E2E_NON_ADMIN_USER || "";
const nonAdminPassword = process.env.E2E_NON_ADMIN_PASS || "";
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
  {
    name: "dashboard",
    path: "/dashboard",
    waitForAny: ["text=勤務サマリー", "text=ダッシュボード"],
    requiresAuth: true,
  },
  { name: "attendance", path: "/attendance", waitFor: "text=勤怠管理", requiresAuth: true },
  { name: "requests", path: "/requests", waitFor: "text=申請管理", requiresAuth: true },
  {
    name: "mfa",
    path: "/mfa/register",
    waitForAny: [
      'h1:has-text("MFA 設定")',
      'button:has-text("シークレットを発行")',
      "text=MFA 設定",
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
      "button:has-text(\"新規作成\")",
    ],
    requiresAuth: true,
  },
  {
    name: "admin-export-empty",
    path: "/admin/export",
    waitForAny: ['h2:has-text("データエクスポート")', "text=データエクスポート"],
    requiresAuth: true,
    expectAbsentSelector: "text=プレビュー (先頭2KB)",
  },
  {
    name: "users",
    path: "/settings",
    waitFor: 'h2:has-text("パスワード変更")',
    requiresAuth: true,
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
    page.locator(selector).first().waitFor({ state: "visible", timeout: timeoutMs })
  );
  await Promise.any(tasks);
}

async function settlePage(page) {
  await page.waitForLoadState("domcontentloaded", { timeout: 10000 }).catch(() => { });
  await page.waitForLoadState("networkidle", { timeout: 10000 }).catch(() => { });
}



const browser = await chromium.launch({ headless: true });
const hasNonAdminCredentials = Boolean(nonAdminUsername && nonAdminPassword);

try {
  if (!hasNonAdminCredentials) {
    log("非管理者ユーザーが未設定のため未許可画面をスキップします。");
    log("E2E_NON_ADMIN_USER と E2E_NON_ADMIN_PASS を設定してください。");
  }

  const colorSchemes = ["light", "dark"];

  for (const viewport of viewports) {
    for (const colorScheme of colorSchemes) {
      log(`==== ${viewport.name} (${colorScheme}) ====`);
      const context = await browser.newContext({
        viewport: { width: viewport.width, height: viewport.height },
        colorScheme: colorScheme,
      });
      const page = await context.newPage();
      page.setDefaultTimeout(30000);

      // Helper to capture with correct filename suffix
      const capture = async (p, pageDef) => {
        const url = `${base}${pageDef.path}`;
        log(`${viewport.name}-${colorScheme} ${pageDef.name} を取得中: ${url}`);
        await p.goto(url, { waitUntil: "domcontentloaded" });
        await settlePage(p);

        if (pageDef.waitForUrl) {
          await p.waitForURL(pageDef.waitForUrl, { timeout: 30000 });
        }

        if (pageDef.waitForAny && pageDef.waitForAny.length > 0) {
          try {
            await waitForAnySelector(p, pageDef.waitForAny, 30000);
          } catch (error) {
            if (!pageDef.allowMissingSelector) throw error;
            log(`${pageDef.name} selector missing but allowed.`);
          }
        } else if (pageDef.waitFor) {
          try {
            await p.locator(pageDef.waitFor).first().waitFor({ state: "visible", timeout: 30000 });
          } catch (error) {
            if (!pageDef.allowMissingSelector) throw error;
            log(`${pageDef.name} selector missing but allowed.`);
          }
        }

        if (pageDef.expectAbsentSelector) {
          const count = await p.locator(pageDef.expectAbsentSelector).count();
          if (count > 0) throw new Error(`Expected ${pageDef.expectAbsentSelector} to be absent.`);
        }

        const filename = `${pageDef.name}-${viewport.name}-${colorScheme}.png`;
        const filePath = path.join(outputDir, filename);
        await p.screenshot({ path: filePath, fullPage: true });
        log(`保存: ${filePath}`);
      };

      for (const pageDef of pages) {
        if (pageDef.requiresAuth) {
          break;
        }
        await capture(page, pageDef);
      }

      await login(page, {
        username: adminUsername,
        password: adminPassword,
        totp: adminTotpCode,
      });

      for (const pageDef of pages.filter((p) => p.requiresAuth)) {
        await capture(page, pageDef);
      }

      await context.close();

      if (hasNonAdminCredentials) {
        const nonAdminContext = await browser.newContext({
          viewport: { width: viewport.width, height: viewport.height },
          colorScheme: colorScheme,
        });
        const nonAdminPage = await nonAdminContext.newPage();
        nonAdminPage.setDefaultTimeout(30000);

        await login(nonAdminPage, {
          username: nonAdminUsername,
          password: nonAdminPassword,
          totp: nonAdminTotpCode,
        });

        const unauthorizedCapture = async (p, pageDef) => {
          // simplified capture logic for unauthorized pages check (usually just wait for text)
          const url = `${base}${pageDef.path}`;
          log(`${viewport.name}-${colorScheme} ${pageDef.name} (Non-Admin) を取得中: ${url}`);
          await p.goto(url, { waitUntil: "domcontentloaded" });
          await settlePage(p);
          if (pageDef.waitFor) {
            await p.locator(pageDef.waitFor).first().waitFor({ state: "visible", timeout: 30000 });
          }
          const filename = `${pageDef.name}-${viewport.name}-${colorScheme}.png`;
          const filePath = path.join(outputDir, filename);
          await p.screenshot({ path: filePath, fullPage: true });
          log(`保存: ${filePath}`);
        };

        for (const pageDef of unauthorizedPages) {
          await unauthorizedCapture(nonAdminPage, pageDef);
        }

        await nonAdminContext.close();
      }
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
