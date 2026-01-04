# Cấu trúc dự án

## Cấu trúc cơ bản

```
my-book/
├── book.json       # Cấu hình (tùy chọn)
├── README.md       # Giới thiệu
├── SUMMARY.md      # Mục lục
├── chapter1.md
├── chapter2/
│   ├── README.md   # Giới thiệu chương 2
│   ├── section1.md
│   └── section2.md
├── assets/
│   └── images/
└── styles/
    └── website.css # CSS tùy chỉnh
```

## File bắt buộc

### SUMMARY.md

Định nghĩa mục lục và cấu trúc navigation:

```markdown
# Mục lục

* [Giới thiệu](README.md)
* [Bắt đầu](getting-started.md)
* [Chủ đề nâng cao](advanced/README.md)
  * [Chủ đề 1](advanced/topic1.md)
  * [Chủ đề 2](advanced/topic2.md)
```

### README.md

Trang giới thiệu, trở thành `index.html`.

## File tùy chọn

### book.json

File cấu hình. Xem [Cấu hình](config.md).

### LANGS.md

Cho sách đa ngôn ngữ:

```markdown
# Languages

* [English](en/)
* [日本語](ja/)
* [Tiếng Việt](vi/)
```

## Assets

Đặt hình ảnh và assets khác trong thư mục `assets/`:

```
![Image](assets/images/screenshot.png)
```

Đường dẫn tương đối cũng được hỗ trợ:

```
![Image](../assets/images/screenshot.png)
```
