//! Windows-Integration: Ereigniskanal, Einzelinstanz, Hauptfenster, Tray, globaler Hotkey,
//! Autostart und Benachrichtigungen. Alles lokal, keine Netzwerkzugriffe.
use crate::reminders::Target;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
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

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const APP_ID: &str = "NoteWarden.App";
const WINDOW_TITLE: &str = "Note Warden";

/// Ereignisse aus Tray, Hotkey, Benachrichtigungen und zweitem Programmstart.
pub enum Event {
    ShowWindow,
    QuickCapture,
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
    let name = wide("Local\\NoteWarden.SingleInstance");
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
    let capture = MenuItem::with_id("capture", "Neues Update", true, None);
    let today = MenuItem::with_id("today", "Heute", true, None);
    let quit = MenuItem::with_id("quit", "Beenden", true, None);
    let menu = Menu::new();
    if menu
        .append_items(&[
            &open,
            &capture,
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
