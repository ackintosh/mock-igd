# mock-igd 設計ドキュメント

## 概要

UPnP IGD（Internet Gateway Device）のモックサーバー。クライアント実装のテストに使用する。

## アーキテクチャ

### UPnP IGDの構成要素

1. **SSDP**（UDP 1900）- デバイス発見プロトコル
2. **Device Description**（HTTP GET）- XML形式のデバイス情報
3. **SOAP Actions**（HTTP POST）- ポートマッピング等の操作

### モジュール構成

```
src/
├── lib.rs              # パブリックAPI
├── server/
│   ├── mod.rs
│   ├── ssdp.rs         # SSDP応答（M-SEARCH）
│   └── http.rs         # HTTP/SOAPサーバー
├── action/
│   ├── mod.rs
│   ├── types.rs        # UPnP IGDアクション定義
│   └── parser.rs       # SOAPリクエストパーサー
├── matcher/
│   ├── mod.rs
│   └── builder.rs      # リクエストマッチング条件
├── responder/
│   ├── mod.rs
│   ├── builder.rs      # レスポンス生成
│   └── templates.rs    # XML/SOAPテンプレート
└── mock.rs             # Mock登録・管理
```

## 振る舞い定義（Matcher + Responder パターン）

wiremock-rsに着想を得た設計。リクエストのマッチング条件とレスポンスを分離することで柔軟性を確保する。

### 基本的な使い方

```rust
use mock_igd::{MockIgdServer, Action, Responder};

#[tokio::test]
async fn test_get_external_ip() {
    let server = MockIgdServer::start().await;

    // 振る舞いを定義
    server.mock(
        Action::GetExternalIPAddress,
        Responder::success()
            .with_external_ip("203.0.113.1")
    ).await;

    // テスト対象のクライアントを実行
    let client = IgdClient::new(server.url());
    let ip = client.get_external_ip().await.unwrap();
    assert_eq!(ip, "203.0.113.1".parse().unwrap());
}
```

### 条件付きマッチング

```rust
// 特定のパラメータにマッチ
server.mock(
    Action::AddPortMapping
        .with_external_port(8080)
        .with_protocol("TCP"),
    Responder::success()
).await;

// 任意のリクエストにマッチ
server.mock(
    Action::any(),
    Responder::error(501, "ActionNotImplemented")
).await;
```

### エラーレスポンス

```rust
// UPnP標準エラーコード
server.mock(
    Action::AddPortMapping.with_external_port(80),
    Responder::error(718, "ConflictInMappingEntry")
).await;
```

### カスタムレスポンス

```rust
// 完全にカスタムなレスポンスを返す
server.mock(
    Action::GetExternalIPAddress,
    Responder::custom(|_request| {
        // 任意のロジック
        HttpResponse::Ok()
            .content_type("text/xml")
            .body(custom_soap_xml)
    })
).await;
```

## 主要な型定義

### Action（UPnP IGDアクション）

```rust
pub enum Action {
    // WANIPConnection
    GetExternalIPAddress,
    AddPortMapping(AddPortMappingMatcher),
    DeletePortMapping(DeletePortMappingMatcher),
    GetGenericPortMappingEntry(GetGenericPortMappingEntryMatcher),
    GetSpecificPortMappingEntry(GetSpecificPortMappingEntryMatcher),

    // WANCommonInterfaceConfig
    GetCommonLinkProperties,
    GetTotalBytesReceived,
    GetTotalBytesSent,

    // 任意のアクションにマッチ
    Any,
}
```

### Responder

```rust
pub struct Responder {
    kind: ResponderKind,
}

enum ResponderKind {
    Success(SuccessResponse),
    Error { code: u16, description: String },
    Custom(Box<dyn Fn(&SoapRequest) -> HttpResponse + Send + Sync>),
}
```

### Mock

```rust
pub struct Mock {
    action: Action,
    responder: Responder,
    priority: u32,        // 高いほど優先
    times: Option<u32>,   // 何回マッチするか（Noneは無制限）
}
```

## マッチング優先順位

1. `times`が残っているMockのみが対象
2. `priority`が高い順
3. 同じpriorityなら登録順（後勝ち）
4. 最初にマッチしたMockのResponderを使用
5. どれにもマッチしなければ404または501エラー

## 将来の拡張（Phase 2以降）

### Statefulモード

内部にポートマッピングテーブルを持ち、実際のIGDのように振る舞う。

```rust
let server = MockIgdServer::start()
    .with_stateful_behavior()
    .with_external_ip("203.0.113.1")
    .await;

// AddPortMappingで追加したエントリが
// GetGenericPortMappingEntryで取得できる
```

### 録画・再生モード

```rust
// 録画
let server = MockIgdServer::start()
    .record_to("fixtures/session.json")
    .await;

// 再生
let server = MockIgdServer::from_recording("fixtures/session.json").await;
```

### 設定ファイルからの読み込み

```rust
let server = MockIgdServer::from_config("mock-config.yaml").await;
```

## 依存クレート（予定）

- `tokio` - 非同期ランタイム
- `hyper` または `axum` - HTTPサーバー
- `quick-xml` - XML解析・生成
- `socket2` - UDPソケット（SSDP用）
