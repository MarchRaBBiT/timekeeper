## コーディング規約（関数設計・リファクタリング・コード探索）

この節の規約は、すべてのコーディングエージェントが共通して守ること。
以下に要約版を記載する。

---

### 1. 関数設計

#### 1.1 単一責務の原則

関数は「単一責務」を原則とする。1つの関数に複数の関心事（I/O、ビジネスロジック、フォーマット変換、ログ出力など）を混在させてはならない。

**具体例**（混在している悪い例）:
```rust
// ❌ 悪い例: I/O、ビジネスロジック、検証が混在
async fn handle_user_request(user_id: Uuid) -> Result<impl IntoResponse> {
    // I/O: データベースからユーザー情報を取得
    let user = get_user_from_db(user_id).await?;
    
    // ビジネスロジック: 権限チェック
    if user.role != Role::Admin {
        return Err(AppError::Forbidden);
    }
    
    // I/O: 勤怠データを取得
    let attendance = get_attendance(user_id).await?;
    
    // I/O: CSV エクスポート
    let csv = export_attendance_to_csv(&attendance).await?;
    
    // 検証: 入力検証
    validate_attendance_request(&user, &attendance)?;
    
    Ok(Response::new(csv))
}
```

**改善例**（責務分離）:
```rust
// ✅ 良い例: 各層に委譲
async fn handle_user_request(user_id: Uuid) -> Result<impl IntoResponse> {
    // データ取得は Repository に委譲
    let user = user_repo.get_by_id(user_id).await?;
    
    // 権限チェックは Service に委譲
    authorization_service.check_admin_permission(&user)?;
    
    // CSV エクスポートは別モジュール
    let csv = attendance_export_service.generate(user_id).await?;
    
    Ok(Response::new(csv))
}
```

#### 1.2 関数サイズと複雑度の判断基準

以下の基準のどれかに該当する場合、「分割候補」と判断すること。

| 基準 | 閾値 | 例 |
|--------|--------|------|
| 論理的段落が3つ以上ある | 条件分岐、エラーハンドリング、ログ記録などが含まれる場合 |
| ネストが3段以上ある | if 文の中に if 文、または match 文が深くネストしている場合 |
| 複数の外部コンポーネント（3つ以上）に直接アクセスしている | DB、外部API、ファイルシステムなど、関数内で直接3つ以上の外部リソースを操作している場合 |

**具体例**:
```rust
// ❌ 分割候補: 3つの論理段落 + 3段ネスト
async fn process_request(req: Request) -> Result<Response> {
    // 段落1: 入力検証
    if !is_valid(&req) {
        return Err(AppError::BadRequest);
    }
    
    // 段落2: 認証
    let user = authenticate(&req)?;
    if !user.is_active {
        return Err(AppError::Unauthorized);
    }
    
    // 段落3: ビジネスロジック（ネスト）
    if user.role == Role::Admin {
        if requires_special_permission(&req) { // ネストレベル1
            if !has_permission(&user, &req) { // ネストレベル2
                return Err(AppError::Forbidden);
            }
        }
    }
    
    // 段落4: 処理
    let result = execute_business_logic(&user, &req)?;
    
    // 段落5: ログ記録
    log_activity(&user, &result);
    
    Ok(Response::new(result))
}
```

```rust
// ✅ 改善例: 責務を分離
async fn process_request(req: Request) -> Result<Response> {
    // 責務1: 入力検証
    validate_request(&req)?;
    
    // 責務2: 認証
    let user = authenticate(&req)?;
    
    // 責務3: 権限チェック（Service に委譲）
    authorization_service.check_permission(&user, &req)?;
    
    // 責務4: ビジネスロジック（Service に委譲）
    let result = business_service.execute(&user, &req)?;
    
    // 責務5: ログ記録（別関数）
    log_result(&result);
    
    Ok(Response::new(result))
}
```

#### 1.3 密結合関数の扱い

既存コードにおいて、すでに多機能な関数（いわゆる God function）を見つけた場合、さらに別の責務を追加してはならない。

**NG な対応**（禁止）:
```rust
// ❌ 悪い例: さらに別の責務を追加
async fn handle_everything(user_id: Uuid, action: Action, params: Params) -> Result<impl IntoResponse> {
    match action {
        Action::Login => login(user_id, params),
        Action::Logout => logout(user_id),
        Action::Update => update(user_id, params),
        // ... 他の 10 個以上のアクション
    }
}
```

**良い対応**（分離か委譲）:
```rust
// ✅ 良い例: 責務を委譲
// 各アクションは別ハンドラー/Service 関数
mod auth_handlers;
mod user_handlers;
mod attendance_handlers;
```

---

### 2. リファクタリング時の絶対禁止事項

#### 2.1 空関数・空実装への置き換え禁止

「リファクタリング」「整理」「関数分割」などの指示を受けた場合でも、既存のロジックを中身のない関数に置き換えたり、振る舞いを削ったりしてはならない。

**NG な例**:
```rust
// ❌ 悪い例: 空関数に置き換え
async fn complex_validation_logic(user: &User, data: &Data) -> Result<bool> {
    // TODO: あとで実装する
    unimplemented!()
}

// 既存コード:
if complex_validation_logic(&user, &data)? {
    // ビジネスロジック
}
```

**OK な対応**:
```rust
// ✅ 良い例: 必ず動作する実装を作成
fn complex_validation_logic(user: &User, data: &Data) -> Result<bool> {
    // 実装を含める
    validate_user_data(user)?;
    validate_data_constraints(data)?;
    Ok(true)
}
```

#### 2.2 ユーザーから明示的な許可がない限り、挙動や仕様を簡略化・削除を禁止

ユーザーから明示的な許可がない限り、「仕様簡略化」「分岐削除」「異常系の削除」「ログやバリデーションの削除」など、外部から観測可能な挙動を変える変更をしてはならない。判断に迷う場合は削らない。

**NG な例**:
```rust
// ❌ 悪い例: ユーザー許可なくエラー処理を削除
// 指示: 「エラー処理は不要そう」
fn process_result(result: Result<Report>) -> Response {
    match result {
        Ok(report) => Response::new(report),  // 成功のみ返す
        Err(e) => Response::new(e.to_string()), // エラーは捨てる
    }
}
```

**OK な対応**:
```rust
// ✅ 良い例: 問題点を特定してユーザーに確認
fn process_result(result: Result<Report>) -> Response {
    match result {
        Ok(report) => {
            info!("Report generated successfully: {}", report.id);
            Response::new(report)
        },
        Err(e) => {
            warn!("Report generation failed: {}", e);
            Response::new(e.to_string())
        }
    }
}
```

#### 2.3 不完全な分割禁止

関数分割の際、元の関数の一部だけを新関数に移し、残りのロジックを「後で書く」前提で削ることは禁止。「リファクタリング後の構造だけ提示し、実装は空にする」ような抽象的な提案は行わない。必ず動作する実装まで落とし込むこと。

**NG な例**:
```rust
// ❌ 悪い例: 一部のみ抽出、残りは「後で実装」
fn extract_validation_logic(data: &Data) -> ValidationContext {
    // 入力検証のみ抽出
    ValidationContext {
        user_data: data.user_data.clone(),
        // 他の複雑なロジックは「後で実装」として削除
    }
}

// 元の関数:
async fn process_request(data: &Data) -> Result<Response> {
    let ctx = extract_validation_logic(data);
    
    // 残りのロジックは「後で実装」として空関数呼び出し
    let result = ctx.execute_complex_logic()?; // エラー発生！
    
    Ok(Response::new(result))
}
```

**OK な対応**:
```rust
// ✅ 良い例: 責務を明確に分離
fn extract_validation_logic(data: &Data) -> ValidationContext {
    ValidationContext {
        user_data: data.user_data.clone(),
        business_rules: load_business_rules(data)?,
    }
}

async fn process_request(data: &Data) -> Result<Response> {
    let ctx = extract_validation_logic(data);
    
    // 実装を含める（「後で実装」で空にしない）
    let result = ctx.validate_and_execute()?;
    
    Ok(Response::new(result))
}
```

---

### 3. リファクタリングの手順（強制プロセス）

リファクタリングを行う場合、以下のステップを内部的に必ず踏むこと（ユーザーへの出力は要求された範囲でよい）。

#### 3.1 現状挙動の把握

対象関数について、現在の挙動を箇条書きで整理すること：

- **入力の前提条件**: 何を受け取るか？検証済みか？
- **正常系で行っている処理の流れ**: 成功時のフロー
- **エラー例外ケースの扱い**: どのようなエラーが発生しうるか？
- **副作用（ロギング、DB更新、外部API呼び出し、ファイル操作など）**: 関数が外部に与える影響

**テンプレート例**:
```rust
/// 対象関数: `process_attendance_export`
/// 
/// [入力の前提条件]
/// - user_id は有効な UUID
/// - user は `is_active = true` の状態
/// - export_date は当月以内
///
/// [正常系フロー]
/// 1. 勤怠データを取得
/// 2. CSV フォーマットに変換
/// 3. ファイルに保存
/// 4. 保存結果を返す
///
/// [副作用]
/// - DB 参照: `attendance` テーブルを参照
/// - ファイル操作: 一時ファイルを作成・削除
/// - ロギング: エクスポート開始/完了を記録
///
/// [エラー例外ケース]
/// - ユーザーが見つからない: AppError::NotFound
/// - 権限エラー: AppError::Forbidden
/// - データベースエラー: AppError::DatabaseError
/// - ファイル書き込みエラー: AppError::IoError
```

#### 3.2 分割方針の決定

「どのロジックをどの新関数に切り出すか」を決め、簡潔な関数名と責務を内部的に定義すること。

**良い例**:
```rust
// ❌ 悪い: 「処理を分離」程度の指示
// 「DB 処理を extract_db_logic に移す」では不十分

// ✅ 良い: 責務を明確に定義
/// [新しい関数の責務]
/// 入力パラメータの検証を行う
fn validate_request_params(params: &ExportParams) -> Result<ValidatedParams> {
    check_user_active(params.user_id)?;
    check_date_range(params.from_date, params.to_date)?;
    Ok(params.clone())
}

/// [新しい関数の責務]
/// データベースから勤怠データを取得する
fn fetch_attendance_data(user_id: Uuid, date_range: &DateRange) -> Result<Vec<AttendanceRecord>> {
    attendance_repo.get_by_user_and_date(user_id, date_range)
}

/// [新しい関数の責務]
/// 取得したデータを CSV に変換する
fn convert_to_csv(records: &[AttendanceRecord]) -> String {
    csv_formatter.format(records)
}
```

#### 3.3 実装と再確認

新しい関数群を実装した後、元の関数から呼び出すよう書き換え、元の挙動の箇条書きと新実装を照合し、「どの行がどの責務を担っているか」を対応づけること。

**照合チェックリスト**:
- [ ] 入力検証が正しく移動しているか？
- [ ] エラーハンドリングが維持されているか？
- [ ] 副作用（ロギング、DB更新）が維持されているか？
- [ ] 成功時の処理内容が同じか？

**実装例**:
```rust
// 変更後の元関数
async fn process_attendance_export(user_id: Uuid, date_range: ExportParams) -> Result<Response> {
    // 前処理: 入力検証（分離）
    let validated = validate_request_params(&ExportParams {
        user_id,
        date_range: date_range.clone(),
    })?;
    
    // 前処理: データ取得（分離）
    let records = fetch_attendance_data(user_id, &date_range)?;
    
    // 前処理: CSV 変換（分離）
    let csv = convert_to_csv(&records);
    
    // 前処理: 結果返却（新規追加）
    save_export_result(user_id, &csv)?;
    
    Ok(Response::new(csv))
}
```

#### 3.4 テストコードが存在する場合の対応

テストコードが存在する場合、それを利用する前提でコードを書く（実行はユーザー環境依存だが、どのテストが影響を受けるかまで言及すること）。

**NG な例**:
```rust
// ❌ 悪い: テスト前提を無視して実装
#[cfg(test)]  // このフラグが意味不明
fn get_test_data() -> TestData {
    // テスト用データを直接返す
}
```

**OK な対応**:
```rust
// ✅ 良い: テスト前提を明示
/// テストモードでのみ使用可能
#[cfg(test)]
pub fn get_test_data() -> TestData {
    TestData {
        users: vec![/* ... */],
    }
}

/// 本番モード用の実装
pub fn get_production_data(user_id: Uuid) -> Result<ProductionData> {
    user_repo.get_by_id(user_id)?
}
```

---

### 4. コード探索ポリシー（目的コード片の特定）

#### 4.1 線形読み込みの禁止

大きなファイルやリポジトリ全体に対して、「先頭から順に読む」ことをデフォルトの探索戦略としてはならない。

**NG な例**:
```
// ❌ 悪い: 全体を順に読み込む
1. src/main.rs を 1 行目から 126 行目まで読む
2. src/handlers/ を 1 行目から読む
3. ... 全体を読み終えてから分析開始
```

**OK な例**:
```
// ✅ 良い: 目的に合わせて焦点を絞る
1. ユーザー登録機能を調査する場合:
   - `handlers/auth.rs` の login 関数
   - `models/user.rs` の User 構造体
   - `handlers/auth.rs` の register 関数
   
2. すべての探索を完了してから、全体像を整理
```

#### 4.2 探索の初期動作

コード片を探すタスクを受けた場合、以下の順序で行うこと：

1. **目的のコードの役割を 1 行で定義する**（例：「ユーザー登録フォームのバリデーションロジック」）
2. **その役割から推定されるキーワードを列挙する**（関数名/クラス名候補、ドメイン用語、API名など）
3. **これらのキーワードを使い、以下の情報ソースから該当箇所を絞り込む**:
   - ファイル名・ディレクトリ構造（例: `user`, `auth`, `signup` 等を含むパス）
   - コード中のシンボル名（関数、メソッド、クラス、インターフェース名）
   - コメントやドキュメント中の見出し・節タイトル

**探索の初期動作例**:
```
ユーザー: 「ユーザー登録フォームのバリデーションロジックを調査して」

Step 1: 目的の定義
  目的: ユーザー登録フォームのバリデーションロジック

Step 2: キーワードの列挙
  関数: validate_user_registration, validate_email, validate_password
  ドメイン用語: validation, email, password, user

Step 3: 情報ソースの優先順位
  1. `handlers/auth.rs` - 実装コード（最優先）
  2. `models/user.rs` - データモデル構造
  3. `validation/*.rs` - 検証ユーティリティ
  4. `AGENTS.md` - プロジェクト規約

Step 4: キーワードで絞り込み
  - `validate_user_registration` で `handlers/auth.rs` を検索
  - `User` 構造体で `validate()` メソッドの使用を確認
```

#### 4.3 補索が複数ある場合の優先度付け

ユーザーがファイルパス、クラス名、関数名などのポインタを提供した場合、それを最優先の手がかりとして扱うこと。これらのポインタを無視して、別の場所から読み始めてはならない。

**優先度順位**（高い順）:
1. **ユーザー提供のポインタ**（ファイルパス、クラス名、関数名など）
2. **ユーザー提供の説明文**（目的やコンテキストの説明）
3. **ユーザーの意図を推定**（ポインタから類推）

**良い対応例**:
```
ユーザー提供: 「src/handlers/auth.rs の login 関数を見て」

✅ 良い: ポインタを優先
  1. `src/handlers/auth.rs` の login 関数を開く
  2. login 関数周辺の処理を確認
  
❌ 悪い: ポインタを無視
  1. 全体を grep で検索（目的不明になる）
  2. すべての auth.rs ファイルを読む
```

#### 4.4 周辺コードの確認

検索対象となる箇所を特定した後、その周辺の関連コードを確認して理解を深めること。

**周辺確認の例**:
```
login 関数を特定した後の周辺確認:

1. 前後の文脈: 関数内の前後のコードを読み、文脈を理解
2. 関連する関数: login 関数から呼ばれている他の関数（authenticate, validate_credentials 等）
3. 依存モジュール: use 文で import されているモジュールを確認
4. テストコード: 同じ機能のテストコードがあるか確認
```

---

### 5. ドキュメントとコードの対応

#### 5.1 ドキュメントにも単一責務を適用

関数やモジュールの説明を書く場合、1つの節や段落に複数の責務を詰め込まないこと。「概要」「入出力」「副作用」「例外」「使用例」など、論理的なまとまりごとに節を分けること。

**良い例**:
```rust
/// ユーザー登録処理
///
/// # 概要
/// 新規ユーザー登録フォームの送信データを検証し、
/// データベースに保存する。
///
/// # 入出力
/// - 登録リクエスト（JSON）
///
/// # 副作用
/// - データベースにユーザーレコードを保存
/// - 検証メールを送信
///
/// # 使用例
/// ```rust
/// let request = RegistrationRequest { /* ... */ };
/// register_handler(request).await?;
/// ```
pub async fn register_handler(
    State(pool): State<PgPool>,
    Json(request): Json<RegistrationRequest>,
) -> Result<impl IntoResponse> {
    // 入力検証
    validate_email(&request.email)?;
    
    // 保存処理
    save_user_to_db(&pool, &request).await?;
    
    // メール送信
    send_verification_email(&request.email).await?;
    
    Ok(Response::new("Registration successful"))
}
```

#### 5.2 ドキュメント更新時の制約

コードのリファクタリングや関数分割を行った場合、関連するドキュメントの更新を必ず検討すること。

**更新時のチェックリスト**:
- [ ] 古い関数名や古い責務の説明を残したままにしていないか？
- [ ] コメントと実装が矛盾していないか？
- [ ] コードとドキュメントの整合性を確認したか？
- [ ] 関連するテストの更新が必要か？

---

この節の規約は、すべてのコーディングエージェントが共通して守ること。
