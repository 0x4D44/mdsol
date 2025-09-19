#![windows_subsystem = "windows"]

use std::mem::size_of;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, AlphaBlend, BeginPaint, BitBlt, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DIB_RGB_COLORS, EndPaint,
    FillRect, HBITMAP, HBRUSH, HDC, HGDIOBJ, HPEN, PAINTSTRUCT, PS_SOLID, RoundRect, SelectObject,
    SRCCOPY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
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
use windows::Win32::UI::Controls::{
    CreateStatusWindowW, InitCommonControlsEx, ICC_BAR_CLASSES, INITCOMMONCONTROLSEX,
    SBARS_SIZEGRIP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, LoadAcceleratorsW, LoadCursorW, LoadIconW, LoadMenuW, MessageBoxW,
    PostQuitMessage, RegisterClassExW, SendMessageW, SetWindowLongPtrW, TranslateAcceleratorW,
    TranslateMessage, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    GWLP_USERDATA, HACCEL, HCURSOR, HICON, HMENU, IDC_ARROW, IDI_APPLICATION, MB_ICONINFORMATION,
    MB_OK, MSG, WINDOW_EX_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_ERASEBKGND,
    WM_PAINT, WM_SIZE, WNDCLASSEXW, WNDCLASS_STYLES, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

// Resource IDs (keep in sync with res/resource.h)
const IDR_MAINMENU: u16 = 101;
const IDR_ACCEL: u16 = 201;
const IDB_CARDS: u16 = 301;

const IDM_FILE_NEW: u16 = 40001;
const IDM_FILE_DEALAGAIN: u16 = 40002;
const IDM_FILE_OPTIONS: u16 = 40003;
const IDM_FILE_EXIT: u16 = 40004;
const IDM_EDIT_UNDO: u16 = 40010;
const IDM_EDIT_REDO: u16 = 40011;
const IDM_GAME_DRAW1: u16 = 40020;
const IDM_GAME_DRAW3: u16 = 40021;
const IDM_GAME_AUTOCOMPLETE: u16 = 40024;
const IDM_HELP_ABOUT: u16 = 40100;

const APP_TITLE: PCWSTR = w!("Klondike Solitaire");
const CLASS_NAME: PCWSTR = w!("KlondikeWindowClass");

#[inline]
const fn make_int_resource(id: u16) -> PCWSTR {
    // Equivalent to MAKEINTRESOURCEW; used to avoid import issues.
    PCWSTR(id as usize as *const u16)
}

#[derive(Default)]
struct WindowState {
    status: HWND,
    bg_brush: HBRUSH,
    back: Option<BackBuffer>,
    card: Option<CardImage>,
    card_dc: HDC,
    card_old: HGDIOBJ,
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
                });

                // Create background brush (green felt)
                fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
                    COLORREF(((b as u32) << 16) | ((g as u32) << 8) | (r as u32))
                }
                state.bg_brush = CreateSolidBrush(rgb(0, 128, 0));

                // Init common controls and create status bar
                let icc = INITCOMMONCONTROLSEX {
                    dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
                    dwICC: ICC_BAR_CLASSES,
                };
                InitCommonControlsEx(&icc);
                let style = (WS_CHILD.0 | WS_VISIBLE.0 | SBARS_SIZEGRIP as u32) as i32;
                state.status = CreateStatusWindowW(style, w!(""), hwnd, 1001);

                // Try to load embedded card PNG (optional)
                match load_card_bitmap_from_resource(IDB_CARDS) {
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
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as u16;
                match id {
                    IDM_FILE_EXIT => {
                        let _ = DestroyWindow(hwnd);
                    }
                    IDM_FILE_NEW | IDM_FILE_DEALAGAIN => {
                        MessageBoxW(
                            hwnd,
                            w!("New game not yet implemented."),
                            APP_TITLE,
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                    IDM_EDIT_UNDO => {
                        MessageBoxW(
                            hwnd,
                            w!("Undo not yet implemented."),
                            APP_TITLE,
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                    IDM_EDIT_REDO => {
                        MessageBoxW(
                            hwnd,
                            w!("Redo not yet implemented."),
                            APP_TITLE,
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                    IDM_HELP_ABOUT => {
                        MessageBoxW(
                            hwnd,
                            w!("Klondike Solitaire\nNative Win32 app in Rust\n\n© 2025"),
                            APP_TITLE,
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                    IDM_GAME_DRAW1 | IDM_GAME_DRAW3 | IDM_FILE_OPTIONS | IDM_GAME_AUTOCOMPLETE => {
                        MessageBoxW(
                            hwnd,
                            w!("Option not yet implemented."),
                            APP_TITLE,
                            MB_OK | MB_ICONINFORMATION,
                        );
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
                if let Some(state) = get_state(hwnd) {
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
        // Initialize COM for future WIC usage
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;

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
            LoadMenuW(hinstance, make_int_resource(IDR_MAINMENU)).unwrap_or_default();

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

        // Load accelerators
        let haccel: HACCEL =
            LoadAcceleratorsW(hinstance, make_int_resource(IDR_ACCEL)).unwrap_or_default();

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

            if !haccel.is_invalid() && TranslateAcceleratorW(hwnd, haccel, &mut msg) != 0 {
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

        let mut bi = BITMAPINFO::default();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
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
    w: i32,
    h: i32,
}

unsafe fn load_card_bitmap_from_resource(res_id: u16) -> anyhow::Result<Option<CardImage>> {
    let hinst = HINSTANCE(GetModuleHandleW(None)?.0);
    // Try to locate RCDATA resource
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
    let len = size as usize;
    let bytes = std::slice::from_raw_parts(locked, len);

    // WIC: decode PNG from memory and convert to 32bpp PBGRA
    let factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

    let stream: IWICStream = factory.CreateStream()?;
    let mut_owned = bytes.to_vec();
    stream.InitializeFromMemory(&mut_owned)?;

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

    // Create DIB and copy pixels
    let mut bi = BITMAPINFO::default();
    bi.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
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

    Ok(Some(CardImage { hbm, w, h }))
}

unsafe fn paint_window(hwnd: HWND, hdc: HDC, state: &mut WindowState) {
    // Ensure backbuffer
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    ensure_backbuffer(hwnd, state, rc.right - rc.left, rc.bottom - rc.top);

    if let Some(back) = &state.back {
        // Background
        FillRect(back.dc, &rc, state.bg_brush);

        // Draw a test card in the middle
        let (dw, dh) = (120, 160); // nominal card size
        let cx = (rc.right - rc.left - dw) / 2;
        let cy = (rc.bottom - rc.top - dh) / 2;

        if let Some(card) = &state.card {
            // Assume a 13x4 grid (A..K across, Spades/Hearts/Diamonds/Clubs rows)
            let cols = 13;
            let rows = 4;
            let cell_w = (card.w / cols) as i32;
            let cell_h = (card.h / rows) as i32;
            let src_x = 0; // Ace of Spades at col 0
            let src_y = 0; // row 0 (Spades)
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            AlphaBlend(
                back.dc,
                cx,
                cy,
                dw,
                dh,
                state.card_dc,
                src_x,
                src_y,
                cell_w,
                cell_h,
                blend,
            );
        } else {
            // Placeholder: white rounded rect with dark border to mimic a card
            let white = CreateSolidBrush(COLORREF(0x00FFFFFF));
            let old_br = SelectObject(back.dc, HGDIOBJ(white.0));
            let pen: HPEN = CreatePen(PS_SOLID, 2, COLORREF(0x00222222));
            let old_pen = SelectObject(back.dc, HGDIOBJ(pen.0));
            RoundRect(back.dc, cx, cy, cx + dw, cy + dh, 12, 12);
            let _ = SelectObject(back.dc, old_br);
            let _ = SelectObject(back.dc, old_pen);
            DeleteObject(HGDIOBJ(white.0));
            DeleteObject(HGDIOBJ(pen.0));
        }

        // Blit to screen
        let _ = BitBlt(hdc, 0, 0, back.w, back.h, back.dc, 0, 0, SRCCOPY);
    } else {
        // Fallback: paint green directly
        FillRect(hdc, &rc, state.bg_brush);
    }
}
