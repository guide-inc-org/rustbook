# FAQ

## インストール

### guidebook を使うのに Rust は必要？

いいえ。macOS、Linux、Windows 用のビルド済みバイナリが利用可能です。インストールスクリプトを実行するだけです。

### guidebook をアップデートするには？

```bash
guidebook update
```

## 使い方

### 出力先はどこ？

デフォルトでは、`guidebook build` は `_book/` に出力します。`-o` で変更可能：

```bash
guidebook build -o dist
```

### ポートを変更するには？

```bash
guidebook serve -p 3000
```

### 開発中に検索が動かないのはなぜ？

パフォーマンス向上のため、ホットリロード時に検索インデックスは再生成されません。`guidebook serve` を再起動すると検索インデックスが更新されます。

## 互換性

### HonKit プロジェクトで動く？

はい、guidebook はドロップイン置き換えです。`npx honkit build` の代わりに `guidebook build` を実行するだけです。

### JavaScript プラグインは使える？

いいえ、guidebook は組み込みの Rust 実装を使用します。一般的なプラグイン（折りたたみチャプター、トップに戻る、mermaid）はネイティブでサポートされています。

### PDF エクスポートはできる？

現在はサポートしていません。guidebook は Web 出力に特化しています。

## トラブルシューティング

### "Command not found: guidebook"

インストールディレクトリを PATH に追加：

```bash
export PATH="$PATH:$HOME/.local/bin"
```

この行を `~/.zshrc` または `~/.bashrc` に追加してください。

### "SUMMARY.md not found" でビルド失敗

`SUMMARY.md` を含むディレクトリで `guidebook build` を実行していることを確認してください。

### 画像が表示されない

画像パスが Markdown ファイルからの相対パスであることを確認：

```
![Image](./assets/image.png)
```
