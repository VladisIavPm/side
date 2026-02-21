use std::path::Path;

fn main() {
    // Проверяем, что мы на Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        
        // Указываем путь к иконке
        res.set_icon("assets/side.ico");
        
        // Добавляем информацию о версии (правильный синтаксис)
        res.set("FileDescription", "Side Language Interpreter");
        res.set("ProductName", "Side");
        res.set("ProductVersion", "1.2.0");
        res.set("FileVersion", "1.2.0");
        res.set("OriginalFilename", "side-interpreter.exe");
        res.set("InternalName", "side-interpreter");
        
        // Компилируем ресурсы
        res.compile().expect("Failed to compile resources");
    }
}