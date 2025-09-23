#![windows_subsystem = "windows"]

mod constants;
mod engine;
mod solver;

use std::{mem::size_of, time::Instant};

use crate::engine::{Card, DrawMode, GameState, Rank, StockAction};

use windows::core::{w, PCWSTR};

use windows::Win32::Foundation::{BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};

use windows::Win32::Graphics::Gdi::{
    AlphaBlend, BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject,
    InvalidateRect, RoundRect, SelectObject, SetBkMode, SetTextColor, AC_SRC_ALPHA, AC_SRC_OVER,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE,
    DT_TOP, DT_VCENTER, HBITMAP, HBRUSH, HDC, HGDIOBJ, HOLLOW_BRUSH, HPEN, PAINTSTRUCT, PS_SOLID,
    SRCCOPY, TRANSPARENT,
};

use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICFormatConverter,
    IWICImagingFactory, IWICStream, WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom,
    WICDecodeOptions,
};

use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;

use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};

use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, REG_BINARY,
    REG_OPTION_NON_VOLATILE,
};

use windows::Win32::UI::Controls::{
    CreateStatusWindowW, InitCommonControlsEx, ICC_BAR_CLASSES, INITCOMMONCONTROLSEX,
    SBARS_SIZEGRIP, SB_SETTEXTW,
};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_DOWN, VK_LEFT, VK_RIGHT, VK_SPACE, VK_UP,
};

use windows::Win32::UI::WindowsAndMessaging::{
    CheckMenuItem, CreateWindowExW, DefWindowProcW, DestroyWindow, DialogBoxParamW,
    DispatchMessageW, EndDialog, GetClientRect, GetMenu, GetMessageW, GetWindowLongPtrW,
    GetWindowPlacement, KillTimer, LoadAcceleratorsW, LoadCursorW, LoadIconW, LoadMenuW,
    PostQuitMessage, RegisterClassExW, SendMessageW, SetTimer, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, SystemParametersInfoW, TranslateAcceleratorW, TranslateMessage, CS_DBLCLKS,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HACCEL, HCURSOR, HICON, HMENU, HWND_TOP,
    IDCANCEL, IDC_ARROW, IDI_APPLICATION, IDOK, MF_BYCOMMAND, MF_CHECKED, MF_UNCHECKED, MSG,
    SPI_GETWORKAREA, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOWMAXIMIZED, SW_SHOWNORMAL,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOWPLACEMENT, WINDOW_EX_STYLE, WM_COMMAND, WM_CREATE,
    WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_INITDIALOG,
    WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SIZE,
    WM_TIMER, WNDCLASSEXW, WNDCLASS_STYLES, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

const APP_TITLE: PCWSTR = w!("Solitaire");
const CLASS_NAME: PCWSTR = w!("SolitaireWindowClass");

const WINDOW_BOUNDS_VALUE: &str = "WindowBounds";
const WINDOW_MIN_WIDTH: i32 = 640;
const WINDOW_MIN_HEIGHT: i32 = 480;
#[inline]
const fn make_int_resource(id: u16) -> PCWSTR {
    // Equivalent to MAKEINTRESOURCEW; used to avoid import issues.
    PCWSTR(id as usize as *const u16)
}

fn to_wide(message: &str) -> Vec<u16> {
    message.encode_utf16().chain(std::iter::once(0)).collect()
}

fn loword(value: WPARAM) -> u16 {
    (value.0 & 0xFFFF) as u16
}

fn debug_log(message: &str) {
    let wide = to_wide(message);
    unsafe {
        OutputDebugStringW(PCWSTR(wide.as_ptr()));
    }
}

fn lparam_point(lparam: LPARAM) -> (i32, i32) {
    let raw = lparam.0 as u32;
    let x = (raw & 0xFFFF) as i16 as i32;
    let y = (raw >> 16) as i16 as i32;
    (x, y)
}

#[inline]
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(((b as u32) << 16) | ((g as u32) << 8) | (r as u32))
}

const CARD_SPRITE_COLS: i32 = 13;
const CARD_SPRITE_ROWS: i32 = 4;
const DEFAULT_CARD_WIDTH: i32 = 120;
const DEFAULT_CARD_HEIGHT: i32 = 168;
const MAX_TABLEAU_DRAW_CARDS: i32 = 19;
const FOUNDATION_COLUMNS: usize = 4;
const TABLEAU_COLUMNS: usize = 7;
const DRAG_THRESHOLD: i32 = 4;
const VICTORY_TIMER_ID: usize = 1;
const ANIM_EMIT_INTERVAL: f32 = 0.16;
const ANIM_FIXED_DT: f32 = 0.02;
const ANIM_GRAVITY: f32 = 3000.0;
const ANIM_FLOOR_DAMPING: f32 = 0.78;
const ANIM_WALL_DAMPING: f32 = 0.82;
const ANIM_POINTER_SCALE: f32 = 0.0015;
const ANIM_MAX_POINTER_SCALE: f32 = 3.5;
const ANIM_MAX_POINTER_SPEED: f32 = 4000.0;
const ANIM_EXIT_BOUNCES: u32 = 8;
const ANIM_MAX_DELTA: f32 = 0.1;

const RANK_EMIT_ORDER: [Rank; 13] = [
    Rank::King,
    Rank::Queen,
    Rank::Jack,
    Rank::Ten,
    Rank::Nine,
    Rank::Eight,
    Rank::Seven,
    Rank::Six,
    Rank::Five,
    Rank::Four,
    Rank::Three,
    Rank::Two,
    Rank::Ace,
];
struct ComApartment;

impl ComApartment {
    unsafe fn new() -> anyhow::Result<Self> {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn load_window_bounds() -> Option<(RECT, bool)> {
    unsafe {
        let subkey = to_wide(constants::REGISTRY_BASE_KEY);
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return None;
        }

        let mut data = [0i32; 5];
        let mut data_size = (data.len() * size_of::<i32>()) as u32;
        let mut value_type = REG_BINARY;
        let value_name = to_wide(WINDOW_BOUNDS_VALUE);
        let status = RegQueryValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(data.as_mut_ptr() as *mut u8),
            Some(&mut data_size),
        );
        let _ = RegCloseKey(hkey);

        if status.is_err() || value_type != REG_BINARY || data_size < (4 * size_of::<i32>()) as u32
        {
            return None;
        }

        let left = data[0];
        let top = data[1];
        let width = data[2];
        let height = data[3];
        if width <= 0 || height <= 0 {
            return None;
        }

        let rect = RECT {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        let maximized = data.get(4).copied().unwrap_or(0) != 0;
        Some((rect, maximized))
    }
}

fn save_window_bounds(hwnd: HWND) {
    unsafe {
        let mut placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        if GetWindowPlacement(hwnd, &mut placement).is_err() {
            return;
        }

        let rect = placement.rcNormalPosition;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return;
        }

        let data = [
            rect.left,
            rect.top,
            width,
            height,
            if placement.showCmd == SW_SHOWMAXIMIZED.0 as u32 {
                1
            } else {
                0
            },
        ];

        let subkey = to_wide(constants::REGISTRY_BASE_KEY);
        let mut hkey = HKEY::default();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE | KEY_QUERY_VALUE,
            None,
            &mut hkey,
            None,
        )
        .is_err()
        {
            return;
        }

        let value_name = to_wide(WINDOW_BOUNDS_VALUE);
        let bytes =
            std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<i32>());
        let _ = RegSetValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            0,
            REG_BINARY,
            Some(bytes),
        );
        let _ = RegCloseKey(hkey);
    }
}

fn apply_saved_window_bounds(hwnd: HWND) {
    if let Some((mut rect, maximized)) = load_window_bounds() {
        let mut width = (rect.right - rect.left).max(WINDOW_MIN_WIDTH);
        let mut height = (rect.bottom - rect.top).max(WINDOW_MIN_HEIGHT);
        clamp_rect_to_work_area(&mut rect, &mut width, &mut height);

        unsafe {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                rect.left,
                rect.top,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            ShowWindow(
                hwnd,
                if maximized {
                    SW_SHOWMAXIMIZED
                } else {
                    SW_SHOWNORMAL
                },
            );
        }
    }
}

fn clamp_rect_to_work_area(rect: &mut RECT, width: &mut i32, height: &mut i32) {
    unsafe {
        let mut work = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            let work_width = work.right - work.left;
            if work_width > 0 {
                let min_width = WINDOW_MIN_WIDTH.min(work_width);
                *width = (*width).clamp(min_width, work_width);
            }

            let work_height = work.bottom - work.top;
            if work_height > 0 {
                let min_height = WINDOW_MIN_HEIGHT.min(work_height);
                *height = (*height).clamp(min_height, work_height);
            }

            let max_left = work.right - *width;
            let max_top = work.bottom - *height;
            rect.left = rect.left.clamp(work.left, max_left.max(work.left));
            rect.top = rect.top.clamp(work.top, max_top.max(work.top));
        }
    }

    rect.right = rect.left + *width;
    rect.bottom = rect.top + *height;
}

unsafe fn update_draw_menu(hwnd: HWND, draw_mode: DrawMode) {
    let menu = GetMenu(hwnd);
    if menu.0 != 0 {
        let draw1_flags = MF_BYCOMMAND.0
            | if matches!(draw_mode, DrawMode::DrawOne) {
                MF_CHECKED.0
            } else {
                MF_UNCHECKED.0
            };
        let draw3_flags = MF_BYCOMMAND.0
            | if matches!(draw_mode, DrawMode::DrawThree) {
                MF_CHECKED.0
            } else {
                MF_UNCHECKED.0
            };
        let _ = CheckMenuItem(menu, constants::IDM_GAME_DRAW1 as u32, draw1_flags);
        let _ = CheckMenuItem(menu, constants::IDM_GAME_DRAW3 as u32, draw3_flags);
    }
}

fn update_status_bar(state: &mut WindowState) {
    if state.status.0 == 0 {
        return;
    }

    let draw_label = match state.game.draw_mode {
        DrawMode::DrawOne => "Draw 1",
        DrawMode::DrawThree => "Draw 3",
    };

    let text = format!(
        "{}   Stock: {}   Waste: {}   Score: {}   Moves: {}",
        draw_label,
        state.game.stock_count(),
        state.game.waste_count(),
        state.game.score,
        state.game.moves
    );

    let wide = to_wide(&text);
    unsafe {
        SendMessageW(
            state.status,
            SB_SETTEXTW,
            WPARAM(0),
            LPARAM(wide.as_ptr() as isize),
        );
    }
}

fn request_redraw(hwnd: HWND) {
    unsafe {
        let _ = InvalidateRect(hwnd, None, BOOL(0));
    }
}

#[derive(Default)]
struct WindowState {
    status: HWND,
    bg_brush: HBRUSH,
    back: Option<BackBuffer>,
    card: Option<CardImage>,
    card_dc: HDC,
    card_old: HGDIOBJ,
    game: GameState,
    layout_metrics: Option<CardMetrics>,
    client_size: (i32, i32),
    tableau_slots: [Vec<CardSlot>; TABLEAU_COLUMNS],
    drag: Option<DragContext>,
    mouse_down: Option<MouseDownContext>,
    pending_selection: Option<Selection>,
    focus: Option<HitTarget>,
    win_anim: Option<VictoryAnimation>,
    victory_timer_active: bool,
    undo_stack: Vec<GameState>,
    redo_stack: Vec<GameState>,
    pointer_pos: (i32, i32),
    pointer_speed: f32,
    pointer_last: Option<Instant>,
}

impl WindowState {
    fn push_undo(&mut self, snapshot: GameState) {
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
    }

    fn clear_transients(&mut self) {
        self.drag = None;
        self.mouse_down = None;
        self.pending_selection = None;
        self.layout_metrics = None;
        self.focus = Some(HitTarget::Stock);
    }
}

unsafe fn set_state(hwnd: HWND, state: Box<WindowState>) {
    let ptr = Box::into_raw(state) as isize;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr);
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut WindowState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

unsafe fn clear_state(hwnd: HWND) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                // Allocate per-window state
                let mut state = Box::new(WindowState {
                    status: HWND(0),
                    bg_brush: HBRUSH(0),
                    back: None,
                    card: None,
                    card_dc: HDC(0),
                    card_old: HGDIOBJ(0),
                    game: GameState::default(),
                    layout_metrics: None,
                    client_size: (0, 0),
                    tableau_slots: Default::default(),
                    drag: None,
                    mouse_down: None,
                    pending_selection: None,
                    focus: Some(HitTarget::Stock),
                    win_anim: None,
                    victory_timer_active: false,
                    undo_stack: Vec::new(),
                    redo_stack: Vec::new(),
                    pointer_pos: (0, 0),
                    pointer_speed: 0.0,
                    pointer_last: None,
                });

                // Create background brush (green felt)
                state.bg_brush = CreateSolidBrush(rgb(0, 128, 0));

                // Init common controls and create status bar
                let icc = INITCOMMONCONTROLSEX {
                    dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
                    dwICC: ICC_BAR_CLASSES,
                };
                InitCommonControlsEx(&icc);
                let style = (WS_CHILD.0 | WS_VISIBLE.0 | SBARS_SIZEGRIP) as i32;
                state.status = CreateStatusWindowW(style, w!(""), hwnd, constants::STATUS_BAR_ID);

                if let Err(err) = state.game.deal_new_game(DrawMode::DrawOne) {
                    debug_log(&format!("deal_new_game failed: {err:?}"));
                }

                update_draw_menu(hwnd, state.game.draw_mode);
                update_status_bar(&mut state);

                // Try to load embedded card PNG (optional)
                match load_card_bitmap_from_resource(constants::IDB_CARDS) {
                    Ok(Some(card)) => {
                        state.card_dc = CreateCompatibleDC(HDC(0));
                        state.card_old = SelectObject(state.card_dc, card.hbm);
                        state.card = Some(card);
                    }
                    Ok(None) => {
                        OutputDebugStringW(w!("No cards resource found; using placeholder."));
                    }
                    Err(_e) => {
                        OutputDebugStringW(w!("Failed to load cards resource."));
                    }
                }

                set_state(hwnd, state);
                LRESULT(0)
            }
            WM_SIZE => {
                if let Some(state) = get_state(hwnd) {
                    // Let the status bar auto-size itself and resize backbuffer
                    SendMessageW(state.status, msg, wparam, lparam);
                    ensure_backbuffer(hwnd, state, 0, 0);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                if wparam.0 == VICTORY_TIMER_ID {
                    if let Some(state) = get_state(hwnd) {
                        update_victory_animation(hwnd, state);
                        request_redraw(hwnd);
                    }
                    LRESULT(0)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
            WM_LBUTTONDOWN => {
                if let Some(state) = get_state(hwnd) {
                    let position = lparam_point(lparam);
                    let target = hit_test(&*state, position.0, position.1);
                    state.mouse_down = Some(MouseDownContext { target, position });
                    set_focus(state, target);
                }
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if let Some(state) = get_state(hwnd) {
                    let (mx, my) = lparam_point(lparam);
                    if state.drag.is_some() {
                        let hover = hit_test(&*state, mx, my);
                        if let Some(drag) = state.drag.as_mut() {
                            drag.position = (mx - drag.hotspot.0, my - drag.hotspot.1);
                            drag.hover = hover;
                        }
                        request_redraw(hwnd);
                    } else if let Some(mouse) = state.mouse_down {
                        let dx = (mx - mouse.position.0).abs();
                        let dy = (my - mouse.position.1).abs();
                        if dx.max(dy) >= DRAG_THRESHOLD
                            && begin_drag(hwnd, state, mouse.target, (mx, my))
                        {
                            state.mouse_down = None;
                            request_redraw(hwnd);
                        }
                    }
                    let now = Instant::now();
                    if let Some(last) = state.pointer_last {
                        let dt = (now - last).as_secs_f32();
                        if dt > 0.0 {
                            let dx = (mx - state.pointer_pos.0) as f32;
                            let dy = (my - state.pointer_pos.1) as f32;
                            let distance = (dx * dx + dy * dy).sqrt();
                            let speed = (distance / dt).min(ANIM_MAX_POINTER_SPEED);
                            state.pointer_speed = state.pointer_speed * 0.8 + speed * 0.2;
                        }
                    }
                    state.pointer_pos = (mx, my);
                    state.pointer_last = Some(now);
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if let Some(state) = get_state(hwnd) {
                    let (mx, my) = lparam_point(lparam);
                    if let Some(drag) = state.drag.take() {
                        let _ = ReleaseCapture();
                        let drop_target = match hit_test(&*state, mx, my) {
                            HitTarget::Stock => HitTarget::None,
                            other => other,
                        };
                        let snapshot = drag.snapshot.clone();
                        if finalize_drag(state, drag, drop_target) {
                            state.push_undo(snapshot);
                            update_status_bar(state);
                            check_for_victory(hwnd, state);
                        }
                        request_redraw(hwnd);
                    } else if let Some(mouse) = state.mouse_down.take() {
                        let release_target = hit_test(&*state, mx, my);
                        if release_target == mouse.target {
                            handle_click(hwnd, state, release_target);
                        } else {
                            state.pending_selection = None;
                        }
                    }
                    state.mouse_down = None;
                }
                LRESULT(0)
            }
            WM_LBUTTONDBLCLK => {
                if let Some(state) = get_state(hwnd) {
                    state.mouse_down = None;
                    state.pending_selection = None;
                    if let Some(drag) = state.drag.take() {
                        match drag.source {
                            DragSource::Waste => state.game.waste.cards.extend(drag.cards),
                            DragSource::Tableau { column } => {
                                state.game.cancel_tableau_stack(column, drag.cards);
                            }
                        }
                        let _ = ReleaseCapture();
                    }
                    let (mx, my) = lparam_point(lparam);
                    let target = hit_test(&*state, mx, my);
                    let mut moved = false;
                    let mut snapshot: Option<GameState> = None;
                    match target {
                        HitTarget::Waste => {
                            let snap = state.game.clone();
                            if state.game.move_waste_to_any_foundation() {
                                snapshot = Some(snap);
                                moved = true;
                            }
                        }
                        HitTarget::Tableau {
                            column,
                            card_index: Some(idx),
                        } if idx + 1 == state.game.tableau_len(column) => {
                            let snap = state.game.clone();
                            if state.game.move_tableau_top_to_any_foundation(column) {
                                snapshot = Some(snap);
                                moved = true;
                            }
                        }
                        _ => {}
                    }
                    if moved {
                        if let Some(snap) = snapshot {
                            state.push_undo(snap);
                        }
                        update_status_bar(state);
                        check_for_victory(hwnd, state);
                    }
                    request_redraw(hwnd);
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                if let Some(state) = get_state(hwnd) {
                    if handle_key_down(hwnd, state, wparam.0 as u32) {
                        return LRESULT(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as u16;
                if id == constants::IDM_FILE_EXIT {
                    let _ = DestroyWindow(hwnd);
                    return LRESULT(0);
                }
                match id {
                    constants::IDM_FILE_NEW => {
                        if let Some(state) = get_state(hwnd) {
                            stop_victory_animation(hwnd, state);
                            let snapshot = state.game.clone();
                            let draw_mode = state.game.draw_mode;
                            match state.game.deal_new_game(draw_mode) {
                                Ok(()) => {
                                    state.push_undo(snapshot);
                                    state.clear_transients();
                                    state.layout_metrics = None;
                                    update_status_bar(state);
                                }
                                Err(err) => {
                                    debug_log(&format!("deal_new_game failed: {err:?}"));
                                }
                            }
                        }
                        request_redraw(hwnd);
                    }
                    constants::IDM_FILE_DEALAGAIN => {
                        if let Some(state) = get_state(hwnd) {
                            stop_victory_animation(hwnd, state);
                            let snapshot = state.game.clone();
                            match state.game.deal_again() {
                                Ok(()) => {
                                    state.push_undo(snapshot);
                                    state.clear_transients();
                                    state.layout_metrics = None;
                                    update_status_bar(state);
                                }
                                Err(err) => {
                                    debug_log(&format!("deal_again failed: {err:?}"));
                                }
                            }
                        }
                        request_redraw(hwnd);
                    }
                    constants::IDM_GAME_DRAW1 => {
                        if let Some(state) = get_state(hwnd) {
                            if state.game.draw_mode != DrawMode::DrawOne {
                                state.game.draw_mode = DrawMode::DrawOne;
                                state.pending_selection = None;
                                update_draw_menu(hwnd, DrawMode::DrawOne);
                                update_status_bar(state);
                            }
                        }
                    }
                    constants::IDM_GAME_VICTORY => {
                        if let Some(state) = get_state(hwnd) {
                            stop_victory_animation(hwnd, state);
                            let mut snapshot: Option<GameState> = None;
                            if !state.game.is_won() {
                                let snap = state.game.clone();
                                if state.game.force_complete_foundations() {
                                    snapshot = Some(snap);
                                    state.drag = None;
                                    state.mouse_down = None;
                                    state.pending_selection = None;
                                    set_focus(state, HitTarget::Foundation(0));
                                    update_status_bar(state);
                                }
                            }
                            let _ = force_victory_animation(hwnd, state);
                            if let Some(snap) = snapshot {
                                state.push_undo(snap);
                            }
                            request_redraw(hwnd);
                        }
                    }
                    constants::IDM_EDIT_UNDO => {
                        if let Some(state) = get_state(hwnd) {
                            stop_victory_animation(hwnd, state);
                            if let Some(snapshot) = state.undo_stack.pop() {
                                let current = state.game.clone();
                                state.redo_stack.push(current);
                                state.game = snapshot;
                                state.clear_transients();
                                update_status_bar(state);
                                update_draw_menu(hwnd, state.game.draw_mode);
                                check_for_victory(hwnd, state);
                                request_redraw(hwnd);
                            }
                        }
                    }
                    constants::IDM_EDIT_REDO => {
                        if let Some(state) = get_state(hwnd) {
                            stop_victory_animation(hwnd, state);
                            if let Some(snapshot) = state.redo_stack.pop() {
                                let current = state.game.clone();
                                state.undo_stack.push(current);
                                state.game = snapshot;
                                state.clear_transients();
                                update_status_bar(state);
                                update_draw_menu(hwnd, state.game.draw_mode);
                                check_for_victory(hwnd, state);
                                request_redraw(hwnd);
                            }
                        }
                    }
                    constants::IDM_HELP_ABOUT => {
                        show_about_dialog(hwnd);
                    }
                    _ => {}
                }

                LRESULT(0)
            }
            WM_ERASEBKGND => {
                // Avoid flicker; we paint in WM_PAINT
                LRESULT(1)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if let Some(state) = get_state(hwnd) {
                    paint_window(hwnd, hdc, state);
                }
                EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_DESTROY => {
                save_window_bounds(hwnd);
                if let Some(state) = get_state(hwnd) {
                    stop_victory_animation(hwnd, state);
                    if state.bg_brush.0 != 0 {
                        let _ = DeleteObject(state.bg_brush);
                    }
                    if let Some(mut back) = state.back.take() {
                        back.destroy();
                    }
                    if state.card_dc.0 != 0 {
                        if state.card_old.0 != 0 {
                            let _ = SelectObject(state.card_dc, state.card_old);
                        }
                        DeleteDC(state.card_dc);
                    }
                    if let Some(card) = state.card.take() {
                        if card.hbm.0 != 0 {
                            let _ = DeleteObject(card.hbm);
                        }
                    }
                }
                clear_state(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn main() -> anyhow::Result<()> {
    unsafe {
        let _com = ComApartment::new()?;

        let hmodule = GetModuleHandleW(None)?;
        let hinstance = HINSTANCE(hmodule.0);

        // Register window class
        let class_name = CLASS_NAME;

        // Try to load custom icon (not yet embedded); fallback to default
        let h_icon: HICON = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        let h_cursor: HCURSOR = LoadCursorW(None, IDC_ARROW).unwrap_or_default();

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(CS_HREDRAW.0 | CS_VREDRAW.0 | CS_DBLCLKS.0),
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            hIcon: h_icon,
            hCursor: h_cursor,
            hbrBackground: HBRUSH(0), // no background; we paint manually
            lpszClassName: class_name,
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(anyhow::anyhow!("RegisterClassExW failed"));
        }

        // Load menu from resources
        let hmenu: HMENU =
            LoadMenuW(hinstance, make_int_resource(constants::IDR_MAINMENU)).unwrap_or_default();

        // Create the main window
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            APP_TITLE,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1024,
            768,
            None,
            hmenu,
            hinstance,
            None,
        );
        if hwnd.0 == 0 {
            return Err(anyhow::anyhow!("CreateWindowExW failed"));
        }

        apply_saved_window_bounds(hwnd);

        // Load accelerators
        let haccel: HACCEL = LoadAcceleratorsW(hinstance, make_int_resource(constants::IDR_ACCEL))
            .unwrap_or_default();

        // Standard message loop with accelerator translation
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, HWND(0), 0, 0).0;
            if ret == -1 {
                break; // error
            }
            if ret == 0 {
                break; // WM_QUIT
            }

            if !haccel.is_invalid() && TranslateAcceleratorW(hwnd, haccel, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        CoUninitialize();
    }
    Ok(())
}

// ------------ Back buffer ------------
struct BackBuffer {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    w: i32,
    h: i32,
}

impl BackBuffer {
    unsafe fn new(width: i32, height: i32) -> anyhow::Result<Self> {
        let dc = CreateCompatibleDC(HDC(0));

        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
        let bmp = CreateDIBSection(HDC(0), &bi, DIB_RGB_COLORS, &mut bits, None, 0)?;
        if bmp.is_invalid() {
            return Err(anyhow::anyhow!("CreateDIBSection failed"));
        }
        let old = SelectObject(dc, bmp);

        Ok(Self {
            dc,
            bmp,
            old,
            w: width,
            h: height,
        })
    }

    unsafe fn destroy(&mut self) {
        if self.dc.0 != 0 {
            if self.old.0 != 0 {
                let _ = SelectObject(self.dc, self.old);
            }
            let _ = DeleteObject(self.bmp);
            let _ = DeleteDC(self.dc);
            self.dc = HDC(0);
        }
    }
}

unsafe fn ensure_backbuffer(hwnd: HWND, state: &mut WindowState, _w: i32, _h: i32) {
    let mut client = RECT::default();
    let _ = GetClientRect(hwnd, &mut client);
    let mut height = client.bottom - client.top;
    // Status bar will overlay; no special handling needed now
    let width = client.right - client.left;
    height = height.max(1);
    let width = width.max(1);

    state.client_size = (width, height);

    let recreate = match &state.back {
        Some(b) => b.w != width || b.h != height,
        None => true,
    };
    if recreate {
        if let Some(mut old) = state.back.take() {
            old.destroy();
        }
        if let Ok(bb) = BackBuffer::new(width, height) {
            state.back = Some(bb);
        }
    }
}

// ------------ Card image ------------
struct CardImage {
    hbm: HBITMAP,
    cell_w: i32,
    cell_h: i32,
}

#[derive(Clone, Copy)]
struct CardMetrics {
    card_w: i32,
    card_h: i32,
    column_gap: i32,
    row_gap: i32,
    face_down_offset: i32,
    face_up_offset: i32,
    face_inset: i32,
    margin: i32,
}

#[derive(Clone, Copy, Default)]
struct CardSlot {
    top: i32,
    height: i32,
}

impl CardMetrics {
    fn compute(state: &WindowState, width: i32, height: i32) -> Self {
        let card_base_w = state
            .card
            .as_ref()
            .map(|img| img.cell_w)
            .unwrap_or(DEFAULT_CARD_WIDTH);
        let card_base_h = state
            .card
            .as_ref()
            .map(|img| img.cell_h)
            .unwrap_or(DEFAULT_CARD_HEIGHT);

        let margin_base = (card_base_w / 4).max(16);
        let column_gap_base = (card_base_w / 8).max(12);
        let row_gap_base = (card_base_h / 6).max(16);
        let face_down_offset_base = (card_base_h / 6).max(12);
        let face_up_offset_base = (card_base_h / 4).max(20);
        let face_inset_base = (card_base_w / 24).max(4);

        let required_width = margin_base * 2 + card_base_w * 7 + column_gap_base * 6;
        let mut max_tableau_height = card_base_h;
        for pile in &state.game.tableaus {
            if pile.cards.is_empty() {
                max_tableau_height = max_tableau_height.max(card_base_h);
                continue;
            }
            let len = pile.cards.len();
            let visible = len.min(MAX_TABLEAU_DRAW_CARDS as usize);
            let start_index = len - visible;
            let mut height = card_base_h;
            if visible > 1 {
                for card in &pile.cards[start_index..len - 1] {
                    let offset = if card.face_up {
                        face_up_offset_base
                    } else {
                        face_down_offset_base
                    };
                    height += offset;
                }
            }
            max_tableau_height = max_tableau_height.max(height);
        }
        let required_height = margin_base * 2 + card_base_h + row_gap_base + max_tableau_height;

        let width = width.max(1);
        let height = height.max(1);
        let scale_w = width as f32 / required_width as f32;
        let scale_h = height as f32 / required_height as f32;
        let mut scale = scale_w.min(scale_h);
        scale = scale.clamp(0.35, 4.0);

        let scale_i32 = |value: i32, minimum: i32| -> i32 {
            ((value as f32 * scale).round() as i32).max(minimum)
        };

        Self {
            card_w: scale_i32(card_base_w, 8),
            card_h: scale_i32(card_base_h, 12),
            column_gap: scale_i32(column_gap_base, 6),
            row_gap: scale_i32(row_gap_base, 8),
            face_down_offset: scale_i32(face_down_offset_base, 6),
            face_up_offset: scale_i32(face_up_offset_base, 10),
            face_inset: scale_i32(face_inset_base, 2),
            margin: scale_i32(margin_base, 12),
        }
    }

    fn column_x(&self, column: usize) -> i32 {
        self.margin + column as i32 * (self.card_w + self.column_gap)
    }

    fn top_y(&self) -> i32 {
        self.margin
    }

    fn tableau_y(&self) -> i32 {
        self.margin + self.card_h + self.row_gap
    }
}

fn make_rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitTarget {
    Stock,
    Waste,
    Foundation(usize),
    Tableau {
        column: usize,
        card_index: Option<usize>,
    },
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Selection {
    Waste,
    Tableau { column: usize, index: usize },
}

struct DragContext {
    source: DragSource,
    cards: Vec<Card>,
    hotspot: (i32, i32),
    position: (i32, i32),
    hover: HitTarget,
    snapshot: GameState,
}

#[derive(Clone, Copy)]
struct MouseDownContext {
    target: HitTarget,
    position: (i32, i32),
}

#[derive(Clone, Copy)]
enum DragSource {
    Waste,
    Tableau { column: usize },
}

struct AnimCard {
    card: Card,
    start_pos: (f32, f32),
    pos: (f32, f32),
    vel: (f32, f32),
    emitted: bool,
    finished: bool,
    foundation: Option<usize>,
    bounces: u32,
}

struct VictoryAnimation {
    cards: Vec<AnimCard>,
    next_emit: usize,
    emit_timer: f32,
    accumulator: f32,
    last_tick: Instant,
    foundation_emitted: [usize; FOUNDATION_COLUMNS],
}

impl VictoryAnimation {
    fn emitted_from(&self, index: usize) -> usize {
        self.foundation_emitted.get(index).copied().unwrap_or(0)
    }
}

#[derive(Clone)]
struct AnimationSeed {
    card: Card,
    pos: (f32, f32),
    foundation: Option<usize>,
}

fn point_in_rect(x: i32, y: i32, left: i32, top: i32, width: i32, height: i32) -> bool {
    x >= left && x < left + width && y >= top && y < top + height
}

fn stop_victory_animation(hwnd: HWND, state: &mut WindowState) {
    if state.victory_timer_active {
        unsafe {
            let _ = KillTimer(hwnd, VICTORY_TIMER_ID);
        }
        state.victory_timer_active = false;
    }
    state.win_anim = None;
}

fn start_victory_animation_internal(hwnd: HWND, state: &mut WindowState, force: bool) -> bool {
    if state.win_anim.is_some() {
        return false;
    }

    let (width, height) = state.client_size;
    if width <= 0 || height <= 0 {
        return false;
    }

    let metrics = state
        .layout_metrics
        .unwrap_or_else(|| CardMetrics::compute(state, width.max(1), height.max(1)));

    let mut seeds = gather_animation_seeds(state, &metrics);
    if !force {
        seeds.retain(|seed| seed.foundation.is_some());
    }
    if seeds.is_empty() {
        return false;
    }

    let cards = create_victory_cards(seeds);
    if cards.is_empty() {
        return false;
    }

    let now = Instant::now();
    state.win_anim = Some(VictoryAnimation {
        cards,
        next_emit: 0,
        emit_timer: 0.0,
        accumulator: 0.0,
        last_tick: now,
        foundation_emitted: [0; FOUNDATION_COLUMNS],
    });

    unsafe {
        if SetTimer(hwnd, VICTORY_TIMER_ID, 16, None) != 0 {
            state.victory_timer_active = true;
        }
    }
    request_redraw(hwnd);
    true
}

fn start_victory_animation(hwnd: HWND, state: &mut WindowState) -> bool {
    start_victory_animation_internal(hwnd, state, false)
}

fn force_victory_animation(hwnd: HWND, state: &mut WindowState) -> bool {
    start_victory_animation_internal(hwnd, state, true)
}

fn gather_animation_seeds(state: &WindowState, metrics: &CardMetrics) -> Vec<AnimationSeed> {
    let mut seeds = Vec::new();
    let top_y = metrics.top_y() as f32;
    let waste_x = metrics.column_x(1) as f32;

    // Foundations emit from the top-right stacks.
    for (idx, pile) in state.game.foundations.iter().enumerate() {
        let base_x = metrics.column_x(3 + idx) as f32;
        for (offset, card) in pile.cards.iter().enumerate() {
            let mut c = *card;
            c.face_up = true;
            seeds.push(AnimationSeed {
                card: c,
                pos: (base_x, top_y - offset as f32 * 2.0),
                foundation: Some(idx),
            });
        }
    }

    // Waste pile
    for (offset, card) in state.game.waste.cards.iter().enumerate() {
        let mut c = *card;
        c.face_up = true;
        seeds.push(AnimationSeed {
            card: c,
            pos: (waste_x + offset as f32 * 2.0, top_y),
            foundation: None,
        });
    }

    // Stock pile
    let stock_x = metrics.column_x(0) as f32;
    for (offset, card) in state.game.stock.cards.iter().enumerate() {
        let mut c = *card;
        c.face_up = true;
        seeds.push(AnimationSeed {
            card: c,
            pos: (stock_x - offset as f32 * 1.5, top_y),
            foundation: None,
        });
    }

    // Tableau columns
    let tableau_top = metrics.tableau_y() as f32;
    for column in 0..TABLEAU_COLUMNS {
        let x = metrics.column_x(column) as f32;
        let mut y = tableau_top;
        if let Some(pile) = state.game.tableaus.get(column) {
            for card in &pile.cards {
                let mut c = *card;
                c.face_up = true;
                seeds.push(AnimationSeed {
                    card: c,
                    pos: (x, y),
                    foundation: None,
                });
                y += if card.face_up {
                    metrics.face_up_offset as f32
                } else {
                    metrics.face_down_offset as f32
                };
            }
        }
    }

    // Drag stack if present
    if let Some(drag) = state.drag.as_ref() {
        let mut y = drag.position.1 as f32;
        let x = drag.position.0 as f32;
        for card in &drag.cards {
            let mut c = *card;
            c.face_up = true;
            seeds.push(AnimationSeed {
                card: c,
                pos: (x, y),
                foundation: None,
            });
            y += if card.face_up {
                metrics.face_up_offset as f32
            } else {
                metrics.face_down_offset as f32
            };
        }
    }

    seeds
}

fn create_victory_cards(seeds: Vec<AnimationSeed>) -> Vec<AnimCard> {
    let (foundation_seeds, extra): (Vec<_>, Vec<_>) = seeds
        .into_iter()
        .partition(|seed| seed.foundation.is_some());
    let mut ordered: Vec<AnimationSeed> = Vec::with_capacity(foundation_seeds.len() + extra.len());
    let mut used = vec![false; foundation_seeds.len()];

    for rank in RANK_EMIT_ORDER {
        for foundation_idx in 0..FOUNDATION_COLUMNS {
            if let Some(pos) = foundation_seeds.iter().enumerate().find_map(|(idx, seed)| {
                if !used[idx] && seed.foundation == Some(foundation_idx) && seed.card.rank == rank {
                    Some(idx)
                } else {
                    None
                }
            }) {
                ordered.push(foundation_seeds[pos].clone());
                used[pos] = true;
            }
        }
    }

    for (seed, flag) in foundation_seeds.into_iter().zip(used) {
        if !flag {
            ordered.push(seed);
        }
    }

    ordered.extend(extra);

    ordered
        .into_iter()
        .map(|seed| {
            let mut card = seed.card;
            card.face_up = true;
            AnimCard {
                card,
                start_pos: seed.pos,
                pos: seed.pos,
                vel: (0.0, 0.0),
                emitted: false,
                finished: false,
                foundation: seed.foundation,
                bounces: 0,
            }
        })
        .collect()
}

fn update_victory_animation(hwnd: HWND, state: &mut WindowState) {
    let (width, height) = state.client_size;
    let metrics = CardMetrics::compute(state, width.max(1), height.max(1));
    let Some(anim) = state.win_anim.as_mut() else {
        return;
    };
    let now = Instant::now();

    let mut delta = (now - anim.last_tick).as_secs_f32();
    if delta <= 0.0 {
        delta = ANIM_FIXED_DT;
    }
    if delta > ANIM_MAX_DELTA {
        delta = ANIM_MAX_DELTA;
    }
    anim.last_tick = now;

    let speed_scale = 1.0 + (state.pointer_speed * ANIM_POINTER_SCALE).min(ANIM_MAX_POINTER_SCALE);

    anim.emit_timer += delta * speed_scale;
    anim.accumulator += delta * speed_scale;

    let card_w = metrics.card_w as f32;
    let card_h = metrics.card_h as f32;
    let width_f = width.max(1) as f32;
    let height_f = height.max(1) as f32;
    let floor_y = (height_f - card_h).max(0.0);
    while anim.emit_timer >= ANIM_EMIT_INTERVAL && anim.next_emit < anim.cards.len() {
        emit_victory_card(anim, anim.next_emit, speed_scale, card_w, width_f);
        anim.next_emit += 1;
        anim.emit_timer -= ANIM_EMIT_INTERVAL;
    }

    while anim.accumulator >= ANIM_FIXED_DT {
        anim.accumulator -= ANIM_FIXED_DT;
        integrate_victory_cards(
            &mut anim.cards,
            ANIM_FIXED_DT,
            floor_y,
            card_w,
            card_h,
            width_f,
        );
    }

    state.pointer_speed *= 0.9;

    let all_emitted = anim.next_emit >= anim.cards.len();
    if all_emitted
        && anim
            .cards
            .iter()
            .filter(|card| card.emitted)
            .all(|card| card.finished)
    {
        stop_victory_animation(hwnd, state);
        request_redraw(hwnd);
    }
}

fn emit_victory_card(
    anim: &mut VictoryAnimation,
    index: usize,
    speed_scale: f32,
    card_w: f32,
    width: f32,
) {
    if let Some(card) = anim.cards.get_mut(index) {
        card.emitted = true;
        card.finished = false;
        card.bounces = 0;
        card.pos = card.start_pos;
        let span = (width - card_w).max(0.0);
        card.pos.0 = card.pos.0.clamp(0.0, span);
        let foundation = card.foundation.unwrap_or(0);
        let dir = if foundation % 2 == 0 { -1.0 } else { 1.0 };
        let rank_factor = card.card.rank as i32 as f32;
        let base_horizontal = 760.0 + foundation as f32 * 55.0 + rank_factor * 6.0;
        let base_vertical = -1050.0 - foundation as f32 * 40.0 - rank_factor * 4.0;
        card.vel.0 = dir * base_horizontal * speed_scale;
        card.vel.1 = base_vertical * speed_scale;
        card.pos.1 -= 2.0;
        if let Some(foundation_idx) = card.foundation {
            let emitted = &mut anim.foundation_emitted[foundation_idx];
            *emitted = emitted.saturating_add(1);
        }
    }
}

fn integrate_victory_cards(
    cards: &mut [AnimCard],
    dt: f32,
    floor_y: f32,
    card_w: f32,
    card_h: f32,
    width: f32,
) {
    let min_x = 0.0;
    let max_x = (width - card_w).max(0.0);
    for card in cards.iter_mut() {
        if !card.emitted || card.finished {
            continue;
        }

        card.vel.1 += ANIM_GRAVITY * dt;
        card.pos.0 += card.vel.0 * dt;
        card.pos.1 += card.vel.1 * dt;

        if card.pos.0 <= min_x {
            card.pos.0 = min_x;
            card.vel.0 = card.vel.0.abs() * ANIM_WALL_DAMPING;
        } else if card.pos.0 >= max_x {
            card.pos.0 = max_x;
            card.vel.0 = -card.vel.0.abs() * ANIM_WALL_DAMPING;
        }

        if card.pos.1 >= floor_y {
            card.pos.1 = floor_y;
            if card.vel.1 > 0.0 {
                card.vel.1 = -card.vel.1 * ANIM_FLOOR_DAMPING;
                card.bounces = card.bounces.saturating_add(1);
            }
            if card.bounces >= ANIM_EXIT_BOUNCES && card.vel.1.abs() < 120.0 {
                card.finished = true;
            }
        }

        card.vel.0 *= 0.996;

        if card.pos.1 < -card_h * 2.0 {
            card.finished = true;
        }

        if card.pos.0 + card_w < -card_w || card.pos.0 > width + card_w {
            card.finished = true;
        }
    }
}

fn check_for_victory(hwnd: HWND, state: &mut WindowState) {
    if state.win_anim.is_some() {
        return;
    }
    if state.game.is_won() {
        start_victory_animation(hwnd, state);
    }
}
fn hit_test(state: &WindowState, x: i32, y: i32) -> HitTarget {
    let metrics = state.layout_metrics.unwrap_or_else(|| {
        let (w, h) = state.client_size;
        CardMetrics::compute(state, w.max(1), h.max(1))
    });

    let card_w = metrics.card_w;
    let card_h = metrics.card_h;
    let top_y = metrics.top_y();

    let stock_x = metrics.column_x(0);
    let stock_height = card_h;
    if point_in_rect(x, y, stock_x, top_y, card_w, stock_height) {
        return HitTarget::Stock;
    }

    let waste_x = metrics.column_x(1);
    if point_in_rect(x, y, waste_x, top_y, card_w, card_h) && state.game.waste_count() > 0 {
        return HitTarget::Waste;
    }

    for foundation in 0..FOUNDATION_COLUMNS {
        let fx = metrics.column_x(3 + foundation);
        if point_in_rect(x, y, fx, top_y, card_w, card_h) {
            return HitTarget::Foundation(foundation);
        }
    }

    let tableau_top = metrics.tableau_y();
    for column in 0..TABLEAU_COLUMNS {
        let col_x = metrics.column_x(column);
        if x < col_x || x >= col_x + card_w {
            continue;
        }

        let cards = match state.game.tableau_column(column) {
            Some(cards) => cards,
            None => continue,
        };
        let slots = &state.tableau_slots[column];

        if cards.is_empty() {
            let slot = slots.first().copied().unwrap_or(CardSlot {
                top: tableau_top,
                height: card_h,
            });
            if point_in_rect(x, y, col_x, slot.top, card_w, slot.height.max(card_h)) {
                return HitTarget::Tableau {
                    column,
                    card_index: None,
                };
            }
            continue;
        }

        if slots.len() == cards.len() {
            for (idx, slot) in slots.iter().enumerate().rev() {
                let height = if idx + 1 == cards.len() {
                    card_h
                } else {
                    slot.height
                };
                if point_in_rect(x, y, col_x, slot.top, card_w, height.max(1)) {
                    return HitTarget::Tableau {
                        column,
                        card_index: Some(idx),
                    };
                }
            }
            if let Some(last) = slots.last() {
                let bottom = last.top + card_h;
                if y >= last.top && y < bottom {
                    return HitTarget::Tableau {
                        column,
                        card_index: Some(cards.len() - 1),
                    };
                }
            }
            continue;
        }

        let mut y_pos = tableau_top;
        for (idx, card) in cards.iter().enumerate() {
            let height = if idx + 1 == cards.len() {
                card_h
            } else if card.face_up {
                metrics.face_up_offset
            } else {
                metrics.face_down_offset
            };
            if point_in_rect(x, y, col_x, y_pos, card_w, height.max(1)) {
                return HitTarget::Tableau {
                    column,
                    card_index: Some(idx),
                };
            }
            let offset = if card.face_up {
                metrics.face_up_offset
            } else {
                metrics.face_down_offset
            };
            y_pos += offset;
        }
        if y >= tableau_top && y < y_pos + card_h {
            return HitTarget::Tableau {
                column,
                card_index: Some(cards.len() - 1),
            };
        }
    }

    HitTarget::None
}

fn tableau_card_top(
    state: &WindowState,
    metrics: &CardMetrics,
    column: usize,
    index: usize,
) -> i32 {
    if let Some(slot) = state.tableau_slots[column].get(index) {
        slot.top
    } else {
        let mut y = metrics.tableau_y();
        if let Some(cards) = state.game.tableau_column(column) {
            for (i, card) in cards.iter().enumerate() {
                if i == index {
                    return y;
                }
                y += if card.face_up {
                    metrics.face_up_offset
                } else {
                    metrics.face_down_offset
                };
            }
        }
        y
    }
}

fn highlight_rect(dc: HDC, rect: RECT, color: COLORREF) {
    let radius = ((rect.right - rect.left).min(rect.bottom - rect.top) / 8).max(4);
    draw_round_outline(dc, rect, radius, color, 3);
}

fn inset_rect(rect: RECT, inset: i32) -> RECT {
    RECT {
        left: rect.left + inset,
        top: rect.top + inset,
        right: rect.right - inset,
        bottom: rect.bottom - inset,
    }
}

fn draw_round_rect_fill(dc: HDC, rect: RECT, radius: i32, fill: COLORREF, border: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        if brush.0 == 0 {
            return;
        }
        let pen = CreatePen(PS_SOLID, 1, border);
        if pen.0 == 0 {
            let _ = DeleteObject(HGDIOBJ(brush.0));
            return;
        }
        let old_brush = SelectObject(dc, HGDIOBJ(brush.0));
        let old_pen = SelectObject(dc, HGDIOBJ(pen.0));
        let radius = radius.max(0);
        let _ = RoundRect(
            dc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            radius,
            radius,
        );
        if old_brush.0 != 0 {
            let _ = SelectObject(dc, old_brush);
        }
        if old_pen.0 != 0 {
            let _ = SelectObject(dc, old_pen);
        }
        let _ = DeleteObject(HGDIOBJ(brush.0));
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

fn draw_round_outline(dc: HDC, rect: RECT, radius: i32, color: COLORREF, thickness: i32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, thickness.max(1), color);
        if pen.0 == 0 {
            return;
        }
        let hollow = GetStockObject(HOLLOW_BRUSH);
        let old_pen = SelectObject(dc, HGDIOBJ(pen.0));
        let old_brush = SelectObject(dc, hollow);
        let radius = radius.max(0);
        let _ = RoundRect(
            dc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            radius,
            radius,
        );
        if old_pen.0 != 0 {
            let _ = SelectObject(dc, old_pen);
        }
        if old_brush.0 != 0 {
            let _ = SelectObject(dc, old_brush);
        }
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

fn draw_card_back(dc: HDC, rect: RECT) {
    let radius = ((rect.right - rect.left).min(rect.bottom - rect.top) / 6).max(8);
    let border = rgb(240, 240, 240);
    draw_round_rect_fill(dc, rect, radius, rgb(30, 60, 150), border);

    let inner = inset_rect(rect, 4);
    let inner_radius = (radius - 4).max(4);
    draw_round_rect_fill(dc, inner, inner_radius, rgb(12, 32, 104), rgb(12, 32, 104));

    let stripe_width = ((inner.right - inner.left) / 6).max(8);
    let mut left_stripe = inset_rect(inner, 6);
    left_stripe.right = left_stripe.left + stripe_width;
    let stripe_radius = (inner_radius - 4).max(3);
    draw_round_rect_fill(
        dc,
        left_stripe,
        stripe_radius,
        rgb(200, 48, 64),
        rgb(200, 48, 64),
    );

    let mut right_stripe = inset_rect(inner, 6);
    right_stripe.left = right_stripe.right - stripe_width;
    draw_round_rect_fill(
        dc,
        right_stripe,
        stripe_radius,
        rgb(200, 48, 64),
        rgb(200, 48, 64),
    );
}

fn selection_rect(metrics: &CardMetrics, state: &WindowState, column: usize, index: usize) -> RECT {
    let x = metrics.column_x(column);
    let top = tableau_card_top(state, metrics, column, index);
    make_rect(x, top, metrics.card_w, metrics.card_h)
}

fn waste_rect(metrics: &CardMetrics) -> RECT {
    make_rect(
        metrics.column_x(1),
        metrics.top_y(),
        metrics.card_w,
        metrics.card_h,
    )
}

fn foundation_rect(metrics: &CardMetrics, index: usize) -> RECT {
    make_rect(
        metrics.column_x(3 + index),
        metrics.top_y(),
        metrics.card_w,
        metrics.card_h,
    )
}

fn stock_rect(metrics: &CardMetrics) -> RECT {
    make_rect(
        metrics.column_x(0),
        metrics.top_y(),
        metrics.card_w,
        metrics.card_h,
    )
}

fn tableau_focus_rect(
    metrics: &CardMetrics,
    state: &WindowState,
    column: usize,
    card_index: Option<usize>,
) -> RECT {
    if let Some(idx) = card_index {
        selection_rect(metrics, state, column, idx)
    } else if let Some(slot) = state.tableau_slots[column].first() {
        make_rect(
            metrics.column_x(column),
            slot.top,
            metrics.card_w,
            slot.height.max(metrics.card_h),
        )
    } else {
        make_rect(
            metrics.column_x(column),
            metrics.tableau_y(),
            metrics.card_w,
            metrics.card_h,
        )
    }
}

fn draw_focus_outline(dc: HDC, metrics: &CardMetrics, state: &WindowState, focus: HitTarget) {
    let color = rgb(255, 215, 0);
    match focus {
        HitTarget::Stock => highlight_rect(dc, stock_rect(metrics), color),
        HitTarget::Waste => highlight_rect(dc, waste_rect(metrics), color),
        HitTarget::Foundation(index) => highlight_rect(dc, foundation_rect(metrics, index), color),
        HitTarget::Tableau { column, card_index } => {
            let rect = tableau_focus_rect(metrics, state, column, card_index);
            highlight_rect(dc, rect, color);
        }
        HitTarget::None => {}
    }
}

fn draw_selection_outline(
    dc: HDC,
    metrics: &CardMetrics,
    state: &WindowState,
    selection: Selection,
) {
    let color = COLORREF(0x0000_FFFF);
    match selection {
        Selection::Waste => highlight_rect(dc, waste_rect(metrics), color),
        Selection::Tableau { column, index } => {
            highlight_rect(dc, selection_rect(metrics, state, column, index), color)
        }
    }
}

fn draw_drag_outline(dc: HDC, metrics: &CardMetrics, state: &WindowState, drag: &DragContext) {
    let color = COLORREF(0x0000_FF00);
    match drag.hover {
        HitTarget::Foundation(index) => highlight_rect(dc, foundation_rect(metrics, index), color),
        HitTarget::Tableau { column, card_index } => {
            let rect = if let Some(idx) = card_index {
                selection_rect(metrics, state, column, idx)
            } else {
                make_rect(
                    metrics.column_x(column),
                    metrics.tableau_y(),
                    metrics.card_w,
                    metrics.card_h,
                )
            };
            highlight_rect(dc, rect, color);
        }
        HitTarget::Waste => highlight_rect(dc, waste_rect(metrics), color),
        _ => {}
    }
}

fn set_focus(state: &mut WindowState, focus: HitTarget) {
    state.focus = Some(normalize_focus(state, focus));
}

fn ensure_focus_valid(state: &mut WindowState) {
    let current = state.focus.unwrap_or(HitTarget::Stock);
    state.focus = Some(normalize_focus(state, current));
}

fn normalize_focus(state: &WindowState, focus: HitTarget) -> HitTarget {
    match focus {
        HitTarget::Tableau { column, card_index } => {
            if TABLEAU_COLUMNS == 0 {
                return HitTarget::Stock;
            }
            if column >= TABLEAU_COLUMNS {
                return focus_tableau_top(state, TABLEAU_COLUMNS - 1);
            }
            let len = state.game.tableau_len(column);
            if len == 0 {
                HitTarget::Tableau {
                    column,
                    card_index: None,
                }
            } else {
                let mut idx = card_index.unwrap_or(len - 1);
                if idx >= len {
                    idx = len - 1;
                }
                HitTarget::Tableau {
                    column,
                    card_index: Some(idx),
                }
            }
        }
        HitTarget::Foundation(idx) => {
            HitTarget::Foundation(idx.min(FOUNDATION_COLUMNS.saturating_sub(1)))
        }
        HitTarget::None => HitTarget::Stock,
        other => other,
    }
}

fn focus_tableau_top(state: &WindowState, column: usize) -> HitTarget {
    if TABLEAU_COLUMNS == 0 {
        return HitTarget::Stock;
    }
    let column = column.min(TABLEAU_COLUMNS - 1);
    let len = state.game.tableau_len(column);
    if len == 0 {
        HitTarget::Tableau {
            column,
            card_index: None,
        }
    } else {
        HitTarget::Tableau {
            column,
            card_index: Some(len - 1),
        }
    }
}

fn top_target_for_column(column: usize) -> HitTarget {
    match column {
        0 => HitTarget::Stock,
        1 => HitTarget::Waste,
        _ => {
            let idx = column
                .saturating_sub(3)
                .min(FOUNDATION_COLUMNS.saturating_sub(1));
            HitTarget::Foundation(idx)
        }
    }
}

fn column_for_top_target(target: HitTarget) -> usize {
    match target {
        HitTarget::Stock => 0,
        HitTarget::Waste => 1,
        HitTarget::Foundation(idx) => {
            if TABLEAU_COLUMNS == 0 {
                0
            } else {
                (3 + idx).min(TABLEAU_COLUMNS - 1)
            }
        }
        _ => 0,
    }
}

fn move_focus_horizontal(state: &mut WindowState, delta: i32) -> bool {
    ensure_focus_valid(state);
    let focus = state.focus.unwrap_or(HitTarget::Stock);
    match focus {
        HitTarget::Stock | HitTarget::Waste | HitTarget::Foundation(_) => {
            let top_count = 2 + FOUNDATION_COLUMNS;
            let current = match focus {
                HitTarget::Stock => 0,
                HitTarget::Waste => 1,
                HitTarget::Foundation(idx) => (idx + 2).min(top_count.saturating_sub(1)),
                _ => unreachable!(),
            } as i32;
            let max_index = (top_count.saturating_sub(1)) as i32;
            let next = (current + delta).clamp(0, max_index);
            if next != current {
                let new_focus = match next {
                    0 => HitTarget::Stock,
                    1 => HitTarget::Waste,
                    idx => {
                        let foundation = (idx - 2) as usize;
                        HitTarget::Foundation(foundation.min(FOUNDATION_COLUMNS.saturating_sub(1)))
                    }
                };
                set_focus(state, new_focus);
                return true;
            }
        }
        HitTarget::Tableau { column, .. } => {
            let new_column = column as i32 + delta;
            if TABLEAU_COLUMNS > 0 && new_column >= 0 && new_column < TABLEAU_COLUMNS as i32 {
                set_focus(state, focus_tableau_top(state, new_column as usize));
                return true;
            }
        }
        HitTarget::None => {
            set_focus(state, HitTarget::Stock);
            return true;
        }
    }
    false
}

fn move_focus_vertical(state: &mut WindowState, down: bool) -> bool {
    ensure_focus_valid(state);
    let focus = state.focus.unwrap_or(HitTarget::Stock);
    match focus {
        HitTarget::Stock | HitTarget::Waste | HitTarget::Foundation(_) => {
            if down {
                let column = column_for_top_target(focus);
                set_focus(state, focus_tableau_top(state, column));
                return true;
            }
        }
        HitTarget::Tableau { column, card_index } => {
            let len = state.game.tableau_len(column);
            if len == 0 {
                if !down {
                    set_focus(state, top_target_for_column(column));
                    return true;
                }
                return false;
            }
            let mut idx = card_index.unwrap_or(len - 1);
            if down {
                if idx + 1 < len {
                    idx += 1;
                    set_focus(
                        state,
                        HitTarget::Tableau {
                            column,
                            card_index: Some(idx),
                        },
                    );
                    return true;
                }
            } else if idx > 0 {
                idx -= 1;
                set_focus(
                    state,
                    HitTarget::Tableau {
                        column,
                        card_index: Some(idx),
                    },
                );
                return true;
            } else {
                set_focus(state, top_target_for_column(column));
                return true;
            }
        }
        HitTarget::None => {
            set_focus(state, HitTarget::Stock);
            return true;
        }
    }
    false
}

fn handle_key_down(hwnd: HWND, state: &mut WindowState, key: u32) -> bool {
    match key as u16 {
        k if k == VK_LEFT.0 => {
            if move_focus_horizontal(state, -1) {
                request_redraw(hwnd);
                true
            } else {
                false
            }
        }
        k if k == VK_RIGHT.0 => {
            if move_focus_horizontal(state, 1) {
                request_redraw(hwnd);
                true
            } else {
                false
            }
        }
        k if k == VK_UP.0 => {
            if move_focus_vertical(state, false) {
                request_redraw(hwnd);
                true
            } else {
                false
            }
        }
        k if k == VK_DOWN.0 => {
            if move_focus_vertical(state, true) {
                request_redraw(hwnd);
                true
            } else {
                false
            }
        }
        k if k == VK_SPACE.0 => {
            ensure_focus_valid(state);
            let target = state.focus.unwrap_or(HitTarget::Stock);
            handle_click(hwnd, state, target);
            request_redraw(hwnd);
            true
        }
        _ => false,
    }
}

fn begin_drag(hwnd: HWND, state: &mut WindowState, target: HitTarget, cursor: (i32, i32)) -> bool {
    let metrics = state.layout_metrics.unwrap_or_else(|| {
        let (w, h) = state.client_size;
        CardMetrics::compute(state, w.max(1), h.max(1))
    });
    match target {
        HitTarget::Tableau {
            column,
            card_index: Some(index),
        } => {
            if let Some(card) = state.game.tableau_card(column, index) {
                if !card.face_up {
                    return false;
                }
            } else {
                return false;
            }
            let snapshot = state.game.clone();
            let top = tableau_card_top(state, &metrics, column, index);
            if let Some(stack) = state.game.extract_tableau_stack(column, index) {
                state.tableau_slots[column].truncate(index);
                state.drag = Some(DragContext {
                    source: DragSource::Tableau { column },
                    cards: stack,
                    hotspot: (cursor.0 - metrics.column_x(column), cursor.1 - top),
                    position: (metrics.column_x(column), top),
                    hover: HitTarget::None,
                    snapshot,
                });
                state.pending_selection = None;
                state.layout_metrics = Some(metrics);
                unsafe {
                    SetCapture(hwnd);
                }
                true
            } else {
                false
            }
        }
        HitTarget::Waste => {
            if state.game.waste.cards.is_empty() {
                return false;
            }
            let snapshot = state.game.clone();
            let card = state.game.waste.cards.pop().unwrap();
            let top = metrics.top_y();
            state.drag = Some(DragContext {
                source: DragSource::Waste,
                cards: vec![card],
                hotspot: (cursor.0 - metrics.column_x(1), cursor.1 - top),
                position: (metrics.column_x(1), top),
                hover: HitTarget::None,
                snapshot,
            });
            state.pending_selection = None;
            state.layout_metrics = Some(metrics);
            unsafe {
                SetCapture(hwnd);
            }
            true
        }
        _ => false,
    }
}

fn finalize_drag(state: &mut WindowState, drag: DragContext, drop_target: HitTarget) -> bool {
    let DragContext { source, cards, .. } = drag;
    match source {
        DragSource::Tableau { column: from } => match drop_target {
            HitTarget::Tableau { column: to, .. } if from != to => {
                if state.game.can_accept_tableau_stack(to, &cards) {
                    state.game.place_tableau_stack(to, cards);
                    state.game.reveal_tableau_top(from);
                    state.pending_selection = None;
                    state.layout_metrics = None;
                    let focus_target = focus_tableau_top(state, to);
                    set_focus(state, focus_target);
                    true
                } else {
                    state.game.cancel_tableau_stack(from, cards);
                    false
                }
            }
            HitTarget::Foundation(index) if cards.len() == 1 => {
                let card = cards.into_iter().next().unwrap();
                if state.game.place_on_foundation(index, card) {
                    state.game.reveal_tableau_top(from);
                    state.pending_selection = None;
                    state.layout_metrics = None;
                    set_focus(state, HitTarget::Foundation(index));
                    true
                } else {
                    state.game.cancel_tableau_stack(from, vec![card]);
                    false
                }
            }
            _ => {
                state.game.cancel_tableau_stack(from, cards);
                let len = state.game.tableau_len(from);
                if len == 0 {
                    state.pending_selection = None;
                    set_focus(
                        state,
                        HitTarget::Tableau {
                            column: from,
                            card_index: None,
                        },
                    );
                } else {
                    let top = len - 1;
                    state.pending_selection = Some(Selection::Tableau {
                        column: from,
                        index: top,
                    });
                    set_focus(
                        state,
                        HitTarget::Tableau {
                            column: from,
                            card_index: Some(top),
                        },
                    );
                }
                state.layout_metrics = None;
                false
            }
        },
        DragSource::Waste => match drop_target {
            HitTarget::Tableau { column: to, .. } => {
                if state.game.can_accept_tableau_stack(to, &cards) {
                    state.game.place_tableau_stack(to, cards);
                    state.pending_selection = None;
                    state.layout_metrics = None;
                    let focus_target = focus_tableau_top(state, to);
                    set_focus(state, focus_target);
                    true
                } else {
                    state.game.waste.cards.extend(cards);
                    false
                }
            }
            HitTarget::Foundation(index) if cards.len() == 1 => {
                let card = cards.into_iter().next().unwrap();
                if state.game.place_on_foundation(index, card) {
                    state.pending_selection = None;
                    state.layout_metrics = None;
                    set_focus(state, HitTarget::Foundation(index));
                    true
                } else {
                    state.game.waste.cards.push(card);
                    false
                }
            }
            _ => {
                state.game.waste.cards.extend(cards);
                state.pending_selection = Some(Selection::Waste);
                set_focus(state, HitTarget::Waste);
                state.layout_metrics = None;
                false
            }
        },
    }
}

fn handle_click(hwnd: HWND, state: &mut WindowState, target: HitTarget) {
    set_focus(state, target);
    match target {
        HitTarget::Stock => {
            state.pending_selection = None;
            let snapshot = state.game.clone();
            match state.game.stock_click() {
                StockAction::Drawn(_) | StockAction::Recycled(_) => {
                    state.push_undo(snapshot);
                    update_status_bar(state);
                    request_redraw(hwnd);
                }
                StockAction::NoOp => {}
            }
        }
        HitTarget::Waste => {
            if state.game.waste_count() > 0 {
                if matches!(state.pending_selection, Some(Selection::Waste)) {
                    state.pending_selection = None;
                } else {
                    state.pending_selection = Some(Selection::Waste);
                }
                request_redraw(hwnd);
            }
        }
        HitTarget::Foundation(index) => {
            let snapshot = state.game.clone();
            let moved = if let Some(selection) = state.pending_selection {
                match selection {
                    Selection::Waste => state.game.move_waste_to_foundation(index),
                    Selection::Tableau {
                        column,
                        index: start,
                    } => {
                        if start + 1 == state.game.tableau_len(column) {
                            state.game.move_tableau_to_foundation(column, index)
                        } else {
                            false
                        }
                    }
                }
            } else {
                state.game.move_waste_to_foundation(index)
            };
            if moved {
                state.pending_selection = None;
                state.push_undo(snapshot);
                update_status_bar(state);
                check_for_victory(hwnd, state);
                request_redraw(hwnd);
            }
        }
        HitTarget::Tableau { column, card_index } => {
            let mut snapshot: Option<GameState> = None;
            let mut moved = false;
            if let Some(selection) = state.pending_selection {
                match selection {
                    Selection::Waste => {
                        snapshot.get_or_insert_with(|| state.game.clone());
                        moved = state.game.move_waste_to_tableau(column);
                    }
                    Selection::Tableau {
                        column: from,
                        index: start,
                    } => {
                        if from != column {
                            snapshot.get_or_insert_with(|| state.game.clone());
                            if let Some(stack) = state.game.extract_tableau_stack(from, start) {
                                if state.game.can_accept_tableau_stack(column, &stack) {
                                    state.game.place_tableau_stack(column, stack);
                                    state.game.reveal_tableau_top(from);
                                    moved = true;
                                } else {
                                    state.game.cancel_tableau_stack(from, stack);
                                }
                            }
                        }
                    }
                }
            }
            if moved {
                state.pending_selection = None;
                if let Some(snap) = snapshot {
                    state.push_undo(snap);
                }
                update_status_bar(state);
                check_for_victory(hwnd, state);
                request_redraw(hwnd);
            } else if let Some(idx) = card_index {
                let len = state.game.tableau_len(column);
                if len == 0 {
                    state.pending_selection = None;
                } else {
                    let top_index = len - 1;
                    if idx == top_index
                        && state
                            .game
                            .tableau_card(column, idx)
                            .map(|card| !card.face_up)
                            .unwrap_or(false)
                    {
                        let snapshot = state.game.clone();
                        if state.game.flip_tableau_top(column) {
                            state.pending_selection = None;
                            state.push_undo(snapshot);
                            update_status_bar(state);
                            request_redraw(hwnd);
                        }
                    } else if matches!(
                        state.pending_selection,
                        Some(Selection::Tableau { column: c, index: i }) if c == column && i == idx
                    ) {
                        state.pending_selection = None;
                        request_redraw(hwnd);
                    } else if state
                        .game
                        .tableau_card(column, idx)
                        .map(|card| card.face_up)
                        .unwrap_or(false)
                    {
                        state.pending_selection = Some(Selection::Tableau { column, index: idx });
                        request_redraw(hwnd);
                    } else {
                        state.pending_selection = None;
                    }
                }
            } else {
                state.pending_selection = None;
                request_redraw(hwnd);
            }
        }
        HitTarget::None => {
            state.pending_selection = None;
            request_redraw(hwnd);
        }
    }
    ensure_focus_valid(state);
}

unsafe fn load_card_bitmap_from_resource(res_id: u16) -> anyhow::Result<Option<CardImage>> {
    let hinst = HINSTANCE(GetModuleHandleW(None)?.0);
    let hresinfo = FindResourceW(hinst, make_int_resource(res_id), make_int_resource(10));
    if hresinfo.0 == 0 {
        return Ok(None);
    }
    let size = SizeofResource(hinst, hresinfo);
    let hres = LoadResource(hinst, hresinfo)?;
    let locked = LockResource(hres) as *const u8;
    if locked.is_null() {
        return Ok(None);
    }
    let bytes = std::slice::from_raw_parts(locked, size as usize);

    let factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

    let stream: IWICStream = factory.CreateStream()?;
    let owned = bytes.to_vec();
    stream.InitializeFromMemory(&owned)?;

    let decoder =
        factory.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeOptions(0))?;
    let frame = decoder.GetFrame(0)?;

    let converter: IWICFormatConverter = factory.CreateFormatConverter()?;
    converter.Initialize(
        &frame,
        &GUID_WICPixelFormat32bppPBGRA,
        WICBitmapDitherTypeNone,
        None,
        0.0,
        WICBitmapPaletteTypeCustom,
    )?;

    let mut w = 0u32;
    let mut h = 0u32;
    converter.GetSize(&mut w, &mut h)?;
    let w = w as i32;
    let h = h as i32;

    let bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
    let hbm = CreateDIBSection(HDC(0), &bi, DIB_RGB_COLORS, &mut bits, None, 0)?;
    if hbm.is_invalid() {
        return Err(anyhow::anyhow!("CreateDIBSection for card failed"));
    }

    let stride = (w * 4) as u32;
    let buf_size = (h * w * 4) as usize;
    let slice = std::slice::from_raw_parts_mut(bits as *mut u8, buf_size);
    converter.CopyPixels(std::ptr::null(), stride, slice)?;

    let cell_w = (w / CARD_SPRITE_COLS).max(1);
    let cell_h = (h / CARD_SPRITE_ROWS).max(1);

    Ok(Some(CardImage {
        hbm,
        cell_w,
        cell_h,
    }))
}

unsafe fn paint_window(hwnd: HWND, hdc: HDC, state: &mut WindowState) {
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    ensure_backbuffer(hwnd, state, rc.right - rc.left, rc.bottom - rc.top);

    if state.back.is_some() {
        let client_width = rc.right - rc.left;
        let client_height = rc.bottom - rc.top;
        let metrics = CardMetrics::compute(state, client_width, client_height);
        state.layout_metrics = Some(metrics);
        ensure_focus_valid(state);

        if let Some(back) = state.back.as_ref() {
            FillRect(back.dc, &rc, state.bg_brush);

            let draw_placeholder = |dc: HDC, x: i32, y: i32| {
                let rect = make_rect(x, y, metrics.card_w, metrics.card_h);
                let radius = (metrics.card_w.min(metrics.card_h) / 6).max(6);
                draw_round_rect_fill(dc, rect, radius, rgb(8, 96, 24), rgb(0, 0, 0));
                let inner = inset_rect(rect, 3);
                draw_round_outline(dc, inner, (radius - 2).max(4), rgb(0, 0, 0), 1);
            };

            let draw_face_up = |card: &Card, x: i32, y: i32| {
                let rect = make_rect(x, y, metrics.card_w, metrics.card_h);
                if let (Some(image), true) = (state.card.as_ref(), state.card_dc.0 != 0) {
                    let radius = (metrics.card_w.min(metrics.card_h) / 6).max(6);
                    draw_round_rect_fill(
                        back.dc,
                        rect,
                        radius,
                        rgb(252, 252, 252),
                        rgb(204, 204, 204),
                    );
                    let sprite = card.sprite_index as i32;
                    let src_x = (sprite % CARD_SPRITE_COLS) * image.cell_w;
                    let src_y = (sprite / CARD_SPRITE_COLS) * image.cell_h;
                    let trim_x = 1;
                    let trim_y = 1;
                    let src_w = (image.cell_w - trim_x * 2).max(1);
                    let src_h = (image.cell_h - trim_y * 2).max(1);
                    let blend = BLENDFUNCTION {
                        BlendOp: AC_SRC_OVER as u8,
                        BlendFlags: 0,
                        SourceConstantAlpha: 255,
                        AlphaFormat: AC_SRC_ALPHA as u8,
                    };
                    let max_inset_w = ((rect.right - rect.left) / 2).saturating_sub(1);
                    let max_inset_h = ((rect.bottom - rect.top) / 2).saturating_sub(1);
                    let face_gap = (metrics.card_w.min(metrics.card_h) / 32).max(2);
                    let inset = metrics
                        .face_inset
                        .saturating_add(face_gap)
                        .min(max_inset_w)
                        .min(max_inset_h)
                        .max(0);
                    let inner = if inset > 0 {
                        inset_rect(rect, inset)
                    } else {
                        rect
                    };
                    let dest_w = (inner.right - inner.left).max(0);
                    let dest_h = (inner.bottom - inner.top).max(0);
                    if dest_w > 0 && dest_h > 0 {
                        unsafe {
                            AlphaBlend(
                                back.dc,
                                inner.left,
                                inner.top,
                                dest_w,
                                dest_h,
                                state.card_dc,
                                src_x + trim_x,
                                src_y + trim_y,
                                src_w,
                                src_h,
                                blend,
                            );
                        }
                    }
                } else {
                    draw_placeholder(back.dc, x, y);
                }
            };

            let draw_face_down = |x: i32, y: i32| {
                let rect = make_rect(x, y, metrics.card_w, metrics.card_h);
                draw_card_back(back.dc, rect);
            };

            let draw_empty = |x: i32, y: i32| {
                draw_placeholder(back.dc, x, y);
            };

            let top_y = metrics.top_y();
            let stock_x = metrics.column_x(0);
            if !state.game.stock.cards.is_empty() {
                draw_face_down(stock_x, top_y);
            } else {
                draw_empty(stock_x, top_y);
            }

            let waste_x = metrics.column_x(1);
            if let Some(card) = state.game.waste.cards.last() {
                draw_face_up(card, waste_x, top_y);
            } else {
                draw_empty(waste_x, top_y);
            }

            let foundation_start = 3usize;
            for (index, pile) in state.game.foundations.iter().enumerate() {
                let x = metrics.column_x(foundation_start + index);
                let emitted = state
                    .win_anim
                    .as_ref()
                    .map(|anim| anim.emitted_from(index))
                    .unwrap_or(0);
                let visible = pile.cards.len().saturating_sub(emitted);
                if visible > 0 {
                    let mut card = pile.cards[visible - 1];
                    card.face_up = true;
                    draw_face_up(&card, x, top_y);
                } else {
                    draw_empty(x, top_y);
                }
            }

            let tableau_top = metrics.tableau_y();
            for slots in &mut state.tableau_slots {
                slots.clear();
            }
            for (column, pile) in state.game.tableaus.iter().enumerate() {
                let x = metrics.column_x(column);
                let slots = &mut state.tableau_slots[column];
                if pile.cards.is_empty() {
                    slots.push(CardSlot {
                        top: tableau_top,
                        height: metrics.card_h,
                    });
                    draw_empty(x, tableau_top);
                    continue;
                }

                let mut y = tableau_top;
                for (idx, card) in pile.cards.iter().enumerate() {
                    let is_last = idx + 1 == pile.cards.len();
                    let height = if is_last {
                        metrics.card_h
                    } else if card.face_up {
                        metrics.face_up_offset
                    } else {
                        metrics.face_down_offset
                    };
                    slots.push(CardSlot {
                        top: y,
                        height: height.max(1),
                    });
                    if card.face_up {
                        draw_face_up(card, x, y);
                        y += metrics.face_up_offset;
                    } else {
                        draw_face_down(x, y);
                        y += metrics.face_down_offset;
                    }
                }
            }

            if let Some(focus) = state.focus {
                draw_focus_outline(back.dc, &metrics, state, focus);
            }
            if let Some(selection) = state.pending_selection {
                draw_selection_outline(back.dc, &metrics, state, selection);
            }
            if let Some(drag) = &state.drag {
                draw_drag_outline(back.dc, &metrics, state, drag);
            }

            if let Some(anim) = &state.win_anim {
                for card in &anim.cards {
                    if !card.emitted || card.finished {
                        continue;
                    }
                    let x = card.pos.0.round() as i32;
                    let y = card.pos.1.round() as i32;
                    draw_face_up(&card.card, x, y);
                }
            }

            if let Some(drag) = &state.drag {
                let mut y = drag.position.1;
                let x = drag.position.0;
                for card in &drag.cards {
                    if card.face_up {
                        draw_face_up(card, x, y);
                        y += metrics.face_up_offset;
                    } else {
                        draw_face_down(x, y);
                        y += metrics.face_down_offset;
                    }
                }
            }

            unsafe {
                let _ = BitBlt(hdc, 0, 0, back.w, back.h, back.dc, 0, 0, SRCCOPY);
            }
        }
    } else {
        FillRect(hdc, &rc, state.bg_brush);
    }
}

fn show_about_dialog(hwnd: HWND) {
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let _ = DialogBoxParamW(
            hinst,
            make_int_resource(constants::IDD_ABOUT),
            hwnd,
            Some(about_dialog_proc),
            LPARAM(0),
        );
    }
}

struct AboutDialogState {
    bg_brush: HBRUSH,
    card_brush: HBRUSH,
    border_pen: HPEN,
}

unsafe fn get_about_state<'a>(hwnd: HWND) -> Option<&'a mut AboutDialogState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AboutDialogState;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

unsafe fn free_about_state(hwnd: HWND) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AboutDialogState;
    if ptr.is_null() {
        return;
    }
    let state = Box::from_raw(ptr);
    if state.bg_brush.0 != 0 {
        let _ = DeleteObject(state.bg_brush);
    }
    if state.card_brush.0 != 0 {
        let _ = DeleteObject(state.card_brush);
    }
    if state.border_pen.0 != 0 {
        let _ = DeleteObject(state.border_pen);
    }
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
}

unsafe extern "system" fn about_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            let bg_brush = CreateSolidBrush(rgb(12, 90, 24));
            let card_brush = CreateSolidBrush(rgb(244, 240, 230));
            let border_pen = CreatePen(PS_SOLID, 2, rgb(24, 48, 24));
            if bg_brush.0 == 0 || card_brush.0 == 0 || border_pen.0 == 0 {
                if bg_brush.0 != 0 {
                    let _ = DeleteObject(bg_brush);
                }
                if card_brush.0 != 0 {
                    let _ = DeleteObject(card_brush);
                }
                if border_pen.0 != 0 {
                    let _ = DeleteObject(border_pen);
                }
                return 0;
            }
            let state = Box::new(AboutDialogState {
                bg_brush,
                card_brush,
                border_pen,
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            1
        }
        WM_CTLCOLORDLG => {
            if let Some(state) = get_about_state(hwnd) {
                return state.bg_brush.0;
            }
            0
        }
        WM_CTLCOLORBTN | WM_CTLCOLORSTATIC => {
            let hdc = HDC(wparam.0 as isize);
            let _ = SetBkMode(hdc, TRANSPARENT);
            0
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if let Some(state) = get_about_state(hwnd) {
                let mut client = RECT::default();
                let _ = GetClientRect(hwnd, &mut client);
                FillRect(hdc, &client, state.bg_brush);
                let old_pen = SelectObject(hdc, state.border_pen);
                let old_brush = SelectObject(hdc, state.card_brush);
                let card_width = 96;
                let card_height = 128;
                let rounding = 12;
                let center_x = (client.right - client.left) / 2;
                let base_x = center_x - card_width / 2;
                let base_y = 36;
                let cards = [
                    (-90, 42, "J\u{2663}", rgb(32, 40, 48)),
                    (-45, 24, "Q\u{2665}", rgb(198, 54, 54)),
                    (0, 16, "K\u{2660}", rgb(32, 40, 48)),
                    (45, 30, "A\u{2666}", rgb(198, 54, 54)),
                ];
                for (offset_x, offset_y, label, color) in cards {
                    let x = base_x + offset_x;
                    let y = base_y + offset_y;
                    RoundRect(
                        hdc,
                        x,
                        y,
                        x + card_width,
                        y + card_height,
                        rounding,
                        rounding,
                    );
                    let _ = SetTextColor(hdc, color);
                    let _ = SetBkMode(hdc, TRANSPARENT);
                    let mut text = to_wide(label);
                    let mut rect = RECT {
                        left: x,
                        top: y + 32,
                        right: x + card_width,
                        bottom: y + card_height - 20,
                    };
                    let _ = DrawTextW(
                        hdc,
                        text.as_mut_slice(),
                        &mut rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    );
                }
                let _ = SelectObject(hdc, old_pen);
                let _ = SelectObject(hdc, old_brush);
                let _ = SetTextColor(hdc, rgb(236, 242, 230));
                let _ = SetBkMode(hdc, TRANSPARENT);
                let mut title = to_wide(&format!("{} V1.0.0", constants::PRODUCT_NAME));
                let mut title_rect = RECT {
                    left: client.left + 20,
                    top: base_y + card_height + 16,
                    right: client.right - 20,
                    bottom: client.bottom - 56,
                };
                let _ = DrawTextW(
                    hdc,
                    title.as_mut_slice(),
                    &mut title_rect,
                    DT_CENTER | DT_SINGLELINE | DT_TOP,
                );
                let _ = SetTextColor(hdc, rgb(200, 212, 198));
                let brand = constants::COMPANY_NAME
                    .split_whitespace()
                    .next()
                    .unwrap_or(constants::COMPANY_NAME);
                let mut copyright = to_wide(&format!("(c) {} 2025", brand));
                let mut copy_rect = RECT {
                    left: client.left + 20,
                    top: client.bottom - 48,
                    right: client.right - 20,
                    bottom: client.bottom - 20,
                };
                let _ = DrawTextW(
                    hdc,
                    copyright.as_mut_slice(),
                    &mut copy_rect,
                    DT_CENTER | DT_SINGLELINE | DT_TOP,
                );
            } else {
                let mut client = RECT::default();
                let _ = GetClientRect(hwnd, &mut client);
                let fallback = CreateSolidBrush(rgb(12, 90, 24));
                if fallback.0 != 0 {
                    FillRect(hdc, &client, fallback);
                    let _ = DeleteObject(fallback);
                }
            }
            EndPaint(hwnd, &ps);
            1
        }
        WM_COMMAND => {
            let id = loword(wparam);
            if id == IDOK.0 as u16 || id == IDCANCEL.0 as u16 {
                free_about_state(hwnd);
                let _ = EndDialog(hwnd, 0);
            }
            1
        }
        WM_DESTROY => {
            free_about_state(hwnd);
            0
        }
        _ => 0,
    }
}
