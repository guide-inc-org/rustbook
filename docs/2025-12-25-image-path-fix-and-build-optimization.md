# 対応記録: 画像パス修正 & ビルド時間短縮

**日付:** 2025-12-25
**バージョン:** v0.1.10

---

## 1. 画像表示されない問題の修正

### 問題

guidebookでビルドしたドキュメントをWebサーバー（S3等）にホスティングすると、画像が表示されない（403エラー）。ローカルでは正常に表示される。HonKitでビルドした場合は問題なし。

### 原因

テンプレートに `<base href="{{ root_path }}">` タグがあり、これがすべての相対パスの基準を変更していた。

```html
<!-- 問題のあったコード -->
<base href="../../">
```

HTMLファイル `jp/Customer/AssetStatus/PortfolioTop.html` から画像パス `../../../assets/...` を解決すると:

- `<base>` あり: `/assets/...` (ルート直下を参照 → 404/403)
- `<base>` なし: `/kcmsr-guidebook/assets/...` (正しいパス)

### 修正内容

**ファイル:** `src/builder/template.rs`

1. `<base href>` タグを削除
2. CSS/JS/サイドバーリンクに `root_path` を直接追加

```html
<!-- Before -->
<base href="{{ root_path }}">
<link rel="stylesheet" href="gitbook/gitbook.css">

<!-- After -->
<link rel="stylesheet" href="{{ root_path }}gitbook/gitbook.css">
```

---

## 2. ビルド時間の短縮

### 問題

毎回のCI実行で:

| ステップ | 時間 |
|---------|------|
| Install Rust and Cargo | 15秒 |
| cargo install guidebook | 1分16秒 |
| **合計** | **1分31秒** |

Rustのインストールとソースからのコンパイルが毎回発生していた。

### 解決策: pre-builtバイナリ方式

#### A. guidebookリポジトリにリリースワークフロー追加

**ファイル:** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build release binary
        run: cargo build --release

      - name: Create release archive
        run: |
          cd target/release
          tar -czvf guidebook-linux-x86_64.tar.gz guidebook

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: target/release/guidebook-linux-x86_64.tar.gz
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

タグ（例: `v0.1.10`）をプッシュすると、Linux用バイナリが自動でGitHub Releasesに公開される。

#### B. 利用側のワークフロー変更

**Before:**
```yaml
- name: Install Rust and Cargo
  run: |
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    echo "$HOME/.cargo/bin" >> $GITHUB_PATH

- name: Install guidebook
  run: cargo install guidebook

- name: Build guidebook
  run: guidebook build
```

**After:**
```yaml
- name: Install guidebook
  run: |
    curl -sL https://github.com/guide-inc-org/guidebook/releases/latest/download/guidebook-linux-x86_64.tar.gz | tar xz
    chmod +x guidebook
    ./guidebook build
```

### 結果

| ステップ | Before | After |
|---------|--------|-------|
| Install Rust | 15秒 | 削除 |
| Install guidebook | 1分16秒 | 4秒 |
| **合計** | **1分31秒** | **4秒** |

---

## 3. 公開

- guidebook v0.1.10 を crates.io に公開
- GitHub Releases に Linux バイナリを公開

---

## 関連コミット

- guidebook: `Fix image paths by removing <base> tag`
- guidebook: `Add release workflow for pre-built binaries`
- kcmsr-member-site-spec: `Use pre-built binary instead of cargo install`
