# プロジェクト構造

## 基本構造

```
my-book/
├── book.json       # 設定（オプション）
├── README.md       # イントロダクション
├── SUMMARY.md      # 目次
├── chapter1.md
├── chapter2/
│   ├── README.md   # 第2章イントロ
│   ├── section1.md
│   └── section2.md
├── assets/
│   └── images/
└── styles/
    └── website.css # カスタムスタイル
```

## 必須ファイル

### SUMMARY.md

目次とナビゲーション構造を定義：

```markdown
# 目次

* [はじめに](README.md)
* [入門](getting-started.md)
* [上級トピック](advanced/README.md)
  * [トピック1](advanced/topic1.md)
  * [トピック2](advanced/topic2.md)
```

### README.md

イントロダクションページ、`index.html` になります。

## オプションファイル

### book.json

設定ファイル。詳細は[設定](config.md)を参照。

### LANGS.md

多言語ブック用：

```markdown
# Languages

* [English](en/)
* [日本語](ja/)
```

## アセット

画像などのアセットは `assets/` フォルダに配置：

```
![Image](assets/images/screenshot.png)
```

相対パスも使用可能：

```
![Image](../assets/images/screenshot.png)
```
