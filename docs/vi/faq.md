# FAQ

## Cài đặt

### Tôi có cần Rust để sử dụng guidebook không?

Không. Binary được build sẵn có sẵn cho macOS, Linux và Windows. Chỉ cần chạy script cài đặt.

### Làm sao để cập nhật guidebook?

```bash
guidebook update
```

## Sử dụng

### Output ở đâu?

Mặc định, `guidebook build` xuất ra `_book/`. Bạn có thể thay đổi bằng `-o`:

```bash
guidebook build -o dist
```

### Làm sao để thay đổi port?

```bash
guidebook serve -p 3000
```

### Tại sao tìm kiếm không hoạt động khi đang phát triển?

Search index không được tạo lại khi hot reload để cải thiện hiệu suất. Khởi động lại `guidebook serve` để cập nhật search index.

## Tương thích

### Có hoạt động với dự án HonKit của tôi không?

Có, guidebook là sự thay thế trực tiếp. Chỉ cần chạy `guidebook build` thay vì `npx honkit build`.

### Tôi có thể sử dụng plugin JavaScript không?

Không, guidebook sử dụng implementation Rust tích hợp. Các plugin phổ biến (collapsible chapters, back-to-top, mermaid) được hỗ trợ native.

### Có hỗ trợ export PDF không?

Hiện tại chưa. guidebook tập trung vào output web.

## Khắc phục sự cố

### "Command not found: guidebook"

Thêm thư mục cài đặt vào PATH:

```bash
export PATH="$PATH:$HOME/.local/bin"
```

Thêm dòng này vào `~/.zshrc` hoặc `~/.bashrc`.

### Build thất bại với "SUMMARY.md not found"

Đảm bảo bạn đang chạy `guidebook build` trong thư mục chứa `SUMMARY.md`.

### Hình ảnh không hiển thị

Kiểm tra đường dẫn hình ảnh là tương đối từ file markdown:

```
![Image](./assets/image.png)
```
