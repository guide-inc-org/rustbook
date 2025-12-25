# CLAUDE.md - guidebook

HonKit/GitBook互換の静的サイトジェネレーター

## プロジェクト概要

- **言語:** Rust
- **公開先:** crates.io (`cargo install guidebook`)
- **リポジトリ:** https://github.com/guide-inc-org/guidebook

## ビルド & テスト

```bash
# ビルド
cargo build --release

# テスト
cargo test

# ローカルでドキュメントをビルド
./target/release/guidebook build

# 開発サーバー起動
./target/release/guidebook serve
```

## リリース手順

1. `Cargo.toml` のバージョンを更新
2. コミット & プッシュ
3. タグを作成してプッシュ（GitHub Releasesにバイナリが自動生成される）
4. crates.io に公開

```bash
# バージョン更新後
git add -A && git commit -m "Bump version to vX.Y.Z"
git push origin main

# タグ作成 & プッシュ（リリースワークフローがトリガーされる）
git tag vX.Y.Z
git push origin vX.Y.Z

# crates.io に公開
cargo publish
```

## ディレクトリ構造

```
src/
├── main.rs          # CLI エントリーポイント
├── builder/
│   ├── mod.rs       # ビルド処理
│   ├── renderer.rs  # Markdown → HTML 変換
│   └── template.rs  # HTMLテンプレート
├── parser/
│   ├── mod.rs
│   ├── book_config.rs  # book.json パーサー
│   ├── langs.rs        # LANGS.md パーサー（多言語対応）
│   └── summary.rs      # SUMMARY.md パーサー
templates/
├── gitbook.css      # スタイルシート
├── gitbook.js       # クライアントJS
├── collapsible.js   # 折りたたみ機能
└── search.js        # 検索機能
```

## 重要な設計判断

### `<base>` タグは使用しない

**理由:** `<base href>` を使うと、マークダウン内の相対画像パス（例: `../../../assets/...`）がbase基準で解決され、サブディレクトリにデプロイした際に壊れる。

**対応:** CSS/JS/リンクには `root_path` を直接埋め込む（HonKitと同じ方式）

参照: `docs/2025-12-25-image-path-fix-and-build-optimization.md`

## CI/CD

### リリースワークフロー

`.github/workflows/release.yml` - タグプッシュ時にLinuxバイナリをGitHub Releasesに公開

利用側はpre-builtバイナリをダウンロードして使用:
```yaml
- name: Install guidebook
  run: |
    curl -sL https://github.com/guide-inc-org/guidebook/releases/latest/download/guidebook-linux-x86_64.tar.gz | tar xz
    ./guidebook build
```

## 主な利用プロジェクト

- kcmsr-member-site-spec (`develop-guidebook` ブランチ)
  - デプロイ先: https://gitbook.guide.inc/kcmsr-guidebook/

## 対応履歴

- **2025-12-25 v0.1.10:** 画像パス修正（`<base>`タグ削除）、リリースワークフロー追加
