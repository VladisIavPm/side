use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};

const MAGIC: &[u8; 4] = b"SIDE";
const KEY: u8 = 0x5A;

fn encrypt(data: &[u8]) -> Vec<u8> {
    data.iter().map(|&b| b ^ KEY).collect()
}

fn decrypt(data: &[u8]) -> Vec<u8> {
    data.iter().map(|&b| b ^ KEY).collect()
}

pub fn build(source_path: &str, output_path: &str) -> Result<(), String> {
    // Читаем исходный код
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read source: {}", e))?;
    
    // Шифруем
    let encrypted = encrypt(source.as_bytes());
    let data_len = encrypted.len() as u32;
    
    // Получаем путь к текущему исполняемому файлу
    let self_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot get current exe path: {}", e))?;
    
    // Создаём выходной файл
    let mut out_file = fs::File::create(output_path)
        .map_err(|e| format!("Cannot create output file: {}", e))?;
    
    // Копируем текущий exe в выходной файл
    let mut self_file = fs::File::open(&self_exe)
        .map_err(|e| format!("Cannot open current exe: {}", e))?;
    io::copy(&mut self_file, &mut out_file)
        .map_err(|e| format!("Failed to copy exe: {}", e))?;
    
    // Дописываем в конец: зашифрованные данные, длину, маркер
    out_file.write_all(&encrypted)
        .map_err(|e| format!("Failed to write encrypted data: {}", e))?;
    out_file.write_all(&data_len.to_le_bytes())
        .map_err(|e| format!("Failed to write length: {}", e))?;
    out_file.write_all(MAGIC)
        .map_err(|e| format!("Failed to write magic: {}", e))?;
    
    Ok(())
}

/// Проверяет, есть ли встроенный скрипт в текущем exe
pub fn has_embedded() -> bool {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let mut file = match fs::File::open(exe_path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let file_len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return false,
    };
    if file_len < 8 {
        return false;
    }
    
    // Читаем последние 4 байта (маркер)
    let mut magic = [0u8; 4];
    if file.seek(SeekFrom::End(-4)).is_err() {
        return false;
    }
    if file.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == *MAGIC
}

/// Извлекает и расшифровывает встроенный скрипт
pub fn extract_embedded() -> Option<String> {
    let exe_path = std::env::current_exe().ok()?;
    let mut file = fs::File::open(exe_path).ok()?;
    let file_len = file.metadata().ok()?.len();
    if file_len < 8 {
        return None;
    }
    
    // Читаем маркер из последних 4 байт
    let mut magic = [0u8; 4];
    file.seek(SeekFrom::End(-4)).ok()?;
    file.read_exact(&mut magic).ok()?;
    if magic != *MAGIC {
        return None;
    }
    
    // Читаем длину (4 байта перед маркером)
    let mut len_bytes = [0u8; 4];
    file.seek(SeekFrom::End(-8)).ok()?;
    file.read_exact(&mut len_bytes).ok()?;
    let data_len = u32::from_le_bytes(len_bytes) as usize;
    
    // Читаем зашифрованные данные (перед длиной)
    let mut encrypted = vec![0u8; data_len];
    file.seek(SeekFrom::End(-(8 + data_len as i64))).ok()?;
    file.read_exact(&mut encrypted).ok()?;
    
    // Расшифровываем и возвращаем как строку
    let decrypted = decrypt(&encrypted);
    String::from_utf8(decrypted).ok()
}