# ⚡ Side Language — Your Personal Programming Language

<p align="center">
  <img src="assets/side.ico" width="120" height="120" alt="Side Logo">
</p>

<p align="center">
  <strong>Side</strong> is a modern, lightweight, and incredibly fast programming language.<br>
  The interpreter is only <strong>350 KB</strong>, yet it can do more than some multi-gigabyte monsters.
</p>

<p align="center">
  <a href="https://side-lang.netlify.app/">📚 Documentation</a> •
  <a href="#🚀-features">Features</a> •
  <a href="#📦-installation">Installation</a> •
  <a href="#🎮-quick-start">Quick Start</a> •
  <a href="#⏱️-7-hours-that-changed-everything">Story</a>
</p>

---

## 🚀 **FEATURES**

| Feature | Description |
|---------|-------------|
| **⚡ 450 KB** | Interpreter lighter than a single photo |
| **🔌 Native Modules** | Load `.dll` written in Rust for GUI, 3D, networking |
| **🎨 GUI with egui** | Windows with buttons in 3 lines of code |
| **📦 .exe Builder** | Encryption + ready .exe in 0.1 seconds |
| **🎯 Own Syntax** | Simpler than Python, clearer than C++ |
| **🪟 Windows Icons** | `.sd` and `.spack` files look stylish |
| **📚 Documentation** | [side-lang.netlify.app](https://side-lang.netlify.app/) |

---

## 📦 **INSTALLATION**

### **Option 1: Installer (Windows)**
1. Download `Side_Setup.exe` from the [releases page](https://github.com/VladislavPm/side/releases)
2. Run and follow the instructions
3. Done! Side is already in your PATH

### **Option 2: Build from source**
```bash
git clone https://github.com/VladislavPm/side.git
cd side
cargo build --release
target\release\side.exe

🎮 QUICK START
Your first program = hello.sd

log "Hello, world!"

spack run Paste the path to .sd...

GUI with buttons

load_native("egui")
message_box("Side says:", "Hey bro!")

Build .exe
Create app.spack:
{
    "name": "MyApp",
    "version": "1.0",
    "main": "hello.sd"
}

spack build Paste the path to .spack...
Run MyApp.exe — and you're done!

📚 DOCUMENTATION
Full documentation is available at:
👉 side-lang.netlify.app

There you'll find:

Complete language guide

Code examples

Description of all functions

Native modules creation guide

Spack build system documentation

🏗️ WHAT SIDE CAN DO
Variables and types

set x = 10
fix PI = 3.14
set name = "Arthur"
set list = [1, 2, 3]

Conditions and loops

check x > 5 start
    log "x is greater than 5"
end

loop i < 10 start
    log i
    set i = i + 1
end

Functions and structures

proc add(a, b) start
    give a + b
end

form Player start
    set hp = 100
    set name = ""
end

File operations

write_file("data.txt", "Secret data")
set text = read_file("data.txt")
copy_file("backup.txt", "data.txt")
delete_file("temp.txt")

Native modules

// Rust module
#[no_mangle]
pub static side_module_info: NativeModuleInfo = NativeModuleInfo {
    name: b"mymodule\0" as *const u8 as *const c_char,
    functions: FUNCTIONS.as_ptr(),
    count: FUNCTIONS.len() as u32,
};

side;
load_native("mymodule")
hello("World!")

📝 LICENSE
MIT License — do whatever you want, but give us a shoutout if you can 😉

