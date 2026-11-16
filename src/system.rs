//! Windows-Integration: Ereigniskanal, Einzelinstanz, Hauptfenster, Tray, globaler Hotkey,
//! Autostart und Benachrichtigungen. Alles lokal, keine Netzwerkzugriffe.
use crate::reminders::Target;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::RefCell, os::windows::process::CommandExt, path::Path, process::Command, sync::OnceLock,
};
use windows_sys::Win32::{
    Foundation::{ERROR_ALREADY_EXISTS, GetLastError},
    System::Threading::CreateMutexW,
    UI::{
        Shell::SetCurrentProcessExplicitAppUserModelID,
        WindowsAndMessaging::{
            FindWindowW, IsIconic, SW_HIDE, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
        },
    },
};

fn native_handle(window: &gpui_kit::Window) -> Option<windows_sys::Win32::Foundation::HWND> {
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as _),
        _ => None,
    }
}

/// Plain handle value that can move into a foreground task.
pub fn native_hwnd(window: &gpui_kit::Window) -> Option<isize> {
    native_handle(window).map(|hwnd| hwnd as isize)
}

// The calls below resize the native window. GPUI misses the resulting `WM_SIZE` when it
// arrives inside a window update, so they must run from a foreground task, never inline.

/// Sticky-note chrome: no title bar and no maximize box, but resize frame, shadow, system
/// menu and taskbar minimize stay. The client area becomes `w` x `h` logical pixels.
pub fn enter_sticky(hwnd: isize, w: f32, h: f32, scale: f32) {
    let hwnd = hwnd as windows_sys::Win32::Foundation::HWND;
    set_frame_bars(hwnd, false);
    resize_client(hwnd, w, h, scale);
}

fn set_frame_bars(hwnd: windows_sys::Win32::Foundation::HWND, visible: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_CAPTION, WS_MAXIMIZEBOX,
    };
    const BARS: isize = (WS_CAPTION | WS_MAXIMIZEBOX) as isize;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let next = if visible { style | BARS } else { style & !BARS };
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

/// Sizes the window so its client area is `w` x `h` logical pixels under the current frame.
fn resize_client(hwnd: windows_sys::Win32::Foundation::HWND, w: f32, h: f32, scale: f32) {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{
            AdjustWindowRectEx, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
        },
    };
    unsafe {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: (w * scale).round() as i32,
            bottom: (h * scale).round() as i32,
        };
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        AdjustWindowRectEx(&mut rect, style, 0, ex_style);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

pub fn pin_window(window: &gpui_kit::Window, pinned: bool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };
    native_handle(window).is_some_and(|hwnd| unsafe {
        SetWindowPos(
            hwnd,
            if pinned { HWND_TOPMOST } else { HWND_NOTOPMOST },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        ) != 0
    })
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const APP_ID: &str = "NoteWarden.App";
const WINDOW_TITLE: &str = "Note Warden";

/// Ereignisse aus Tray, Hotkey, Benachrichtigungen und zweitem Programmstart.
pub enum Event {
    ShowWindow,
    QuickCapture,
    Compact,
    Today,
    Quit,
    UpdaterChanged,
    Reminder {
        target: Option<Target>,
        action: Option<String>,
    },
}

static SENDER: OnceLock<UnboundedSender<Event>> = OnceLock::new();

pub fn init_events() -> UnboundedReceiver<Event> {
    let (sender, receiver) = unbounded();
    let _ = SENDER.set(sender);
    receiver
}

pub fn send(event: Event) {
    if let Some(sender) = SENDER.get() {
        let _ = sender.unbounded_send(event);
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Öffnet Datei oder Ordner mit der Standard-App.
pub fn open_path(path: &Path) -> bool {
    Command::new("explorer")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
}

/// Zeigt eine Datei markiert im Explorer.
pub fn reveal(path: &Path) -> bool {
    Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .is_ok()
}

// ---------- Einzelinstanz und Hauptfenster ----------

/// `false`, wenn Note Warden schon läuft; dann wurde dessen Fenster nach vorn geholt.
pub fn claim_single_instance() -> bool {
    let suffix = std::env::var_os("NOTE_WARDEN_DATA_DIR")
        .map(|path| {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hash);
            format!(".{:x}", hash.finish())
        })
        .unwrap_or_default();
    let name = wide(&format!("Local\\NoteWarden.SingleInstance{suffix}"));
    // Das Handle bleibt bis zum Prozessende offen.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    let exists = !handle.is_null() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if exists {
        show_main_window();
    }
    !exists
}

fn main_window() -> Option<windows_sys::Win32::Foundation::HWND> {
    let title = wide(WINDOW_TITLE);
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    (!hwnd.is_null()).then_some(hwnd)
}

pub fn show_main_window() {
    if let Some(hwnd) = main_window() {
        unsafe {
            ShowWindow(
                hwnd,
                if IsIconic(hwnd) != 0 {
                    SW_RESTORE
                } else {
                    SW_SHOW
                },
            );
            SetForegroundWindow(hwnd);
        }
    }
}

pub fn hide_main_window() {
    if let Some(hwnd) = main_window() {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
}

// ---------- Tray ----------

thread_local! {
    static TRAY: RefCell<Option<tray_icon::TrayIcon>> = const { RefCell::new(None) };
    static HOTKEYS: RefCell<Option<(global_hotkey::GlobalHotKeyManager, Option<global_hotkey::hotkey::HotKey>)>> = const { RefCell::new(None) };
}

/// Legt das Tray-Symbol an (Hauptthread). Menü: Öffnen, Neues Update, Heute, Beenden.
pub fn create_tray() {
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    };
    let open = MenuItem::with_id("open", "Öffnen", true, None);
    let capture = MenuItem::with_id("capture", "Schnell erfassen", true, None);
    let compact = MenuItem::with_id("compact", "Notizzettel", true, None);
    let today = MenuItem::with_id("today", "Heute", true, None);
    let quit = MenuItem::with_id("quit", "Beenden", true, None);
    let menu = Menu::new();
    if menu
        .append_items(&[
            &open,
            &capture,
            &compact,
            &today,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .is_err()
    {
        return;
    }
    MenuEvent::set_event_handler(Some(|event: MenuEvent| match event.id.0.as_str() {
        "open" => send(Event::ShowWindow),
        "capture" => send(Event::QuickCapture),
        "compact" => send(Event::Compact),
        "today" => send(Event::Today),
        "quit" => send(Event::Quit),
        _ => {}
    }));
    TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            send(Event::ShowWindow);
        }
    }));
    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("Note Warden");
    if let Ok(icon) = Icon::from_resource(1, None) {
        builder = builder.with_icon(icon);
    }
    match builder.build() {
        Ok(tray) => TRAY.with(|slot| *slot.borrow_mut() = Some(tray)),
        Err(error) => crate::log::error(&format!("Tray nicht verfügbar: {error}")),
    }
}

// ---------- Globaler Hotkey ----------

pub enum HotkeyError {
    Invalid,
    Taken,
}

/// "Strg+Alt+N" -> Hotkey. Erlaubt Strg/Alt/Umschalt/Win plus Buchstabe, Ziffer oder F-Taste.
fn parse_hotkey(text: &str) -> Option<global_hotkey::hotkey::HotKey> {
    let english: Vec<String> = text
        .split('+')
        .map(|part| match part.trim().to_lowercase().as_str() {
            "strg" | "ctrl" => "control".to_owned(),
            "umschalt" | "shift" => "shift".to_owned(),
            "alt" => "alt".to_owned(),
            "win" | "super" => "super".to_owned(),
            other => other.to_owned(),
        })
        .collect();
    english.join("+").parse().ok()
}

/// Registriert den Hotkey neu; bei Fehler bleibt der bisherige aktiv.
pub fn set_hotkey(text: &str) -> Result<(), HotkeyError> {
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
    let hotkey = parse_hotkey(text).ok_or(HotkeyError::Invalid)?;
    HOTKEYS.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            let manager = GlobalHotKeyManager::new().map_err(|_| HotkeyError::Taken)?;
            GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
                if event.state == HotKeyState::Pressed {
                    send(Event::QuickCapture);
                }
            }));
            *slot = Some((manager, None));
        }
        let (manager, current) = slot.as_mut().expect("Hotkey-Verwaltung");
        if *current == Some(hotkey) {
            return Ok(());
        }
        manager.register(hotkey).map_err(|_| HotkeyError::Taken)?;
        if let Some(old) = current.replace(hotkey) {
            let _ = manager.unregister(old);
        }
        Ok(())
    })
}

// ---------- Autostart ----------

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub fn autostart_enabled() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|key| key.get_value::<String, _>("NoteWarden"))
        .is_ok()
}

/// Beim Windows-Start minimiert im Tray starten.
pub fn set_autostart(on: bool) -> std::io::Result<()> {
    let (key, _) =
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    if on {
        let exe = std::env::current_exe()?;
        key.set_value("NoteWarden", &format!("\"{}\" --minimized", exe.display()))
    } else {
        match key.delete_value("NoteWarden") {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

// ---------- Benachrichtigungen ----------

/// Meldet die App bei Windows an (Name und Symbol in Benachrichtigungen statt "PowerShell").
pub fn register_app_id(data_dir: &Path) {
    unsafe { SetCurrentProcessExplicitAppUserModelID(wide(APP_ID).as_ptr()) };
    let icon = data_dir.join("app.ico");
    if !icon.exists() {
        let _ = std::fs::create_dir_all(data_dir);
        let _ = std::fs::write(&icon, include_bytes!("../branding/app.ico"));
    }
    let result = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .create_subkey(format!("Software\\Classes\\AppUserModelId\\{APP_ID}"))
        .and_then(|(key, _)| {
            key.set_value("DisplayName", &WINDOW_TITLE)?;
            key.set_value("IconUri", &icon.display().to_string())
        });
    if let Err(error) = result {
        crate::log::error(&format!("App-ID nicht registriert: {error}"));
    }
}

/// Windows-Benachrichtigung; Klick und Schaltflächen kommen als `Event::Reminder` zurück.
pub fn notify(title: &str, body: &str, target: Option<Target>) -> bool {
    use tauri_winrt_notification::{Duration, Toast};
    let mut toast = Toast::new(APP_ID)
        .title(title)
        .text1(body)
        .duration(Duration::Long);
    if let Some(target) = target {
        toast = toast
            .add_button("10 Min", "snooze10")
            .add_button("1 Std", "snooze60")
            .add_button("Morgen 9:00", "tomorrow");
        if !matches!(target, Target::Update(_)) {
            toast = toast.add_button("Erledigt", "done");
        }
    }
    toast
        .on_activated(move |action| {
            send(Event::Reminder { target, action });
            Ok(())
        })
        .show()
        .is_ok()
}
