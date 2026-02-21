use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::mpsc;
use eframe::egui;

#[repr(C)]
pub struct NativeFunction {
    pub name: *const c_char,
    pub args: u32,
    pub func: *const std::ffi::c_void,
}

#[repr(C)]
pub struct NativeModuleInfo {
    pub name: *const c_char,
    pub functions: *const NativeFunction,
    pub count: u32,
}

unsafe impl Sync for NativeFunction {}
unsafe impl Sync for NativeModuleInfo {}

// Приложение для окна с сообщением
struct MessageBoxApp {
    title: String,
    message: String,
    tx: mpsc::Sender<i64>,
}

impl eframe::App for MessageBoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&self.title);
            ui.label(&self.message);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    self.tx.send(1).unwrap();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui.button("Cancel").clicked() {
                    self.tx.send(0).unwrap();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

// Экспортируемая функция для Side
extern "C" fn message_box(title_ptr: *const c_char, message_ptr: *const c_char) -> i64 {
    let title = unsafe { CStr::from_ptr(title_ptr).to_string_lossy().into_owned() };
    let message = unsafe { CStr::from_ptr(message_ptr).to_string_lossy().into_owned() };

    let (tx, rx) = mpsc::channel();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 150.0]),
        ..Default::default()
    };

    let app = MessageBoxApp {
        title: title.clone(),
        message,
        tx,
    };

    // Запускаем окно синхронно (блокирует выполнение)
    if let Err(e) = eframe::run_native(
        &title,
        options,
        Box::new(|_cc| Box::new(app)),
    ) {
        eprintln!("eframe error: {}", e);
        return 0;
    }

    // Ждём результат от канала (он должен прийти до закрытия окна)
    match rx.recv() {
        Ok(result) => result,
        Err(_) => 0,
    }
}

// Таблица экспортируемых функций
const FUNCTIONS: [NativeFunction; 1] = [
    NativeFunction {
        name: b"message_box\0" as *const u8 as *const c_char,
        args: 2,
        func: message_box as *const std::ffi::c_void,
    },
];

// Информация о модуле
#[no_mangle]
pub static side_module_info: NativeModuleInfo = NativeModuleInfo {
    name: b"egui\0" as *const u8 as *const c_char,
    functions: FUNCTIONS.as_ptr(),
    count: FUNCTIONS.len() as u32,
};