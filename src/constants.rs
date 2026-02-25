// Shared constants for resources, commands, and metadata.
// Keep these in sync with res/app.rc and the designs document.

pub const COMPANY_NAME: &str = "0x4D44 Software";
pub const PRODUCT_NAME: &str = "Solitaire";

// Resource identifiers
pub const IDR_MAINMENU: u16 = 101;
pub const IDR_ACCEL: u16 = 201;
pub const IDB_CARDS: u16 = 301;
pub const IDD_ABOUT: u16 = 401;
pub const IDD_OPTIONS: u16 = 402;
pub const IDD_HELP: u16 = 403;
#[allow(dead_code)]
pub const IDI_APPICON: u16 = 501;

// Command identifiers (must match MENU/ACCEL definitions)
pub const IDM_FILE_NEW: u16 = 40001;
pub const IDM_FILE_DEALAGAIN: u16 = 40002;
pub const IDM_FILE_OPTIONS: u16 = 40003;
pub const IDM_FILE_EXIT: u16 = 40004;
pub const IDM_EDIT_UNDO: u16 = 40010;
pub const IDM_EDIT_REDO: u16 = 40011;
pub const IDM_GAME_DRAW1: u16 = 40020;
pub const IDM_GAME_DRAW3: u16 = 40021;
pub const IDM_GAME_HINT: u16 = 40022;
pub const IDM_GAME_AUTOCOMPLETE: u16 = 40024;
pub const IDM_GAME_VICTORY: u16 = 40025;
pub const IDM_GAME_CANCEL_VICTORY: u16 = 40026;
#[allow(dead_code)]
pub const IDM_GAME_VICTORY_CLASSIC: u16 = 40027;
#[allow(dead_code)]
pub const IDM_GAME_VICTORY_MODERN: u16 = 40028;
pub const IDM_GAME_VEGAS_RESET: u16 = 40029;
pub const IDM_GAME_TIMER_PAUSE: u16 = 40030;
pub const IDM_HELP_ABOUT: u16 = 40100;
pub const IDM_HELP_SHORTCUTS: u16 = 40101;

// Registry paths
#[allow(dead_code)]
pub const REGISTRY_BASE_KEY: &str = r"Software\0x4D44 Software\Solitaire";

// Status bar identifiers
pub const STATUS_BAR_ID: u32 = 1001;

// Options dialog control IDs
pub const IDC_OPT_DRAW1: u16 = 5001;
pub const IDC_OPT_DRAW3: u16 = 5002;
pub const IDC_OPT_VICTORY_CLASSIC: u16 = 5003;
pub const IDC_OPT_VICTORY_MODERN: u16 = 5004;
pub const IDC_OPT_SCORING_STANDARD: u16 = 5005;
pub const IDC_OPT_SCORING_VEGAS: u16 = 5006;
pub const IDC_OPT_TIMED: u16 = 5007;
pub const IDC_OPT_VEGAS_CUMULATIVE: u16 = 5008;
pub const IDC_OPT_HINT_HIGHLIGHT: u16 = 5009;
